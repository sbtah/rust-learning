# Day 19 — Memory-Mapped Files and `unsafe`

**Domain:** databases • **Time:** 2 hours • **Difficulty:** hard

## What you'll build

A read path for the Bitcask store that uses **memory-mapped I/O** instead of `seek` + `read`. You'll benchmark it against the Day 17 path and see a 2-5x speedup on read-heavy workloads. Along the way, you'll meet Rust's `unsafe` keyword — what it actually means, when to use it, and how to document and contain it. You'll use exactly one `unsafe` block, with a `SAFETY:` comment explaining why it's sound.

## What you'll learn

- **Memory-mapped I/O** — the OS maps a file into your address space
- Why **mmap is fast** for random-access read patterns
- `unsafe` in Rust: what it *actually* means (hint: not "disable safety checks")
- The **`memmap2`** crate and its safe API
- Writing a `SAFETY:` comment that explains why an unsafe block is sound
- When mmap is wrong (small files, writes, async)
- **Criterion** benchmarks for a fair comparison

## Background

### What is mmap?

A regular read looks like this:

1. Your program calls `read(fd, buf, 4096)`.
2. OS switches context from userspace to kernel.
3. Kernel copies 4096 bytes from the file's cache (or disk) into `buf`.
4. Context switches back. Your program continues.

A memory-mapped read looks like this:

1. Once at startup, you call `mmap(fd, length)`. The OS associates a region of your process's virtual address space with the file's pages.
2. Your program reads `mapped[offset]` like any pointer dereference.
3. If the file's page is already cached in the OS's page cache, that's a plain memory read — nanoseconds.
4. If the page isn't cached, the CPU triggers a page fault, the OS pulls the page from disk, and your read proceeds.

The payoff: **no syscalls for cache hits**. On a file whose pages are all resident (common for hot read paths), every read is as fast as reading memory. On a file with cold pages, you get one page fault per page — same cost as a `read` but no userspace/kernel copy.

### Why mmap isn't a silver bullet

- **Writes are complicated.** Modifying a memory-mapped file can leave the disk state inconsistent. `msync` forces pages back to disk. For our read-only use case, not an issue.
- **File size limits.** On 32-bit targets, you can't mmap files larger than ~3 GB. On 64-bit, the limit is address space (huge). We're fine.
- **The OS decides when to page.** You can't force eviction or control the cache directly. For latency-sensitive systems, this non-determinism matters.
- **Files changing under you.** If another process extends or truncates the file, your mmap might point at stale or missing pages. For single-process DBs, not an issue.
- **Small files.** Mapping a 4 KB file is overhead you don't need. Mmap wins on GB-scale data. For a KV store that might hold GBs, it's exactly right.

### What `unsafe` means

Every Python programmer's first reaction to `unsafe` in Rust: "so is it like C now, anything goes?"

No. `unsafe` in Rust means: *the compiler can't prove this code doesn't violate the safety rules — you, the programmer, must prove it*. The safety rules are unchanged. An `unsafe` block is a *promise* from you to the compiler that the invariants hold.

The main things you can do in `unsafe` that you can't in safe code:

1. Dereference raw pointers (`*const T`, `*mut T`).
2. Call unsafe functions (including FFI functions from C libraries).
3. Read/write mutable statics.
4. Implement unsafe traits (like `Send` or `Sync` manually).
5. Access fields of `union`s.

If you violate safety invariants inside `unsafe`, you get undefined behavior — just like in C. Rust doesn't magically prevent that; it just makes you opt in.

The culture: every `unsafe` block should have a `// SAFETY: ...` comment explaining why it's sound. If you can't explain it, it's probably not. Reviewers should push back hard on `unsafe` blocks without good comments.

### `memmap2` — a safe API wrapping mmap

The `memmap2` crate wraps the `mmap` syscall. Its API is *mostly* safe — with one carefully documented exception: creating the mapping is `unsafe`, because the caller is responsible for ensuring the file doesn't change externally while mapped.

```rust
use memmap2::Mmap;
use std::fs::File;

let file = File::open("data.bin")?;

// SAFETY: we don't modify this file from elsewhere in the process,
// and no other process is writing to it.
let mmap = unsafe { Mmap::map(&file)? };

// `mmap` is now a [u8] you can index into
let first_byte = mmap[0];
```

