# Day 21 — Concurrent KV Store: RwLock, Sharding, and Amdahl

**Domain:** databases • **Time:** 2 hours • **Difficulty:** hard

## What you'll build

Two concurrent wrappers around the `KvStore` from Days 17-20. First, an `Arc<RwLock<KvStore>>` that allows many simultaneous readers or a single writer. Then a **sharded store** that partitions keys across N independent stores, so writes can proceed in parallel as long as they target different shards. You'll stress-test both with a worker-pool harness, compare throughput curves as you scale from 1 to 16 threads, and see Amdahl's law up close.

## What you'll learn

- `Arc<T>` vs. `Arc<Mutex<T>>` vs. `Arc<RwLock<T>>` — when to reach for which
- Reader-writer locks and their surprising pitfalls (writer starvation, upgrade deadlocks)
- **Sharding** as a scalability technique — splitting one lock into N locks
- Choosing a shard key: hash vs. range vs. consistent hashing (preview)
- The `parking_lot` crate — faster, poison-free replacement for `std::sync`
- Benchmarking concurrent code honestly (barriers, warmup, jitter)
- Amdahl's law: why 16 threads rarely gets you 16x

## Background

### Why the old Week 2 tools aren't enough

On Day 14 you used `Arc<Mutex<T>>` to share state between threads. That works but it's pessimistic: every access, read or write, takes the exclusive lock. For a read-mostly workload (which most KV workloads are — 90%+ reads is typical for caches), that's leaving huge performance on the table.

Two upgrades exist:

1. **`RwLock`**: many readers OR one writer. Reads don't block each other. A single shared bottleneck though.
2. **Sharding**: replace one lock with N locks, each protecting 1/N of the data. Different shards don't contend at all.

You usually want both. Start simple (RwLock) and shard only if profiling shows lock contention.

### `RwLock` semantics

```rust
use std::sync::RwLock;

let lock = RwLock::new(42);

// Read: acquires a shared guard. Many simultaneous readers are allowed.
let r = lock.read().unwrap();
println!("{}", *r);
drop(r);

// Write: acquires an exclusive guard. Blocks until all readers are done.
let mut w = lock.write().unwrap();
*w += 1;
```

The `.read()` and `.write()` methods return `LockResult<Guard>` — they can fail if another thread panicked while holding the lock (poisoning). For simple stores we `unwrap()`; production code often switches to `parking_lot` which doesn't poison.

### The subtle pitfalls of RwLock

**Writer starvation.** In some implementations, if readers keep arriving, a waiting writer never gets in. `std::sync::RwLock` on most platforms is fair enough, but it's worth knowing. If your reads take a long time and a writer blocks, your writes are stuck.

**Upgrade deadlocks.** You hold a read guard, then want to write. In Rust's `std::sync::RwLock`, you must drop the read guard first, then re-acquire as write. Between those two steps, the world may change — you need to re-check your assumptions. `parking_lot` has `upgradable_read` for atomic upgrade, but we won't need it.

**Reader re-entry deadlocks.** `std::sync::RwLock` does *not* guarantee that a thread holding a read lock can acquire another read lock. On some platforms it works; on others it deadlocks. Don't rely on re-entry. If you need it, use `parking_lot` with `ReentrantMutex`.

### Sharding intuition

Imagine 100 million keys and a single `Mutex<KvStore>`. Every put takes the lock. 32 cores all stall on one cache line. Throughput tops out around 1M ops/sec no matter how wide your hardware is.

Now chop the key space into 64 shards, each its own `Mutex<KvStore>`. A put on shard 3 doesn't touch shards 0-2 or 4-63. If 64 threads each hit a different shard, there's *zero* contention — linear scaling.

Sharding by hash is simple: `shard_id = hash(key) % N`. If the hash is good, keys spread evenly. Bad hash → "hot" shards that bottleneck again.

Sharding by range is harder but allows range scans within a shard. Most production KV stores (e.g., TiKV, CockroachDB) shard by range for scan locality.

