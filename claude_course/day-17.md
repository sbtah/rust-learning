# Day 17 — Bitcask: an Append-Only KV Store

**Domain:** databases • **Time:** 2 hours • **Difficulty:** hard

## What you'll build

A real key-value store. Put a key, get a key, delete a key — all backed by a single append-only log file with an in-memory index. You'll reuse the record format from Day 15 and the serde layer from Day 16. The design is taken from **Bitcask**, the storage engine behind Riak — known for being dead simple and surprisingly fast. You'll also wrap it in a CLI using **`clap`**.

## What you'll learn

- **Append-only logs** — why databases prefer them over in-place mutation
- **In-memory indexes** over on-disk logs
- **Tombstones** — how deletes work in append-only systems
- **`HashMap<String, u64>`** as a persistent-data index
- **`io::Seek`** to jump to an offset for reads
- **`clap`** and derive-based argument parsing
- When (and when not) you need to compact a log

## Background

### Why append-only?

A naive KV store mutates records in place. Update "apple" = "red" to "apple" = "green", and the disk bytes where "red" lived get overwritten with "green" (or trailing bytes if the new value is shorter).

This is fine in happy-path land. But consider:

- The computer crashes mid-write. Now "apple"'s value is half "red", half "green". Unreadable.
- Two threads both write "apple" concurrently. Depending on timing, the result is garbage.
- Rolling back a transaction means having saved the old bytes somewhere — a separate journal.

**Append-only** sidesteps all this. You never overwrite bytes. Every `put` appends a new record at the end of the file. To update a key, write a new record — the old one is still there, but you remember only the new offset.

Appends are atomic at the record level as long as you fsync between records (Day 20). They're also sequential I/O, which disks love. Bitcask, LSM trees, commit logs in Kafka, write-ahead logs in every relational database — all append-only.

The tradeoff: the file grows without bound. You eventually need to **compact** — write a fresh file containing only live records, then swap. We'll skip compaction today and handle it as an exercise.

### The Bitcask design

- **On disk**: a single log file. Each record holds `{ key, value, tombstone_flag }` serialized by serde.
- **In memory**: a `HashMap<String, u64>` mapping each live key to the file offset of its latest record.
- **Get**: look up the key in the HashMap, `seek` to that offset, read the record, return the value.
- **Put**: append a new record, update the HashMap with the new offset.
- **Delete**: append a tombstone record, remove the key from the HashMap.
- **Startup**: rebuild the HashMap by scanning the entire log from start to end.

That's it. No B-trees, no pages, no buffer pools. It fits in 300 lines. The catch is that your live key count must fit in memory — Bitcask keeps all keys in RAM, not all values. For millions of keys, a few hundred MB. Manageable.

### `io::Seek`

Random reads need to jump around the file:

```rust
use std::io::{Seek, SeekFrom};

let mut file = File::open("data.bin")?;
file.seek(SeekFrom::Start(1024))?;    // absolute byte offset
file.seek(SeekFrom::Current(-16))?;   // relative
file.seek(SeekFrom::End(0))?;         // end
```

`File` implements `Seek`. `BufReader<File>` also implements `Seek`, but seeking invalidates its internal buffer — the next read refills. That's fine for our workload (random reads are rare relative to appends).

### `clap` for CLI parsing

For the CLI, we'll use `clap`'s derive API. You define a struct; `clap` generates the parser from the struct's fields and doc comments. No manual `argv` handling.

```rust
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(about = "a tiny kv store")]
struct Cli {
    #[arg(short, long, default_value = "store.rkvs")]
    file: PathBuf,

    #[command(subcommand)]
    cmd: Command,
}

#[derive(Subcommand)]
enum Command {
    Put { key: String, value: String },
    Get { key: String },
    Delete { key: String },
    List,
}
```

Then `Cli::parse()` gives you a fully-parsed, validated CLI. `--help` is auto-generated.

## Setting up

Continue in `rkvs`:

```sh
cargo add clap --features derive
```

Your dependencies:

```toml
[dependencies]
crc32fast = "1"
serde = { version = "1", features = ["derive"] }
bincode = "1"
clap = { version = "4", features = ["derive"] }
```

## Step 1 — The record schema