Once you have the `Mmap`, dereferencing it is safe — it implements `Deref<Target = [u8]>` so you can use it like a slice. All indexing is bounds-checked. Only the *creation* of the mapping is unsafe.

## Setting up

```sh
cd rkvs
cargo add memmap2
cargo add criterion --dev
```

For criterion benchmarks, we also need a `[[bench]]` entry. Add to `Cargo.toml`:

```toml
[dev-dependencies]
proptest = "1"
criterion = "0.5"

[[bench]]
name = "kv_reads"
harness = false
```

Create the directory structure:

```sh
mkdir benches
```

We'll put the benchmark in `benches/kv_reads.rs` later.

## Step 1 — Understanding a file-backed mmap

A warmup. Create `examples/mmap_demo.rs` (Cargo auto-runs files in `examples/` with `cargo run --example`):

```rust
use memmap2::Mmap;
use std::fs::{File, OpenOptions};
use std::io::Write;

fn main() -> std::io::Result<()> {
    // Write a known file
    {
        let mut file = OpenOptions::new()
            .read(true).write(true).create(true).truncate(true)
            .open("demo.bin")?;
        file.write_all(b"Hello, mmap!")?;
    }

    // Map it
    let file = File::open("demo.bin")?;

    // SAFETY: we just wrote this file and no other process or thread is
    // mutating it while this Mmap lives.
    let mmap = unsafe { Mmap::map(&file)? };

    println!("file is {} bytes", mmap.len());
    println!("first 5 bytes as str: {}", std::str::from_utf8(&mmap[0..5]).unwrap());
    println!("raw bytes: {:?}", &mmap[..]);

    Ok(())
}
```

Run:

```sh
cargo run --example mmap_demo
```

Expected output:

```
file is 12 bytes
first 5 bytes as str: Hello
raw bytes: [72, 101, 108, 108, 111, 44, 32, 109, 109, 97, 112, 33]
```

Notice how `mmap[0..5]` is just slice indexing. Zero-cost. The OS already mapped the bytes — we're reading them like any other `&[u8]`.

## Step 2 — Read helper that works from a byte slice

Refactor `format::read_record` to have a companion that works on `&[u8]` instead of `impl Read`. Add to `src/format.rs`:

```rust
/// Parse one record from a byte slice starting at `offset`.
/// Returns the payload and the new offset (pointing at the next record).
pub fn read_record_slice(buf: &[u8], offset: usize) -> io::Result<Option<(Vec<u8>, usize)>> {
    if offset == buf.len() {
        return Ok(None);
    }
    if offset + 8 > buf.len() {
        return Err(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "incomplete record header",
        ));
    }

    let len = u32::from_le_bytes(buf[offset..offset + 4].try_into().unwrap()) as usize;
    let expected_crc = u32::from_le_bytes(buf[offset + 4..offset + 8].try_into().unwrap());

    let payload_start = offset + 8;
    let payload_end = payload_start + len;

    if payload_end > buf.len() {
        return Err(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "incomplete record payload",
        ));
    }

    let payload = &buf[payload_start..payload_end];
    let actual_crc = crc32fast::hash(payload);
    if actual_crc != expected_crc {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("CRC mismatch at offset {}", offset),
        ));
    }

    Ok(Some((payload.to_vec(), payload_end)))
}
```

Two things worth noting:

**`try_into().unwrap()`.** We slice `buf[offset..offset+4]`, which is a `&[u8]`. `u32::from_le_bytes` wants `[u8; 4]`. Converting a slice to a fixed-size array is fallible (the slice length might not match), so `try_into()` returns `Result<[u8; 4], _>`. We've already bounds-checked `offset + 8 <= buf.len()`, so the conversion always succeeds — `unwrap` is safe. A more defensive version would return a proper error.

**The return type.** `(Vec<u8>, usize)` — the payload (owned) and the new offset to pass in next time. Caller iterates until `None`. This mirrors the `Iterator::next` pattern.

The design choice: we return an owned `Vec<u8>` for compatibility with existing code. A zero-copy version would return `&[u8]` — no allocation, just slices pointing into the mmap. That's covered in an exercise.

## Step 3 — Rewrite the store to use mmap for reads