**Rule of thumb**: 2-8x the number of cores for hash sharding. Too few shards → contention; too many → memory overhead per shard (each shard has its own hashmap, its own file handle, etc.).

### `parking_lot`

The `parking_lot` crate reimplements `Mutex`, `RwLock`, `Condvar`, and `Once` with better performance and a saner API:

- No poisoning (a panic while holding the lock doesn't disable the lock forever).
- No `LockResult` — `.lock()` returns the guard directly.
- Smaller (1 byte for `Mutex` vs. ~40 bytes for `std::sync::Mutex` on Linux).
- Faster on contended workloads.

Today we'll start with `std::sync::RwLock` (always available), then swap in `parking_lot::RwLock` at the end and measure the difference.

### Amdahl's law in one sentence

If a fraction *p* of your work is parallelizable and *1-p* is serial, then with N threads your max speedup is `1 / ((1-p) + p/N)`. Even 5% serial work caps you at 20x speedup — forever, no matter how many threads you throw at it.

For a sharded KV store, the "serial" part includes: the hash computation, the shard lookup (atomic index into an array — nearly free), and any global coordination. Keep that bit tiny.

## Setting up

We're still in the `rkvs` project from Days 17-20. Add two new deps:

```bash
cd rkvs
cargo add parking_lot
cargo add --dev rand
```

Your `Cargo.toml` dependencies section should now look roughly like:

```toml
[dependencies]
crc32fast = "1"
serde = { version = "1", features = ["derive"] }
bincode = "1"
clap = { version = "4", features = ["derive"] }
memmap2 = "0.9"
parking_lot = "0.12"

[dev-dependencies]
proptest = "1"
criterion = "0.5"
rand = "0.8"

[[bench]]
name = "kv_reads"
harness = false
```

Create a fresh module today:

```bash
touch src/concurrent.rs
```

And register it in `src/lib.rs`:

```rust
pub mod btree;
pub mod format;
pub mod save;
pub mod store;
pub mod concurrent;  // NEW
```

## Step 1 — The RwLock wrapper

Start with the simplest possible concurrent wrapper. Open `src/concurrent.rs`:

```rust
use crate::store::{KvStore, StoreError};
use std::path::Path;
use std::sync::{Arc, RwLock};

/// A thread-safe KV store allowing many concurrent readers or one writer.
#[derive(Clone)]
pub struct SharedStore {
    inner: Arc<RwLock<KvStore>>,
}

impl SharedStore {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, StoreError> {
        let store = KvStore::open(path)?;
        Ok(Self {
            inner: Arc::new(RwLock::new(store)),
        })
    }

    pub fn get(&self, key: &str) -> Result<Option<Vec<u8>>, StoreError> {
        let guard = self.inner.read().expect("poisoned");
        guard.get(key)
    }

    pub fn put(&self, key: &str, value: &[u8]) -> Result<(), StoreError> {
        let mut guard = self.inner.write().expect("poisoned");
        guard.put(key, value)
    }

    pub fn delete(&self, key: &str) -> Result<(), StoreError> {
        let mut guard = self.inner.write().expect("poisoned");
        guard.delete(key)
    }
}
```

A few points:

- The wrapper is `#[derive(Clone)]`. Cloning just bumps the `Arc` refcount — every clone points at the same underlying store. This is how you share a handle across threads: clone and move into each thread.
- `get` takes `&self` (not `&mut self`). That's deliberate — it's what lets you clone `SharedStore` around and still call `get` from anywhere.
- `put` also takes `&self`, because the `RwLock` provides interior mutability. Callers don't need `&mut SharedStore`.
- We use `.expect("poisoned")` for brevity. In production you'd decide what to do: abort, log and ignore, or switch to `parking_lot` to avoid poisoning entirely.

### Recap: `Arc` vs. `Rc`

`Rc<T>` is a single-threaded reference count. `Arc<T>` is atomically refcounted and safe to share between threads. Both let multiple owners read the inner value. Neither by itself allows mutation — you need interior mutability (`Mutex`, `RwLock`, `RefCell`, `Cell`) for that.

`Arc<Mutex<T>>` — one writer at a time, readers block writers.
`Arc<RwLock<T>>` — many readers, one writer.
`Arc<T>` alone — immutable shared data (no locks, no mutation).

Python comparison: all of these are what Python's GIL gives you for free — one thread executing Python bytecode at a time. Rust makes you pick the granularity, which is tedious but wins when you need real parallelism.

## Step 2 — Prove it compiles across threads

Create a quick manual test at the bottom of `src/concurrent.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use tempfile::tempdir;

    #[test]
    fn rwlock_concurrent_reads() {
        let dir = tempdir().unwrap();
        let store = SharedStore::open(dir.path().join("data.log")).unwrap();

        store.put("greeting", b"hello").unwrap();

        let mut handles = vec![];
        for _ in 0..8 {
            let store = store.clone();
            handles.push(thread::spawn(move || {
                for _ in 0..1000 {
                    let v = store.get("greeting").unwrap();
                    assert_eq!(v.as_deref(), Some(&b"hello"[..]));
                }
            }));
        }
        for h in handles {
            h.join().unwrap();
        }
    }
}
```

Add `tempfile` to dev-deps if it's not already there:

```bash
cargo add --dev tempfile
```

Run:

```bash
cargo test --lib rwlock_concurrent_reads
```

Expected output:

```
running 1 test
test concurrent::tests::rwlock_concurrent_reads ... ok

test result: ok. 1 passed; 0 failed
```

Eight threads, 8000 total reads of the same key, all correct. The `RwLock` serves them all concurrently as shared reads.

## Step 3 — Stress-test mixed reads and writes

A single-key test doesn't exercise contention. Let's simulate a realistic workload:

```rust
    #[test]
    fn rwlock_mixed_workload() {
        let dir = tempdir().unwrap();
        let store = SharedStore::open(dir.path().join("data.log")).unwrap();

        // Pre-load some keys.
        for i in 0..100 {
            let k = format!("key{i:04}");
            store.put(&k, format!("value{i}").as_bytes()).unwrap();
        }

        let mut handles = vec![];

        // 7 reader threads.
        for t in 0..7 {
            let store = store.clone();
            handles.push(thread::spawn(move || {
                for i in 0..1000 {
                    let k = format!("key{:04}", (t * 1000 + i) % 100);
                    let _ = store.get(&k).unwrap();
                }
            }));
        }

        // 1 writer thread.
        {
            let store = store.clone();
            handles.push(thread::spawn(move || {
                for i in 0..500 {
                    let k = format!("key{:04}", i % 100);
                    store.put(&k, format!("updated{i}").as_bytes()).unwrap();
                }
            }));
        }

        for h in handles {
            h.join().unwrap();
        }

        // Check at least the last-written version is visible.
        let v = store.get("key0000").unwrap();
        assert!(v.is_some());
    }
```

Run `cargo test --lib rwlock_mixed_workload`. It should finish in under a second. If you see a deadlock or panic, your worker code has a bug — typical culprits are guards held across thread-spawn points or a recursive `read()` while holding a `write()`.

## Step 4 — Benchmark the RwLock store

Let's quantify. Add a new benchmark file at `benches/concurrent.rs`:

```rust
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use rand::{rngs::StdRng, Rng, SeedableRng};
use rkvs::concurrent::SharedStore;
use std::sync::Arc;
use std::sync::Barrier;
use std::thread;
use tempfile::tempdir;

fn bench_rwlock_reads(c: &mut Criterion) {
    let dir = tempdir().unwrap();
    let store = SharedStore::open(dir.path().join("data.log")).unwrap();

    // Pre-load 10k keys.
    for i in 0..10_000 {
        let k = format!("key{i:06}");
        store.put(&k, format!("value{i}").as_bytes()).unwrap();
    }

    let mut group = c.benchmark_group("rwlock_reads");

    for &n_threads in &[1, 2, 4, 8] {
        group.bench_function(BenchmarkId::from_parameter(n_threads), |b| {
            b.iter(|| {
                let barrier = Arc::new(Barrier::new(n_threads));
                let mut handles = vec![];
                for t in 0..n_threads {
                    let store = store.clone();
                    let barrier = barrier.clone();
                    handles.push(thread::spawn(move || {
                        let mut rng = StdRng::seed_from_u64(t as u64);
                        barrier.wait();
                        for _ in 0..1000 {
                            let k = format!("key{:06}", rng.gen_range(0..10_000));
                            let _ = black_box(store.get(&k).unwrap());
                        }
                    }));
                }
                for h in handles {
                    h.join().unwrap();
                }
            });
        });
    }

    group.finish();
}

criterion_group!(benches, bench_rwlock_reads);
criterion_main!(benches);
```

Add the bench to `Cargo.toml`:

```toml
[[bench]]
name = "concurrent"
harness = false
```

Then run:

```bash
cargo bench --bench concurrent
```

On an 8-core laptop you'll see output like:

```
rwlock_reads/1          time:   [1.2 ms 1.2 ms 1.3 ms]
rwlock_reads/2          time:   [1.3 ms 1.4 ms 1.4 ms]
rwlock_reads/4          time:   [1.9 ms 2.0 ms 2.1 ms]
rwlock_reads/8          time:   [3.4 ms 3.6 ms 3.7 ms]
```

Wait — going *up*? At 8 threads we do 8000 reads vs. 1000 at 1 thread, so per-read cost went from 1.2 µs to 450 ns. That's *good* — more work in less time per op. Divide to see throughput: thread-1 is 833k reads/sec, thread-8 is 2.2M reads/sec. We got 2.6x from 8x threads.

Why so sub-linear? RwLock bookkeeping itself contends. Every reader increments a shared counter; writers wait for it to hit zero. Even when there's no writer, the atomic counter bounces cache lines between cores.

### The key insight

`RwLock` solves the *mutual exclusion* problem but not the *cache line bouncing* problem. For serious scaling, you need data that's physically separate — not just "logically protected separately." That's sharding.

## Step 5 — Design the sharded store

We want a struct that, given a key, picks one of N shards deterministically and delegates to that shard's `KvStore`.

Key decisions:

- **Fixed N at construction time.** Resharding at runtime is a whole research area; skip it.
- **Hash function.** `std::collections::hash_map::DefaultHasher` (SipHash 1-3). Deterministic *within a program run* but not across runs with different hash seeds by default. For deterministic sharding we'll use a stable seeded hasher.
- **Each shard gets its own file.** `shard_000.log`, `shard_001.log`, etc. The original `KvStore::open` already works per-file — no changes to Days 17-20 code required.
- **Mutex per shard, not RwLock.** Reads are still fast inside a shard (no cross-shard blocking); we optimize for write-heavy or mixed workloads. Swap to `RwLock` if profiling says so.

### A stable hash

Add this to `src/concurrent.rs`:

```rust
use std::hash::{BuildHasher, BuildHasherDefault, Hasher};
use std::collections::hash_map::DefaultHasher;

fn shard_of(key: &str, n_shards: usize) -> usize {
    let builder = BuildHasherDefault::<DefaultHasher>::default();
    let mut hasher = builder.build_hasher();
    hasher.write(key.as_bytes());
    (hasher.finish() as usize) % n_shards
}
```

`BuildHasherDefault<DefaultHasher>` uses zero-seeded SipHash, so it's deterministic across processes — essential, because the shard assignment must survive a restart.

Verify it's stable:

```rust
    #[test]
    fn hash_is_stable() {
        assert_eq!(shard_of("hello", 16), shard_of("hello", 16));
        // These exact values depend on SipHash internals but won't drift.
        let s = shard_of("hello", 16);
        assert!(s < 16);
    }
```

### The sharded struct

```rust
use parking_lot::Mutex;
use std::fs;

pub struct ShardedStore {
    shards: Vec<Arc<Mutex<KvStore>>>,
}

impl ShardedStore {
    pub fn open(dir: impl AsRef<Path>, n_shards: usize) -> Result<Self, StoreError> {
        let dir = dir.as_ref();
        fs::create_dir_all(dir).map_err(StoreError::Io)?;

        let mut shards = Vec::with_capacity(n_shards);
        for i in 0..n_shards {
            let path = dir.join(format!("shard_{i:03}.log"));
            let store = KvStore::open(path)?;
            shards.push(Arc::new(Mutex::new(store)));
        }

        Ok(Self { shards })
    }

    fn shard_for(&self, key: &str) -> &Arc<Mutex<KvStore>> {
        &self.shards[shard_of(key, self.shards.len())]
    }

    pub fn get(&self, key: &str) -> Result<Option<Vec<u8>>, StoreError> {
        let shard = self.shard_for(key);
        let guard = shard.lock();
        guard.get(key)
    }

    pub fn put(&self, key: &str, value: &[u8]) -> Result<(), StoreError> {
        let shard = self.shard_for(key);
        let mut guard = shard.lock();
        guard.put(key, value)
    }

    pub fn delete(&self, key: &str) -> Result<(), StoreError> {
        let shard = self.shard_for(key);
        let mut guard = shard.lock();
        guard.delete(key)
    }
}
```

Notice:

- The struct is **not** `Clone`. Callers share it via `Arc<ShardedStore>`. Cloning the `Vec` of `Arc<Mutex<_>>` would be pointless since the `Arc`s inside are what's shared.
- `parking_lot::Mutex::lock()` returns the guard directly — no `.unwrap()` for poison handling.
- Shard access is a single `Vec` index (effectively a pointer arithmetic op). Near-zero overhead.

### Check you can share it

Functions that want to spawn threads will take `Arc<ShardedStore>`:

```rust
let store = Arc::new(ShardedStore::open("mydb", 16)?);
// ...
let s = store.clone();  // Arc clone — bumps refcount
thread::spawn(move || {
    s.put("hello", b"world").unwrap();
});
```

If you need to verify, add the test:

```rust
    #[test]
    fn sharded_basic() {
        let dir = tempdir().unwrap();
        let store = ShardedStore::open(dir.path(), 16).unwrap();

        for i in 0..100 {
            let k = format!("key{i:04}");
            store.put(&k, format!("v{i}").as_bytes()).unwrap();
        }

        for i in 0..100 {
            let k = format!("key{i:04}");
            let v = store.get(&k).unwrap();
            assert_eq!(v.as_deref(), Some(format!("v{i}").as_bytes()));
        }
    }
```

Run `cargo test --lib sharded_basic`. Should pass and create a directory with `shard_000.log` through `shard_015.log`.

## Step 6 — Stress test the sharded store

Now the interesting one. Let's see if sharding actually scales writes:

```rust
    #[test]
    fn sharded_concurrent_writes() {
        let dir = tempdir().unwrap();
        let store = Arc::new(ShardedStore::open(dir.path(), 16).unwrap());

        let mut handles = vec![];
        for t in 0..8 {
            let store = store.clone();
            handles.push(thread::spawn(move || {
                for i in 0..500 {
                    let k = format!("t{t}_key{i:04}");
                    store.put(&k, format!("v{i}").as_bytes()).unwrap();
                }
            }));
        }
        for h in handles {
            h.join().unwrap();
        }

        // Verify all 4000 keys are present.
        for t in 0..8 {
            for i in 0..500 {
                let k = format!("t{t}_key{i:04}");
                let v = store.get(&k).unwrap();
                assert_eq!(v.as_deref(), Some(format!("v{i}").as_bytes()));
            }
        }
    }
```

Run:

```bash
cargo test --lib sharded_concurrent_writes --release
```

The `--release` matters here — `parking_lot::Mutex` is much faster in release mode because its lock loop gets properly inlined.

## Step 7 — Benchmark sharding vs. RwLock

Extend `benches/concurrent.rs`:

```rust
use rkvs::concurrent::ShardedStore;

fn bench_sharded_writes(c: &mut Criterion) {
    let dir = tempdir().unwrap();
    let store = Arc::new(ShardedStore::open(dir.path(), 16).unwrap());

    // Pre-load.
    for i in 0..10_000 {
        let k = format!("key{i:06}");
        store.put(&k, format!("value{i}").as_bytes()).unwrap();
    }

    let mut group = c.benchmark_group("sharded_writes");

    for &n_threads in &[1, 2, 4, 8] {
        group.bench_function(BenchmarkId::from_parameter(n_threads), |b| {
            b.iter(|| {
                let barrier = Arc::new(Barrier::new(n_threads));
                let mut handles = vec![];
                for t in 0..n_threads {
                    let store = store.clone();
                    let barrier = barrier.clone();
                    handles.push(thread::spawn(move || {
                        let mut rng = StdRng::seed_from_u64(t as u64);
                        barrier.wait();
                        for _ in 0..200 {
                            let k = format!("key{:06}", rng.gen_range(0..10_000));
                            store.put(&k, b"x").unwrap();
                        }
                    }));
                }
                for h in handles {
                    h.join().unwrap();
                }
            });
        });
    }

    group.finish();
}

// Also add a RwLock write benchmark for comparison:
fn bench_rwlock_writes(c: &mut Criterion) {
    let dir = tempdir().unwrap();
    let store = SharedStore::open(dir.path().join("data.log")).unwrap();

    for i in 0..10_000 {
        let k = format!("key{i:06}");
        store.put(&k, format!("value{i}").as_bytes()).unwrap();
    }

    let mut group = c.benchmark_group("rwlock_writes");

    for &n_threads in &[1, 2, 4, 8] {
        group.bench_function(BenchmarkId::from_parameter(n_threads), |b| {
            b.iter(|| {
                let barrier = Arc::new(Barrier::new(n_threads));
                let mut handles = vec![];
                for t in 0..n_threads {
                    let store = store.clone();
                    let barrier = barrier.clone();
                    handles.push(thread::spawn(move || {
                        let mut rng = StdRng::seed_from_u64(t as u64);
                        barrier.wait();
                        for _ in 0..200 {
                            let k = format!("key{:06}", rng.gen_range(0..10_000));
                            store.put(&k, b"x").unwrap();
                        }
                    }));
                }
                for h in handles {
                    h.join().unwrap();
                }
            });
        });
    }

    group.finish();
}

criterion_group!(benches, bench_rwlock_reads, bench_rwlock_writes, bench_sharded_writes);
```

Run:

```bash
cargo bench --bench concurrent
```

Typical output on an 8-core laptop:

```
rwlock_writes/1         time:   [820 µs  840 µs  870 µs]
rwlock_writes/2         time:   [1.6 ms  1.7 ms  1.7 ms]
rwlock_writes/4         time:   [3.3 ms  3.4 ms  3.5 ms]
rwlock_writes/8         time:   [6.6 ms  6.8 ms  7.1 ms]

sharded_writes/1        time:   [850 µs  880 µs  910 µs]
sharded_writes/2        time:   [950 µs  1.0 ms  1.1 ms]
sharded_writes/4        time:   [1.1 ms  1.2 ms  1.3 ms]
sharded_writes/8        time:   [1.5 ms  1.6 ms  1.8 ms]
```

Read those columns carefully. For RwLock writes, throughput is *flat* as threads increase — each thread does 200 writes, total work scales 8x, time scales 8x. That's zero parallelism on writes, exactly as expected: `RwLock` serializes all writers.

For sharded writes, 8 threads is only 1.8x slower than 1 thread for 8x the work. That's 4.5x speedup on 8 threads — a huge improvement, limited only by how often two threads happen to hit the same shard (birthday-paradox-like collisions, plus fsync cost).

## Step 8 — Swap in `parking_lot::RwLock`

We used `parking_lot::Mutex` for `ShardedStore`. Let's upgrade `SharedStore` too. Change the imports and type in `src/concurrent.rs`:

```rust
use parking_lot::RwLock;
// Remove: use std::sync::RwLock;
```

And change the methods. `parking_lot::RwLock::read()` returns a guard directly (no `Result`):

```rust
impl SharedStore {
    pub fn get(&self, key: &str) -> Result<Option<Vec<u8>>, StoreError> {
        let guard = self.inner.read();  // no .expect("poisoned")
        guard.get(key)
    }

    pub fn put(&self, key: &str, value: &[u8]) -> Result<(), StoreError> {
        let mut guard = self.inner.write();
        guard.put(key, value)
    }

    pub fn delete(&self, key: &str) -> Result<(), StoreError> {
        let mut guard = self.inner.write();
        guard.delete(key)
    }
}
```

Re-run `cargo bench --bench concurrent`. You'll typically see a 10-30% improvement on contended reads and 2-3x on contended writes, because `parking_lot` uses a faster locking algorithm (word-sized parking queue, adaptive spinning) than the OS-delegated approach `std::sync` uses.

## Step 9 — Deadlock safety review

Three mental checks for any multi-threaded Rust code:

1. **Single-lock code**: always fine.
2. **Multi-lock code**: always acquire locks in the same order on every code path. (For `ShardedStore`, any operation involving two keys across two shards must always lock the shards in ascending order of shard index.)
3. **Guards crossing await/yield points**: if you ever add async, don't hold a `std::sync` guard across `.await`. The runtime may move your task to another thread, and guards don't move. Use `tokio::sync::Mutex` instead.

We don't need multi-key atomic ops today, but it's a common extension. If you do: collect the unique shard indices, sort them, lock in order:

```rust
fn put_two(&self, k1: &str, v1: &[u8], k2: &str, v2: &[u8]) -> Result<(), StoreError> {
    let i1 = shard_of(k1, self.shards.len());
    let i2 = shard_of(k2, self.shards.len());
    let (lo, hi) = if i1 <= i2 { (i1, i2) } else { (i2, i1) };

    let mut g_lo = self.shards[lo].lock();
    let _g_hi = if lo == hi { None } else { Some(self.shards[hi].lock()) };
    // ...
    Ok(())
}
```

Never `lock(i); lock(j);` where the order depends on the input — that's how deadlocks happen.

## Common pitfalls

### Trying to clone `KvStore` directly

```rust
let store = KvStore::open("db.log")?;
thread::spawn(move || {
    let s = store.clone();  // error: KvStore doesn't implement Clone
});
```

`KvStore` holds file handles — you can't just clone it. Either wrap in `Arc<Mutex<_>>` (what `SharedStore` does) or reopen the file in each thread (what `ShardedStore` effectively does, by giving each shard its own `KvStore`).

### Holding a read guard while taking write

```rust
let r = store.inner.read();
if r.get("key")?.is_none() {
    drop(r);  // ← DO NOT forget this
    store.inner.write().put("key", b"value")?;
}
```

Without `drop(r)`, the `write()` call deadlocks: you hold a read guard, are asking for an exclusive write, but the read guard will outlive the write call. This is why higher-level methods (`get`, `put`) are the right abstraction — you never expose guards to callers.

### Poisoned `std::sync::RwLock`

```
thread 'worker' panicked at 'some condition'
thread 'main' panicked at 'poisoned: PoisonError { .. }'
```

If a thread panics while holding a `std::sync::RwLock` or `Mutex`, subsequent `.read()`/`.write()`/`.lock()` calls return `Err(PoisonError)`. This is `std`'s "safe by default" behavior — maybe the inner state is corrupted.

Options: (a) use `.unwrap()` everywhere and crash on poison (crude but fine for many apps), (b) use `.unwrap_or_else(|e| e.into_inner())` to ignore poisoning (you trust the invariants), (c) switch to `parking_lot`, which doesn't poison at all.

### Benchmarking in debug mode

`cargo test` defaults to debug. `cargo bench` always runs release. If you're doing ad-hoc timing with `Instant::now()` in a test, your numbers will be wildly wrong — often 10-50x slower than release — because `Mutex::lock` and friends don't get inlined.

Either use `cargo test --release`, or (better) move timing code into `cargo bench`.

### Too many shards

With 1024 shards and a small working set, you pay memory overhead (each shard has a hashmap, a file handle, its own mmap, its own index rebuild cost on startup) for no contention win. Start with `2 * num_cpus` or `16`, measure, adjust.

### Uneven key distribution

If your keys are `"user:1"`, `"user:2"`, ... `"user:N"`, and your workload is 90% on `"user:1"` (e.g., a big tenant), hashing doesn't save you — that one key lives on one shard, and that shard is hot. Consider hashing on a composite key or splitting hot tenants explicitly. This is why real systems have dynamic rebalancing, which is hard.

## What you learned

- `Arc<RwLock<T>>` lets many readers share access with one writer blocking them — good for read-heavy workloads but doesn't scale writes.
- **Sharding** replaces one global lock with N per-shard locks, letting writers run in parallel as long as they target different shards.
- Hash-based shard assignment must be **deterministic** across runs (use `BuildHasherDefault<DefaultHasher>` or a fixed-seed hasher, not `RandomState`).
- `parking_lot` is a straight-up upgrade over `std::sync`: no poisoning, faster, smaller, saner API.
- Reader-writer locks have **cache-line contention** even for pure reads — `RwLock` isn't a free lunch.
- Lock ordering matters: for multi-shard ops, always lock in a consistent (sorted) order to avoid deadlocks.
- **Amdahl's law** shapes all concurrency gains: small serial fractions cap your speedup hard. A well-designed KV workload has near-zero serial work per op.
- Always benchmark in `--release` mode; locks don't inline otherwise.

## Exercises

1. **Sharded RwLock.** Swap `Mutex` for `RwLock` in `ShardedStore`. Measure on a 90%-read workload. Which sharding-granularity wins?
2. **Dynamic N.** Add a `ShardedStore::rebalance(new_n: usize)` that redistributes keys. What's your lock strategy — do you lock all N shards? Block all reads? (There's no single right answer; production systems use "shadow writes" or consistent hashing to migrate incrementally.)
3. **Consistent hashing.** Replace `hash % N` with a consistent-hash ring (read Karger et al.'s 1997 paper). Observe: adding a new shard only moves `1/N` of keys instead of almost all of them.
4. **Per-shard fsync policy.** Let each shard have its own `SyncPolicy` from Day 20. Measure throughput when "hot" shards use `EveryN` and "cold" shards use `Always`.
5. **Lock-free reads.** Use `arc_swap` or `crossbeam::epoch` to implement `get` without any lock at all, by snapshotting the in-memory index. This is how real KV stores get to 100M+ reads/sec per node. Hard. Start by reading the `arc_swap` docs.

## What's next

That wraps up Week 3 — you've built a real single-node KV store with persistence, crash recovery, and concurrency. Week 4 is a complete change of pace: **ray tracing from scratch**. You'll build `Vec3` math with operator overloading, write a physically-based renderer over six days, parallelize it with rayon, and finish with a capstone project of your choosing.

Day 22 starts simple: a canvas, a pixel buffer, and PPM/PNG output. No rays yet — just prove you can write colored pixels to a file and look at them.

→ [Day 22 — Canvas, Pixels, and PNG Output](day-22.md)