Create `src/store.rs`:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct LogEntry {
    pub key: String,
    pub value: LogValue,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub enum LogValue {
    Data(Vec<u8>),
    Tombstone,
}
```

`LogEntry` is what gets serialized and written into the record payload. `LogValue::Data` holds the actual value bytes; `LogValue::Tombstone` marks a deleted key.

Using a `Vec<u8>` for value (instead of `String`) lets users store arbitrary bytes — binary blobs, not just text. Keys stay as `String` for CLI-friendliness.

## Step 2 — The store struct and `open`

Add to `src/store.rs`:

```rust
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{self, BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use crate::format::{read_header, read_record, write_header, write_record, HEADER_LEN};

pub struct KvStore {
    path: PathBuf,
    writer: BufWriter<File>,
    reader: BufReader<File>,
    /// Live keys → absolute file offset of their latest record's *payload length prefix*.
    index: HashMap<String, u64>,
    /// Position in the file where the next write will begin. Matches `writer.stream_position()`.
    write_pos: u64,
}
```

Note we keep the offset of the **record header** (where the length prefix sits), not the payload. This lets `read_record` read the whole framed record when we seek there.

Now `open`:

```rust
impl KvStore {
    pub fn open(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref().to_path_buf();
        let is_new = !path.exists();

        let writer_file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&path)?;
        let reader_file = File::open(&path)?;

        let mut writer = BufWriter::new(writer_file);
        let mut reader = BufReader::new(reader_file);

        if is_new {
            write_header(&mut writer)?;
            writer.flush()?;
        } else {
            read_header(&mut reader)?;
        }

        // Start writing at end of file
        let write_pos = writer.seek(SeekFrom::End(0))?;

        let mut store = KvStore {
            path,
            writer,
            reader,
            index: HashMap::new(),
            write_pos,
        };

        if !is_new {
            store.rebuild_index()?;
        }

        Ok(store)
    }
}
```

What's going on:

- Open the file twice — once with a writer handle (append mode via `SeekFrom::End(0)`), once with a reader handle. Two handles lets us read and write concurrently from the same process without `BufReader` and `BufWriter` fighting over the cursor.
- `OpenOptions::create(true)` creates the file if it doesn't exist.
- On a brand-new file, write the header. On an existing file, read and validate the header.
- Seek the writer to end — all future writes append.
- Rebuild the index by scanning the log.

### Rebuilding the index

```rust
    fn rebuild_index(&mut self) -> io::Result<()> {
        self.reader.seek(SeekFrom::Start(HEADER_LEN as u64))?;
        self.index.clear();

        loop {
            let offset = self.reader.stream_position()?;
            let bytes = match read_record(&mut self.reader)? {
                Some(b) => b,
                None => break,  // clean EOF
            };

            let entry: LogEntry = bincode::deserialize(&bytes)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

            match entry.value {
                LogValue::Data(_) => {
                    self.index.insert(entry.key, offset);
                }
                LogValue::Tombstone => {
                    self.index.remove(&entry.key);
                }
            }
        }
        Ok(())
    }
```

For each record in the log, parse the `LogEntry`. A `Data` entry becomes (or overwrites) an index entry at the record's offset. A `Tombstone` removes the key from the index. After scanning, the index reflects the current state.

This is the full crash recovery story for Bitcask: shut down anywhere, restart, re-scan the log, you're back where you were. Simple enough to *prove* correct, which is why Bitcask has a reputation for reliability.

## Step 3 — Put and delete

```rust
    pub fn put(&mut self, key: String, value: Vec<u8>) -> io::Result<()> {
        let entry = LogEntry {
            key: key.clone(),
            value: LogValue::Data(value),
        };
        let offset = self.write_pos;
        let bytes = bincode::serialize(&entry)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        write_record(&mut self.writer, &bytes)?;
        self.writer.flush()?;
        self.write_pos = self.writer.stream_position()?;

        self.index.insert(key, offset);
        Ok(())
    }

    pub fn delete(&mut self, key: &str) -> io::Result<bool> {
        if !self.index.contains_key(key) {
            return Ok(false);
        }

        let entry = LogEntry {
            key: key.to_string(),
            value: LogValue::Tombstone,
        };
        let bytes = bincode::serialize(&entry)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        write_record(&mut self.writer, &bytes)?;
        self.writer.flush()?;
        self.write_pos = self.writer.stream_position()?;

        self.index.remove(key);
        Ok(true)
    }
```

Key things:

- We capture `write_pos` *before* writing — that's the offset to put in the index.
- We flush after every write. Every `put` is a syscall. Slower, but guarantees the bytes reach the OS before we acknowledge the write.
- `delete` returns `bool`: `true` if the key existed, `false` otherwise. No tombstone is written for non-existent keys (no point).
- `put` consumes the `value: Vec<u8>`. We don't need it after serialization.

Note there's a subtle durability gap: `flush()` on a `BufWriter` only pushes bytes to the OS. The OS may still be holding them in cache, not written to the actual disk. A power cut can still lose that data. We fix this on Day 20 with `sync_all()`.

## Step 4 — Get

```rust
    pub fn get(&mut self, key: &str) -> io::Result<Option<Vec<u8>>> {
        let offset = match self.index.get(key) {
            Some(&o) => o,
            None => return Ok(None),
        };

        self.reader.seek(SeekFrom::Start(offset))?;
        let bytes = read_record(&mut self.reader)?
            .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "record missing at indexed offset"))?;

        let entry: LogEntry = bincode::deserialize(&bytes)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        match entry.value {
            LogValue::Data(v) => Ok(Some(v)),
            LogValue::Tombstone => Ok(None),  // shouldn't happen if index is correct
        }
    }