Open `src/store.rs`. We'll add a new `get_mmap` method alongside the existing `get`.

First, add the mmap field and a helper to refresh it:

```rust
use memmap2::Mmap;

pub struct KvStore {
    path: PathBuf,
    writer: BufWriter<File>,
    reader: BufReader<File>,
    index: HashMap<String, u64>,
    write_pos: u64,
    // NEW:
    mmap: Option<Mmap>,
    mmap_len: u64,
}
```

Update `open` to initialize the new fields:

```rust
        let mut store = KvStore {
            path,
            writer,
            reader,
            index: HashMap::new(),
            write_pos,
            mmap: None,
            mmap_len: 0,
        };

        if !is_new {
            store.rebuild_index()?;
        }

        store.refresh_mmap()?;
        Ok(store)
```

Add the refresh function:

```rust
impl KvStore {
    fn refresh_mmap(&mut self) -> io::Result<()> {
        let file = File::open(&self.path)?;
        let len = file.metadata()?.len();
        if len == 0 {
            self.mmap = None;
            self.mmap_len = 0;
            return Ok(());
        }

        // SAFETY: this `KvStore` is the only writer to this file (we hold
        // the write handle), and no external process is expected to modify it.
        // The mmap stays valid as long as the underlying file isn't truncated
        // or extended — we guard against the extension case by `refresh_mmap()`
        // after every write.
        let mmap = unsafe { Mmap::map(&file)? };

        self.mmap = Some(mmap);
        self.mmap_len = len;
        Ok(())
    }
}
```

This is our one `unsafe` block. The SAFETY comment:

