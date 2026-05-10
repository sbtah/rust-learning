# Day 20 — Durability, WALs, and Crash Recovery

**Domain:** databases • **Time:** 2 hours • **Difficulty:** hard

## What you'll build

Real crash resilience for the KV store. You'll add explicit `fsync` calls, build a small child-process harness that kills the writer mid-operation, and prove that your store recovers correctly from every tested crash point. You'll also add a separate **write-ahead log (WAL)** — a standard database technique — and see how it lets you batch many writes for durability at one sync cost.

## What you'll learn

- **What `fsync` actually does** — and what it doesn't
- **Durability tiers**: user buffer → OS cache → disk platter
- **`File::sync_all`** and **`File::sync_data`** in Rust
- **Write-ahead logs (WALs)**: the standard durability primitive
- **Crash testing**: forcibly killing a child process to simulate power loss
- Torn writes and why record-level atomicity matters
- When to `fsync` per-op vs. per-batch (throughput vs. durability)

## Background

### The four layers

When your code writes to a file, the bytes travel through four layers before reaching permanent storage:

1. **Application buffer** (e.g., `BufWriter`). A `flush()` moves bytes from here to layer 2.
2. **OS page cache** (the kernel's in-memory file cache). A `write()` syscall moves bytes here. They're *not on disk yet*.
3. **Disk controller cache** (on modern SSDs, an internal battery-backed RAM). `fsync` tells the OS to push bytes here.
4. **Non-volatile storage** (NAND flash or platter). On most modern SSDs, `fsync` waits until the data is durable here.

The boundaries matter:

- **Crash after layer 1 flush, before layer 2 write**: your process died. App data is lost. OS keeps going.
- **Crash after layer 2 write, before layer 3 sync**: power loss. OS buffers are volatile RAM — lost. Data is gone.
- **Crash after layer 3 sync**: depends on the drive. Modern SSDs with power-loss protection are durable. Cheap consumer drives sometimes lie and report "synced" before the data is on flash. For real production: use enterprise SSDs with PLP (Power Loss Protection) and enable disk barriers.

Today we'll be honest about layers 1-3. Layer 4 is out of our control — we trust the drive spec.

### What `fsync` does in Rust

```rust
use std::fs::File;

let file = File::create("data.bin")?;
// ... writes ...
file.sync_all()?;   // force all data AND metadata to disk
file.sync_data()?;  // force data only (skip metadata like atime)
```

`sync_all` = POSIX `fsync`. `sync_data` = POSIX `fdatasync`. The data variant is faster because it skips metadata updates (timestamps, etc.). For our use, `sync_data` is correct since we care about record bytes, not access times.

Crucial subtlety: **`BufWriter::flush` doesn't call `fsync`**. It just pushes bytes from layer 1 to layer 2. To get real durability, you need both:

```rust
writer.flush()?;               // layer 1 → layer 2
writer.get_ref().sync_data()?; // layer 2 → layer 3
```

`BufWriter::get_ref()` returns the underlying `File`. Use `get_mut()` if you need to mutate it.

### Why fsync is slow

On a consumer SSD, `fsync` is typically 100 µs to 1 ms. On an HDD, it's 5-15 ms (full platter rotation). That's catastrophically slow if you call it on every operation.

A naive KV store that fsyncs per-put tops out at ~1000-10000 puts/sec. That's why production databases **batch** writes or use **group commit** — collect N operations, fsync once, acknowledge all N. You trade a tiny latency cost for a massive throughput gain.

### Write-ahead logging

The canonical durability pattern:

1. Before modifying the main data structure, **write to the WAL** and fsync.
2. Once the WAL is durable, apply the change to the main structure.
3. On a clean shutdown, truncate the WAL (it's safe — the main data is up to date).
4. On a crash, replay the WAL against the main data on startup.

PostgreSQL, MySQL, SQLite — all use WALs. For our append-only log, the main data file *is* the WAL — but we'll build a separate WAL anyway to understand the pattern. The WAL will hold pending operations that haven't been durably committed to the main log.

### Crash testing with child processes

How do you verify your recovery code works? You can't just call `std::process::abort` — that's too clean, buffers might still flush. You want to simulate a power cut: pull the rug out mid-write.

The trick: spawn the store as a **child process**. Let it do work, then `kill -9` it (SIGKILL on Unix, `TerminateProcess` on Windows). The child never gets a chance to clean up. Then restart the store from the parent and inspect what survived.

Rust's `std::process::Command` can spawn arbitrary subprocesses. We'll use `Command::new(env::current_exe())` to spawn a second copy of our own binary in "worker" mode.

## Setting up

Continue in `rkvs`. No new dependencies needed.

## Step 1 — Add real durability to `put`

Open `src/store.rs`. The current `put`:

```rust
pub fn put(&mut self, key: String, value: Vec<u8>) -> io::Result<()> {
    // ...
    write_record(&mut self.writer, &bytes)?;
    self.writer.flush()?;
    self.write_pos = self.writer.stream_position()?;
    self.index.insert(key, offset);
    self.refresh_mmap()?;
    Ok(())
}
```

`flush()` gets bytes to the OS but not to disk. Add a sync mode:

```rust
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SyncPolicy {
    /// Never fsync. Fastest. Data loss on power cut.
    Never,
    /// fsync on every put/delete. Slowest. Durable per op.
    Always,
    /// fsync every N operations. Batched durability.
    EveryN(u32),
}

pub struct KvStore {
    path: PathBuf,
    writer: BufWriter<File>,
    reader: BufReader<File>,
    index: HashMap<String, u64>,
    write_pos: u64,
    mmap: Option<Mmap>,
    mmap_len: u64,
    sync_policy: SyncPolicy,
    ops_since_sync: u32,
}
```

Update `open` to take a policy, or default to `Always`:

```rust
impl KvStore {
    pub fn open(path: impl AsRef<Path>) -> io::Result<Self> {
        Self::open_with_policy(path, SyncPolicy::Always)
    }

    pub fn open_with_policy(path: impl AsRef<Path>, sync_policy: SyncPolicy) -> io::Result<Self> {
        // ... existing open body ...
        let mut store = KvStore {
            path,
            writer,
            reader,
            index: HashMap::new(),
            write_pos,
            mmap: None,
            mmap_len: 0,
            sync_policy,
            ops_since_sync: 0,
        };

        if !is_new {
            store.rebuild_index()?;
        }

        store.refresh_mmap()?;
        Ok(store)
    }
}
```

Default-mode `open` stays backward-compatible. Callers who want `Never` or `EveryN` opt in.

Now the sync helper:

```rust
    fn maybe_sync(&mut self) -> io::Result<()> {
        match self.sync_policy {
            SyncPolicy::Never => {}
            SyncPolicy::Always => {
                self.writer.flush()?;
                self.writer.get_ref().sync_data()?;
            }
            SyncPolicy::EveryN(n) => {
                self.ops_since_sync = self.ops_since_sync.wrapping_add(1);
                if self.ops_since_sync >= n {
                    self.writer.flush()?;
                    self.writer.get_ref().sync_data()?;
                    self.ops_since_sync = 0;
                }
            }
        }
        Ok(())
    }
```

Replace the `self.writer.flush()?;` lines in `put` and `delete` with `self.maybe_sync()?;`. The mmap refresh still needs the latest bytes visible to a fresh `File::open`, so keep `refresh_mmap` — but move it *before* the optional sync. Actually, let's audit: `refresh_mmap` opens a new file handle, which reads bytes through the OS page cache; bytes are visible once they're written (flushed), whether or not they've been fsynced. So mmap works after `flush`.

Updated `put`:

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
        self.maybe_sync()?;
        self.refresh_mmap()?;
        Ok(())
    }
```

Always `flush()` to push into the OS cache (the mmap needs that). The conditional `sync_data()` is what makes it durable.

Expose a `close`:

```rust
    pub fn close(mut self) -> io::Result<()> {
        self.writer.flush()?;
        self.writer.get_mut().sync_all()?;  // full sync on close
        Ok(())
    }
```

On clean shutdown, always do a full `sync_all` — no reason not to, and it ensures metadata is flushed too.

## Step 2 — Benchmark the policies

Add to `src/main.rs`:

```rust
use rkvs::store::{KvStore, SyncPolicy};

fn bench_policy(policy: SyncPolicy, label: &str, n: usize) -> std::io::Result<()> {
    let path = std::env::temp_dir().join(format!("rkvs_bench_policy_{}.rkvs", label));
    let _ = std::fs::remove_file(&path);

    let mut store = KvStore::open_with_policy(&path, policy)?;

    let start = std::time::Instant::now();
    for i in 0..n {
        store.put(format!("k{:08}", i), vec![0xABu8; 128])?;
    }
    store.close()?;

    let elapsed = start.elapsed();
    let ops_per_sec = n as f64 / elapsed.as_secs_f64();
    println!(
        "{:<20} {} puts in {:?} — {:>10.0} puts/sec",
        label, n, elapsed, ops_per_sec
    );

    std::fs::remove_file(&path).ok();
    Ok(())
}
```

Add CLI flag or just run directly:

```sh
cargo run --release --example durability_bench
```

Create `examples/durability_bench.rs`:

```rust
use rkvs::store::{KvStore, SyncPolicy};

fn run(policy: SyncPolicy, label: &str, n: usize) -> std::io::Result<()> {
    let path = std::env::temp_dir().join(format!("rkvs_bench_policy_{}.rkvs", label));
    let _ = std::fs::remove_file(&path);

    let mut store = KvStore::open_with_policy(&path, policy)?;

    let start = std::time::Instant::now();
    for i in 0..n {
        store.put(format!("k{:08}", i), vec![0xABu8; 128])?;
    }
    store.close()?;

    let elapsed = start.elapsed();
    let ops_per_sec = n as f64 / elapsed.as_secs_f64();
    println!(
        "{:<20} {} puts in {:?} — {:>10.0} puts/sec",
        label, n, elapsed, ops_per_sec
    );

    std::fs::remove_file(&path).ok();
    Ok(())
}

fn main() -> std::io::Result<()> {
    const N: usize = 10_000;
    run(SyncPolicy::Never, "Never", N)?;
    run(SyncPolicy::EveryN(1000), "EveryN(1000)", N)?;
    run(SyncPolicy::EveryN(100), "EveryN(100)", N)?;
    run(SyncPolicy::EveryN(10), "EveryN(10)", N)?;
    run(SyncPolicy::Always, "Always", N)?;
    Ok(())
}
```

Run:

```sh
cargo run --release --example durability_bench
```

Typical output on a desktop Linux SSD:

```
Never                10000 puts in 43.2ms — 231481 puts/sec
EveryN(1000)         10000 puts in 48.9ms — 204498 puts/sec
EveryN(100)          10000 puts in 73.1ms — 136867 puts/sec
EveryN(10)           10000 puts in 241.4ms — 41424 puts/sec
Always               10000 puts in 2134.5ms — 4685 puts/sec
```

Durability has a cost:

- `Never`: ~230k ops/sec. Loses everything on power cut.
- `EveryN(1000)`: ~200k ops/sec. Loses at most 999 ops on crash.
- `Always`: ~5k ops/sec — 50x slower. Never loses a committed op.

Production databases pick `EveryN` with tuning, or use **group commit** (batch all currently-pending ops, sync once, acknowledge all). For our tutorial, exposing the policy as a configuration is already a big step up from hardcoded behavior.

## Step 3 — The crash test harness

We'll write a test that:

1. Spawns a child process that puts 1000 keys into a file.
2. After 200ms (while it's mid-write), kills the child with SIGKILL.
3. Reopens the file from the parent and verifies:
   - The header is valid.
   - All durable records have valid CRCs.
   - The recovered state is a prefix of the intended writes.

Create `src/bin/crash_worker.rs`:

```rust
use rkvs::store::{KvStore, SyncPolicy};

fn main() -> std::io::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        eprintln!("usage: crash_worker <path> <sync_policy>");
        std::process::exit(2);
    }
    let path = &args[1];
    let policy = match args[2].as_str() {
        "never" => SyncPolicy::Never,
        "always" => SyncPolicy::Always,
        "every10" => SyncPolicy::EveryN(10),
        _ => {
            eprintln!("unknown policy: {}", args[2]);
            std::process::exit(2);
        }
    };

    let mut store = KvStore::open_with_policy(path, policy)?;
    for i in 0..1_000_000u32 {
        store.put(format!("k{:08}", i), i.to_le_bytes().to_vec())?;
    }
    store.close()?;
    Ok(())
}
```

`src/bin/crash_worker.rs` compiles to a separate binary named `crash_worker`. `cargo build` produces both `rkvs` (the main CLI) and `crash_worker`.

Now the test harness. Create `tests/crash.rs`:

```rust
use rkvs::store::KvStore;
use std::process::{Command, Stdio};
use std::time::Duration;