```

Index lookup is O(1). Seek + read is one disk operation. This is why Bitcask is fast.

The `Tombstone` case in the match is defensive — if the index ever points at a tombstone, something's gone wrong (index rebuild should have removed it). Returning `None` is the safe handling; in production you'd log a warning.

## Step 5 — List keys

```rust
    pub fn keys(&self) -> Vec<&String> {
        self.index.keys().collect()
    }
```

Simple — the in-memory index is the list of live keys.

## Step 6 — Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn temp_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("rkvs_test_{}_{}.rkvs", name, std::process::id()))
    }

    #[test]
    fn put_then_get() {
        let path = temp_path("put_get");
        {
            let mut store = KvStore::open(&path).unwrap();
            store.put("hello".into(), b"world".to_vec()).unwrap();
            assert_eq!(store.get("hello").unwrap(), Some(b"world".to_vec()));
            assert_eq!(store.get("missing").unwrap(), None);
        }
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn update_overwrites() {
        let path = temp_path("update");
        {
            let mut store = KvStore::open(&path).unwrap();
            store.put("k".into(), b"v1".to_vec()).unwrap();
            store.put("k".into(), b"v2".to_vec()).unwrap();
            assert_eq!(store.get("k").unwrap(), Some(b"v2".to_vec()));
        }
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn delete_then_get() {
        let path = temp_path("delete");
        {
            let mut store = KvStore::open(&path).unwrap();
            store.put("k".into(), b"v".to_vec()).unwrap();
            assert!(store.delete("k").unwrap());
            assert_eq!(store.get("k").unwrap(), None);
            assert!(!store.delete("k").unwrap());  // second delete is false
        }
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn survives_reopen() {
        let path = temp_path("reopen");

        {
            let mut store = KvStore::open(&path).unwrap();
            store.put("a".into(), b"1".to_vec()).unwrap();
            store.put("b".into(), b"2".to_vec()).unwrap();
            store.put("c".into(), b"3".to_vec()).unwrap();
            store.delete("b").unwrap();
            store.put("a".into(), b"updated".to_vec()).unwrap();
        }

        {
            let mut store = KvStore::open(&path).unwrap();
            assert_eq!(store.get("a").unwrap(), Some(b"updated".to_vec()));
            assert_eq!(store.get("b").unwrap(), None);
            assert_eq!(store.get("c").unwrap(), Some(b"3".to_vec()));
        }

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn many_keys() {
        let path = temp_path("many");

        {
            let mut store = KvStore::open(&path).unwrap();
            for i in 0..1000 {
                store.put(format!("key{:04}", i), format!("value{}", i).into_bytes()).unwrap();
            }
        }

        {
            let mut store = KvStore::open(&path).unwrap();
            for i in 0..1000 {
                let got = store.get(&format!("key{:04}", i)).unwrap();
                assert_eq!(got, Some(format!("value{}", i).into_bytes()));
            }
        }

        std::fs::remove_file(&path).ok();
    }
}
```