1. States our uniqueness claim (we're the only writer).
2. Excludes external processes by design.
3. Acknowledges the brittleness (file extension invalidates the mapping) and the mitigation (refresh after writes).

A reviewer can read this and decide whether they agree with the argument. That's the point of SAFETY comments.

Add a call at the end of `put` and `delete`:

```rust
    pub fn put(&mut self, key: String, value: Vec<u8>) -> io::Result<()> {
        // ... existing body ...
        self.index.insert(key, offset);
        self.refresh_mmap()?;  // NEW
        Ok(())
    }

    pub fn delete(&mut self, key: &str) -> io::Result<bool> {
        // ... existing body ...
        self.index.remove(key);
        self.refresh_mmap()?;  // NEW
        Ok(true)
    }
```

Now add `get_mmap`:

```rust
    pub fn get_mmap(&self, key: &str) -> io::Result<Option<Vec<u8>>> {
        let offset = match self.index.get(key) {
            Some(&o) => o,
            None => return Ok(None),
        };

        let mmap = match &self.mmap {
            Some(m) => m,
            None => return Ok(None),
        };

        let (bytes, _) = crate::format::read_record_slice(mmap, offset as usize)?
            .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "record missing"))?;

        let entry: LogEntry = bincode::deserialize(&bytes)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        match entry.value {
            LogValue::Data(v) => Ok(Some(v)),
            LogValue::Tombstone => Ok(None),
        }
    }
```

Key change: instead of `seek` + `read` on a `File`, we slice directly into the `Mmap`. No syscalls on the hot path.

Another subtle improvement: `get_mmap` takes `&self` (immutable), while `get` takes `&mut self` (because `BufReader::seek` needs mutability). That means `get_mmap` can be shared across threads trivially — `&KvStore` is easier to share than `&mut KvStore`. We'll exploit this on Day 21.

## Step 4 — Tests that exercise both paths

```rust
#[test]
fn get_and_get_mmap_agree() {
    let path = temp_path("mmap_agree");
    std::fs::remove_file(&path).ok();

    let mut store = KvStore::open(&path).unwrap();
    for i in 0..100 {
        store.put(format!("k{:03}", i), format!("v{}", i).into_bytes()).unwrap();
    }

    for i in 0..100 {
        let k = format!("k{:03}", i);
        let via_seek = store.get(&k).unwrap();
        let via_mmap = store.get_mmap(&k).unwrap();
        assert_eq!(via_seek, via_mmap, "mismatch for key {}", k);
    }

    // Also test missing key
    assert_eq!(store.get("nope").unwrap(), None);
    assert_eq!(store.get_mmap("nope").unwrap(), None);

    std::fs::remove_file(&path).ok();
}

#[test]
fn get_mmap_after_delete() {
    let path = temp_path("mmap_delete");
    std::fs::remove_file(&path).ok();

    let mut store = KvStore::open(&path).unwrap();
    store.put("a".into(), b"alpha".to_vec()).unwrap();
    store.put("b".into(), b"bravo".to_vec()).unwrap();
    store.delete("a").unwrap();

    assert_eq!(store.get_mmap("a").unwrap(), None);
    assert_eq!(store.get_mmap("b").unwrap(), Some(b"bravo".to_vec()));

    std::fs::remove_file(&path).ok();
}
```

Run:

```sh
cargo test store::tests
```

All existing tests should still pass, plus the two new ones.

## Step 5 — The criterion benchmark

Create `benches/kv_reads.rs`:

```rust
use criterion::{criterion_group, criterion_main, Criterion};
use rkvs::store::KvStore;
use std::path::PathBuf;

fn setup_store(n: usize) -> (KvStore, PathBuf) {
    let path = std::env::temp_dir().join(format!("rkvs_bench_{}.rkvs", std::process::id()));
    let _ = std::fs::remove_file(&path);

    let mut store = KvStore::open(&path).unwrap();
    for i in 0..n {
        let key = format!("k{:08}", i);
        let value = vec![0xAB; 128];
        store.put(key, value).unwrap();
    }
    (store, path)
}

fn bench_reads(c: &mut Criterion) {
    const N: usize = 10_000;
    let (mut store, path) = setup_store(N);

    let mut group = c.benchmark_group("read_paths");

    group.bench_function("seek_based", |b| {
        let mut i: usize = 0;
        b.iter(|| {
            let key = format!("k{:08}", i % N);
            let _ = store.get(&key).unwrap();
            i = i.wrapping_add(1);
        });
    });

    group.bench_function("mmap_based", |b| {
        let mut i: usize = 0;
        b.iter(|| {
            let key = format!("k{:08}", i % N);
            let _ = store.get_mmap(&key).unwrap();
            i = i.wrapping_add(1);
        });
    });

    group.finish();

    std::fs::remove_file(&path).ok();
}

criterion_group!(benches, bench_reads);
criterion_main!(benches);
```

For this to work, `KvStore` and the `store` module must be public from a library crate. Add `src/lib.rs`:

```rust
pub mod format;
pub mod save;
pub mod store;
pub mod btree;
```

Now `main.rs` can stay as-is (it'll use these as local modules) or switch to `use rkvs::...`. The cleanest: keep `main.rs` using `mod store;` locally AND add `lib.rs` so the benchmark can import via the package name. Rust allows both.

Run:

```sh
cargo bench
```

First run compiles; subsequent runs execute benchmarks. Output looks like:

```
read_paths/seek_based   time:   [2.8431 µs 2.8519 µs 2.8618 µs]
read_paths/mmap_based   time:   [812.46 ns 815.20 ns 818.23 ns]
```

Actual numbers vary; ours are typical on a Linux x86_64 laptop with the file in page cache. The mmap path is ~3.5x faster here — no syscalls per read, no userspace/kernel copy.

On a cold cache (clear it with `echo 3 | sudo tee /proc/sys/vm/drop_caches` on Linux), the difference shrinks because both paths have to go to disk. The mmap path still wins because page faults go through highly optimized kernel paths — but by much less. The big mmap win is for hot working sets.

## Step 6 — Viewing mmap behavior with `strace`

On Linux, watch the syscalls:

```sh
strace -e trace=read,pread64,mmap,munmap ./target/release/rkvs get somekey 2>&1 | head -20
```

You'll see `mmap` called once (during `open`), then zero `read` or `pread64` calls for the get (if the mapping is still warm). The value comes from the memory map without any syscall.

Compare to the seek-based path (force it by commenting out the mmap in `main.rs`):

```
read(3, ...) = 8
read(3, ...) = 8
read(3, ...) = 128
```

Three syscalls per get: header, length+CRC, payload. At ~200 ns per syscall, that's the ~2-3 µs overhead we measured.

## Common pitfalls

### Forgetting to refresh the mmap after writes

If you `put` a key and then `get_mmap` it without `refresh_mmap`, the new record is past the old mmap's end — you get `UnexpectedEof`. Refresh after every write (what we did), or on a clock (every N writes), or hybrid.

This is a major reason many databases use mmap for *reads only* and a separate write path. The mmap is refreshed periodically or at checkpoint boundaries.

### Writing through the mmap

```rust
use memmap2::MmapMut;

let mut mmap = unsafe { MmapMut::map_mut(&file)? };
mmap[0] = 42;  // modifies the file!
mmap.flush()?;  // force writeback
```

Possible, but don't. Crash recovery becomes very hard because you don't know which pages were flushed. Databases that do write through mmap (like LMDB) have sophisticated transaction logic to handle this. For our purposes, mmap for reads, regular writes for appends.

### Mmap on Windows

mmap works on Windows too (memmap2 wraps `MapViewOfFile`), but semantics differ slightly. Shared access may be stricter. Our simple read-only use case works on all three major OSes.

### Tiny files

A 100-byte file isn't worth mapping. The mmap setup cost (a syscall and a page-table entry) outweighs the savings. Real KV stores typically don't mmap below a few KB.

### `unsafe { Mmap::map(&file) }` without the `SAFETY:` comment

Don't do this. The single most important thing about `unsafe` in Rust is that you *justify* every use. Without the comment:

- Code reviewers can't easily approve it.
- You might reuse the pattern in a context where the invariants don't hold, introducing undefined behavior.
- Tooling like `clippy::undocumented_unsafe_blocks` will complain.

The comment is not ceremony. It's the *whole point*.

### Forgetting `harness = false` in `Cargo.toml`

```toml
[[bench]]
name = "kv_reads"
harness = false  # REQUIRED — tells Cargo to use criterion's runner, not the unstable libtest bench
```

Without this, `cargo bench` tries to run the unstable libtest bench harness and fails on stable Rust.

## What you learned

- **mmap** maps a file into process address space — reads become memory dereferences.
- **Page cache hits are free** — no syscall, no copy.
- **`unsafe`** in Rust ≠ "no safety checks" — it's "compiler trusts you on specific invariants."
- **Every `unsafe` block gets a `SAFETY:` comment** explaining why it's sound.
- The **`memmap2`** crate's `Mmap::map` is unsafe only because the caller must ensure no concurrent external modification.
- Memory-mapped reads of hot data are typically 2-5x faster than seek+read.
- Mmap is **read-heavy territory** — writes through mmap are possible but complicated.
- **Criterion** is the standard microbenchmark crate. `harness = false` is required in Cargo.toml.
- `&self` methods (like `get_mmap`) are easier to share across threads than `&mut self`.

## Exercises

1. **Zero-copy get.** Replace `get_mmap` with `fn get_mmap_ref(&self, key: &str) -> Option<&[u8]>` — return a slice into the mmap, no `Vec<u8>` allocation. You'll need to rework the deserialization path to avoid the `bincode::deserialize(&bytes).value` copy. Hint: parse the `LogEntry` in place. Benchmark against `get_mmap`.
2. **Windowed mmap.** For very large files (> 1 GB), map in 256 MB windows and remap as needed. Implement a cache of at most 4 live mmaps.
3. **Prefault.** Add `store.prefault()` that reads through the entire mmap once to pre-populate the page cache. Benchmark the first 1000 reads with and without prefault.
4. **Miri.** Run `cargo +nightly miri test` on the store tests. Miri is an experimental interpreter that catches undefined behavior. Does our one `unsafe` block survive? (Miri doesn't actually execute `mmap`, so this might not tell you much — but it catches other unsafe issues.)
5. **Prefetch hint.** On Linux, call `libc::madvise(ptr, len, MADV_RANDOM)` on the mmap to tell the kernel the access pattern is random (no read-ahead). For sequential scans, use `MADV_SEQUENTIAL`. Benchmark a scan workload with each.

## What's next

Day 20 tackles the thing we've been hand-waving for three days: **durability**. What actually happens on a power cut? You'll write a **write-ahead log** with explicit `fsync` calls, crash-test the store by forcibly killing a child process mid-write, and implement replay-based recovery. This is where Bitcask earns its reliability reputation.

→ [Day 20 — Durability, WALs, and Crash Recovery](day-20.md)