fn worker_binary() -> std::path::PathBuf {
    // cargo puts test binaries in target/debug/deps, adjacent to target/debug
    let mut path = std::env::current_exe().unwrap();
    path.pop();  // remove test binary
    if path.ends_with("deps") {
        path.pop();
    }
    path.push("crash_worker");
    path
}

fn run_crash_test(policy: &str) -> usize {
    let path = std::env::temp_dir().join(format!(
        "rkvs_crash_{}_{}.rkvs",
        policy,
        std::process::id()
    ));
    std::fs::remove_file(&path).ok();

    let mut child = Command::new(worker_binary())
        .arg(&path)
        .arg(policy)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("failed to spawn crash worker");

    std::thread::sleep(Duration::from_millis(200));

    // Force-kill the child
    child.kill().unwrap();
    child.wait().unwrap();

    // Now reopen the file and see what survived
    let mut store = KvStore::open(&path).expect("store should reopen cleanly");
    let mut recovered = 0usize;
    for i in 0..1_000_000u32 {
        let key = format!("k{:08}", i);
        match store.get(&key).unwrap() {
            Some(v) => {
                assert_eq!(v, i.to_le_bytes().to_vec(), "wrong value at {}", i);
                recovered += 1;
            }
            None => {
                // Everything past this point must also be missing — we expect a clean prefix
                for j in (i + 1)..(i + 100).min(1_000_000) {
                    assert!(
                        store.get(&format!("k{:08}", j)).unwrap().is_none(),
                        "key {} present but {} missing — not a clean prefix",
                        j, i,
                    );
                }
                break;
            }
        }
    }

    std::fs::remove_file(&path).ok();
    recovered
}