The `survives_reopen` test is the important one — it proves crash recovery works. Close the store, open it again, and everything's still there with the correct values.

Register the module and run:

```rust
// src/main.rs
mod format;
mod save;
mod store;
```

```sh
cargo test store::tests
```

Five tests should pass.

## Step 7 — The CLI

Rewrite `src/main.rs`:

```rust
mod format;
mod save;
mod store;

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::process::ExitCode;

use store::KvStore;

#[derive(Parser)]
#[command(name = "rkvs", about = "a tiny append-only KV store")]
struct Cli {
    #[arg(short, long, default_value = "store.rkvs")]
    file: PathBuf,

    #[command(subcommand)]
    cmd: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Store a key with a string value
    Put { key: String, value: String },
    /// Retrieve the value for a key
    Get { key: String },
    /// Remove a key
    Delete { key: String },
    /// List all keys currently stored
    List,
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    let mut store = match KvStore::open(&cli.file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("failed to open {}: {}", cli.file.display(), e);
            return ExitCode::from(2);
        }
    };

    let result = match cli.cmd {
        Command::Put { key, value } => store.put(key, value.into_bytes()).map(|_| ()),
        Command::Get { key } => match store.get(&key) {
            Ok(Some(v)) => {
                // Print the value. If it's valid UTF-8, print as text; otherwise hex.
                match std::str::from_utf8(&v) {
                    Ok(s) => println!("{}", s),
                    Err(_) => {
                        for b in &v {
                            print!("{:02x}", b);
                        }
                        println!();
                    }
                }
                Ok(())
            }
            Ok(None) => {
                eprintln!("key not found");
                return ExitCode::from(1);
            }
            Err(e) => Err(e),
        },
        Command::Delete { key } => match store.delete(&key) {
            Ok(true) => Ok(()),
            Ok(false) => {
                eprintln!("key not found");
                return ExitCode::from(1);
            }
            Err(e) => Err(e),
        },
        Command::List => {
            let mut keys = store.keys();
            keys.sort();
            for k in keys {
                println!("{}", k);
            }
            Ok(())
        }
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {}", e);
            ExitCode::FAILURE
        }
    }
}
```

A few things to notice:

- `ExitCode` is the modern way to return non-zero from `main`. Exit code `0` = success, `1` = key not found (shell-friendly), `2` = real error.
- `clap`'s derive API reads doc comments (`///`) and turns them into help text. Run `--help` and see.
- We try UTF-8 decoding on `get`, fall back to hex. Keeps the CLI usable for both text and binary values.

## Step 8 — Try it

```sh
cargo build --release
./target/release/rkvs --help
```

Output:

```
a tiny append-only KV store

Usage: rkvs [OPTIONS] <COMMAND>

Commands:
  put     Store a key with a string value
  get     Retrieve the value for a key
  delete  Remove a key
  list    List all keys currently stored
  help    Print this message or the help of the given subcommand(s)

Options:
  -f, --file <FILE>  [default: store.rkvs]
  -h, --help         Print help
```

Run some commands:

```sh
./target/release/rkvs put name Alice
./target/release/rkvs put favorite_color blue
./target/release/rkvs put hp 100
./target/release/rkvs get name
# prints: Alice

./target/release/rkvs list
# prints:
#   favorite_color
#   hp
#   name

./target/release/rkvs delete hp
./target/release/rkvs list
# prints:
#   favorite_color
#   name
```

Close the shell, reopen it, run `list` again. Everything survives:

```sh
./target/release/rkvs list
```

The state comes back because `open` rebuilds the index from the log on startup.

## Step 9 — Benchmark it

Let's see how fast this actually is. Add a benchmark option to the CLI temporarily, or just write a one-off test:

```rust
#[test]
#[ignore]  // cargo test -- --ignored
fn bench_puts() {
    let path = temp_path("bench");
    std::fs::remove_file(&path).ok();

    let start = std::time::Instant::now();
    {
        let mut store = KvStore::open(&path).unwrap();
        for i in 0..10_000 {
            store.put(format!("k{:06}", i), vec![0xAB; 100]).unwrap();
        }
    }
    let elapsed = start.elapsed();
    let size = std::fs::metadata(&path).unwrap().len();
    println!(
        "10k puts in {:?} — {:.0} puts/sec, file size {} bytes",
        elapsed,
        10_000.0 / elapsed.as_secs_f64(),
        size,
    );

    std::fs::remove_file(&path).ok();
}
```