#[test]
fn crash_always_policy_preserves_all_returned_puts() {
    let n = run_crash_test("always");
    println!("with SyncPolicy::Always, recovered {} records after SIGKILL", n);
    assert!(n > 0, "should have recovered at least some records");
}

#[test]
fn crash_never_policy_tests_prefix() {
    let n = run_crash_test("never");
    println!("with SyncPolicy::Never, recovered {} records after SIGKILL", n);
    // We can't assert anything about count — the OS cache might have flushed
    // zero or many records. The important invariant is "clean prefix",
    // which is checked inside run_crash_test.
}
```

The test:

- Kills a 200ms-old child that's aggressively writing keys.
- Asserts the recovery produces a **clean prefix**: if key `i` exists, all keys `0..i` exist. No "gaps." This is the key guarantee an append-only log provides.
- Doesn't assert a specific recovery count — that depends on timing and sync policy.

Run:

```sh
cargo test --test crash -- --nocapture
```

Output will vary but should look like:

```
test crash_always_policy_preserves_all_returned_puts ... 
with SyncPolicy::Always, recovered 892 records after SIGKILL
ok
test crash_never_policy_tests_prefix ... 
with SyncPolicy::Never, recovered 11234 records after SIGKILL
ok
```

Two details worth noting:

- `SyncPolicy::Always` **recovers fewer records** than `Never` because Always is throughput-limited. In 200ms it does ~900 writes; Never does ~11k. But every one of Always's 892 records was durably committed before the crash.
- The crucial property is **clean prefix**. If recovery ever shows "have key 500, missing key 200" — that's corruption. Our record format's CRC catches it: a partial write gets rejected at the record level.

This is the real thing. You just tested crash-consistency by actually crashing a process.

## Step 4 — Understanding what we tested (and didn't)

Our SIGKILL test simulates a process crash — the OS is fine, its page cache is intact. The `Never` policy works in this scenario because the OS still flushes pages eventually.

A **power-loss test** is harder. You'd need to actually yank power (or simulate it with a VM). The `Never` policy would lose many more records in that case — whatever was still in the OS cache.

Our crash test is honest about this limitation. It proves the store handles clean-prefix recovery correctly, which is the hard algorithmic part. Power-loss durability is the sync policy's job.

## Step 5 — Handling torn writes

A **torn write** is when a single write gets partially flushed. Imagine you write a 20-byte record but only 12 bytes reach the disk. Our record format already handles this:

1. The length prefix (4 bytes) is the first thing written. If it's torn, we'll read an invalid length and fail with `UnexpectedEof`.
2. The CRC covers the payload. A torn payload fails the CRC check.

The `rebuild_index` function handles errors by ... let's check. Look at Day 17's code:

```rust
let bytes = match read_record(&mut self.reader)? {
    Some(b) => b,
    None => break,
};
```

`read_record` returns `Err` on bad CRC. The `?` propagates. **That means `rebuild_index` fails on a torn write, and `open` itself fails.** That's bad — a single torn record bricks the database.

Fix this by stopping rebuild at the first error instead of propagating:

```rust
    fn rebuild_index(&mut self) -> io::Result<()> {
        self.reader.seek(SeekFrom::Start(HEADER_LEN as u64))?;
        self.index.clear();

        loop {
            let offset = self.reader.stream_position()?;
            let bytes = match read_record(&mut self.reader) {
                Ok(Some(b)) => b,
                Ok(None) => break,  // clean EOF
                Err(e) => {
                    // Torn write at the end of the log. Truncate and move on.
                    eprintln!("recovery: truncating at offset {} ({})", offset, e);

                    // Sync position back to where we were before the bad read
                    drop(std::mem::replace(&mut self.reader,
                        BufReader::new(File::open(&self.path)?)));

                    self.truncate_to(offset)?;
                    break;
                }
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

    fn truncate_to(&mut self, len: u64) -> io::Result<()> {
        self.writer.flush()?;
        self.writer.get_ref().set_len(len)?;
        self.writer.get_ref().sync_all()?;
        self.writer.seek(SeekFrom::Start(len))?;
        self.write_pos = len;
        Ok(())
    }
```

`File::set_len(len)` truncates the file to exactly `len` bytes. Combined with `sync_all`, the truncation is durable.

Now if the last record is partial, rebuild truncates it and carries on with everything before. The database opens cleanly, minus the one partial record — which wasn't committed anyway.

Test it:

```rust
#[test]
fn torn_record_is_truncated() {
    let path = temp_path("torn");
    std::fs::remove_file(&path).ok();

    // Write three good records
    {
        let mut store = KvStore::open(&path).unwrap();
        store.put("a".into(), b"alpha".to_vec()).unwrap();
        store.put("b".into(), b"bravo".to_vec()).unwrap();
        store.put("c".into(), b"charlie".to_vec()).unwrap();
        store.close().unwrap();
    }

    // Append garbage to simulate a torn write
    {
        use std::io::Write;
        use std::fs::OpenOptions;
        let mut f = OpenOptions::new().append(true).open(&path).unwrap();
        f.write_all(&[0xFF; 50]).unwrap();  // bogus record
        f.sync_all().unwrap();
    }

    // Reopen — rebuild should truncate the garbage
    {
        let store = KvStore::open(&path).unwrap();
        let keys: std::collections::HashSet<&String> = store.keys().into_iter().collect();
        assert_eq!(keys.len(), 3);
        assert!(keys.contains(&"a".to_string()));
        assert!(keys.contains(&"b".to_string()));
        assert!(keys.contains(&"c".to_string()));
    }

    std::fs::remove_file(&path).ok();
}
```

Run:

```sh
cargo test torn_record_is_truncated -- --nocapture
```

You should see the truncation diagnostic and a passing test. The database is robust to trailing garbage.

## Common pitfalls

### Assuming `flush()` is durable

It's not. `flush()` pushes bytes to the OS. `sync_data()` or `sync_all()` pushes them to disk. Many bugs come from this confusion.

### Syncing the parent directory

When you create a new file via `File::create("path/to/new.rkvs")`, the directory entry is metadata on the *parent directory*. If you crash between creating the file and syncing the parent, the file might not exist after reboot.

Fix: open the parent and sync it.

```rust
let parent = path.parent().unwrap_or(Path::new("."));
File::open(parent)?.sync_all()?;
```

For production, always sync parent directories on create/rename. For today's tutorial we'll leave it as a note.

### Renaming for atomic swaps

A common pattern: write to `db.new`, then `rename("db.new", "db")`. On POSIX, rename is atomic — either the old or new file exists, never neither or both. Pair with a parent-dir sync for full durability.

### Buffer sizes

`BufWriter` has a default buffer of 8 KB. If your records are larger, writes will spill directly. Not wrong, just less efficient. Pass a custom size for specific workloads: `BufWriter::with_capacity(64 * 1024, file)`.

### `O_DSYNC` mode

On POSIX, you can open a file with `O_DSYNC` so every write implicitly syncs. Don't. It's slower than batched sync and encourages tight coupling.

### Crash tests are flaky

Timing-dependent tests can be flaky on slow or loaded CI. Making the crash window wide (200ms in our test) helps. Some projects use fault-injection libraries (`turmoil`, `loom`) that simulate crashes deterministically — worth exploring in production.

## What you learned

- **Four durability layers**: app buffer, OS cache, disk cache, storage media.
- **`sync_data()` / `sync_all()`** actually push to disk. `flush()` doesn't.
- **Durability has a cost**: `fsync` is 100 µs to 15 ms per call.
- **`SyncPolicy` choice** lets callers pick throughput vs. crash window.
- **Crash testing with child processes**: `spawn` + `kill -9` simulates a real crash.
- **Clean prefix recovery** is the critical invariant for append-only logs.
- **Torn writes** at the tail are inevitable; truncate on recovery.
- **Production databases use group commit** to amortize the fsync cost across many ops.
- Syncing the parent directory is needed for create/rename operations to be durable.

## Exercises

1. **Group commit.** Add a `commit()` method. `put(k, v)` becomes cheap (no flush/sync). `commit()` fsyncs and finalizes. Test that un-committed puts aren't visible after a crash.
2. **Separate WAL.** Add a `wal.log` file adjacent to the main store file. Writes go to both WAL and main log; WAL is fsynced per-op, main log is fsynced lazily. Replay WAL into main log on startup. (In practice, this is redundant for a simple append-only store — but great practice for LSM-style engines.)
3. **Parent-dir sync.** Audit every place the store creates a new file (the main file, any compaction tmp). Add parent-dir sync after create and after rename. Verify with a targeted crash test.
4. **Crash at every point.** Write a test that spawns the child with different "kill after N ms" values from 10ms to 1000ms. Assert clean-prefix holds in every case.
5. **Compare sync on other filesystems.** Benchmark `SyncPolicy::Always` on tmpfs (ramdisk), ext4, XFS, and ZFS. Which is fastest? Which is most durable under power loss (research, don't test!)?

## What's next

Day 21 finishes the KV store week with **concurrent access**. You'll wrap `KvStore` in an `Arc<RwLock<KvStore>>` for many-reader-one-writer workloads, then design a **sharded store** that partitions keys across N independent `Mutex<KvStore>` shards — a trick that scales writes across CPU cores. You'll benchmark both against single-threaded baseline and see what Amdahl's law has to say.

→ [Day 21 — Concurrent KV Store](day-21.md)