Run:

```sh
cargo test --release bench_puts -- --ignored --nocapture
```

Typical output on a modern laptop SSD:

```
10k puts in 124.5ms — 80321 puts/sec, file size 1480008 bytes
```

~80k puts per second. For a 300-line database with a `flush()` on every write. Not bad.

If you comment out the `self.writer.flush()?;` in `put`, you'll see numbers go up dramatically — perhaps 10x — at the cost of durability. This is the tradeoff Day 20 will explore properly.

## Common pitfalls

### Forgetting to update `write_pos` after writing

If you `flush()` and then try to read `self.writer.stream_position()`, but don't cache the value, every `put` takes an extra syscall. We cache `write_pos` so the hot loop is pure-memory.

### Two `File` handles fighting

The reader and writer share a file. On most OSes this is fine — writes are visible to the reader after flush. But seeking the writer to 0 to read would drop the write position. Always `SeekFrom::End(0)` or track `write_pos` yourself.

### Index grows unbounded

Every update adds an entry to the log, but the old record stays. Over time, your file is mostly dead records. A real Bitcask runs a background compaction thread that rewrites the log containing only the entries the index points to. We skipped this for today. The exercise builds it.

### Not handling the empty-file case on open

If you `cargo run get whatever` without ever writing, you'd previously read the (empty, no-header) file and crash. The `is_new = !path.exists()` branch writes the header on first open. Test by deleting the file and opening cold.

### `Vec<u8>` vs `String` in the schema

We stored values as `Vec<u8>` so the store is binary-safe. Keys are `String` for CLI convenience. If you need binary keys, use `Vec<u8>` there too and pass them hex-encoded in the CLI.

### Overwriting records doesn't free disk

`put("x", a)` then `put("x", b)` — both records are in the file. Disk usage only grows. The index is tight, but the file isn't. Compaction is the fix.

## What you learned

- **Append-only logs** are crash-resilient and simple.
- **In-memory index** (`HashMap<Key, Offset>`) makes reads O(1) + one seek.
- **Tombstones** let you represent deletes in an append-only world.
- **Crash recovery** is free: scan the log, rebuild the index.
- **`io::Seek`** on `File` and `BufReader<File>` for random reads.
- **`clap` derive** gives you CLI parsing with help text from struct fields.
- **`ExitCode`** is the proper way to return non-zero exit status from `main`.
- Every `put` syscall costs a flush — there's a real throughput/durability tradeoff.
- Log compaction is essential for a long-running database; we skipped it today.

## Exercises

1. **Compaction.** Implement `KvStore::compact(&mut self) -> io::Result<()>`. Write a new file containing only live records (one per key, latest). Swap it in (rename `compacted.tmp` → the original path; tmp-then-rename is atomic on POSIX). Measure: what's file size before and after compaction for a workload that updates the same 1000 keys 100 times each?
2. **Generic values.** Make `KvStore` generic over the value type: `KvStore<V: Serialize + DeserializeOwned>`. What changes? What about the keys?
3. **TTL support.** Add an optional `expires_at: Option<SystemTime>` to `LogEntry`. On `get`, return `None` if the entry is expired. How does this interact with index rebuilding at startup?
4. **File size limit.** Add a `max_file_size: u64` config field. When the log exceeds it, automatically trigger compaction. Test that a long-running workload stays under the limit.
5. **Range queries.** Replace the `HashMap<String, u64>` index with `BTreeMap<String, u64>`. Add a `range(start: &str, end: &str) -> Vec<(String, Vec<u8>)>` method. What's the performance difference for point lookups? This is Day 18 territory.

## What's next

Day 18 swaps out the HashMap index for a **B-tree you'll write yourself**, generic over key type, with splits and range queries. You'll test it against `std::collections::BTreeMap` to verify correctness. The goal is to understand what's inside the data structure every database ships with.

→ [Day 18 — A B-tree From Scratch](day-18.md)
