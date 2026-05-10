# Day 14 — Threads and Channels

**Domain:** games • **Time:** 2 hours • **Difficulty:** hard

## What you'll build

A parallel A* pathfinder. Sixteen "AI snakes" on an 80×40 board, each needing a path to a target every tick. You'll implement sequential A* first, then parallelize across OS threads using message-passing channels. Measure the speedup. Meet `Arc<Mutex<T>>` and `Arc<RwLock<T>>`. See why Rust calls its concurrency story "fearless" — the compiler catches data races at compile time, not in production at 3am.

## What you'll learn

- **OS threads** via `std::thread::spawn`
- **`Send`** and **`Sync`** marker traits — who's allowed across thread boundaries
- **Channels** (`mpsc`) for message passing between threads
- **`Arc<T>`** for shared immutable data across threads
- **`Mutex<T>`** and **`RwLock<T>`** for shared mutable data
- **A\* algorithm** — a good parallel workload
- Benchmarking parallel vs serial implementations
- `recv_timeout` for robustness against slow/panicking workers

## Background

### Rust's concurrency story

Rust's threading story is built on two marker traits:

- **`Send`** — a type is safe to *transfer* to another thread. Most types are `Send`; the exceptions (like `Rc<T>`, raw pointers) are explicit.
- **`Sync`** — a type is safe to *share* across threads via shared references. Most types are `Sync`; the exceptions (like `RefCell<T>`, `Cell<T>`) opt out.

These traits are auto-implemented by the compiler based on a type's fields. You don't usually write `unsafe impl Sync for MyType` — the compiler works it out.

The payoff: the compiler *statically* prevents data races. If you try to share a `RefCell` across threads:

```rust
let cell = RefCell::new(5);
thread::spawn(move || {
    cell.borrow_mut();        // ERROR: RefCell is not Sync
});
```

The compiler refuses. No data race, ever, in safe Rust. This is what "fearless concurrency" means.

### `std::thread::spawn`

```rust
use std::thread;

let handle = thread::spawn(|| {
    println!("hello from a thread");
    42
});

let result = handle.join().unwrap();
println!("thread returned {}", result);
```

- `spawn(closure)` creates a new OS thread and returns a `JoinHandle<T>` where `T` is the closure's return type.
- `join()` waits for the thread to finish and returns its result as `Result<T, Box<dyn Any + Send>>`. The `Err` case is a panic — `Box<dyn Any>` because Rust doesn't know what was panicked with.

### Channels

`std::sync::mpsc` — multi-producer, single-consumer. Send values from one or more sender threads, receive them on one receiver thread.

```rust
use std::sync::mpsc;
use std::thread;

let (tx, rx) = mpsc::channel::<String>();

thread::spawn(move || {
    tx.send("hello".to_string()).unwrap();
});

let msg = rx.recv().unwrap();
println!("got: {}", msg);
```

- `channel()` creates a sender and receiver.
- `tx.send(v)` — non-blocking (assuming unbounded capacity). Fails if the receiver was dropped.
- `rx.recv()` — blocks until a message arrives. Returns `Err` when all senders are dropped.
- `rx.recv_timeout(d)` — blocks up to `d`; returns `Err` on timeout or disconnect.
- You can clone `tx` to get multiple senders.

### Arc and Mutex

To share *state* (not pass messages), use `Arc<T>` — atomic reference counting, the `Rc<T>` of threads.

```rust
use std::sync::Arc;
use std::thread;

let data = Arc::new(vec![1, 2, 3]);
for i in 0..3 {
    let data = Arc::clone(&data);
    thread::spawn(move || {
        println!("thread {}: {:?}", i, data);
    });
}
```

Shared immutable reads. If the threads need to mutate, add `Mutex<T>` or `RwLock<T>`:

```rust
use std::sync::Mutex;

let counter = Arc::new(Mutex::new(0));
for _ in 0..10 {
    let c = Arc::clone(&counter);
    thread::spawn(move || {
        *c.lock().unwrap() += 1;   // lock, mutate, unlock via drop
    });
}
```

`mutex.lock()` returns `Result<MutexGuard<T>, PoisonError>`. The guard derefs to `&mut T`; dropping it unlocks. The `Result` is `Err` only if a previous holder panicked (the mutex is "poisoned"); usually just `.unwrap()` it.

`RwLock` is similar but allows many concurrent readers or one writer.

### Python comparison

In Python, the GIL means only one thread runs Python code at a time — threads are good for I/O parallelism, not CPU. In Rust, threads are real OS threads with real parallelism, all statically checked for safety.

## Setting up

```bash
cargo new day-14
cd day-14
cargo add rand@0.8
```

## Step 1 — A simple thread demo

`src/main.rs`:

```rust
use std::thread;
use std::time::Duration;

fn main() {
    let mut handles = vec![];

    for i in 0..4 {
        let h = thread::spawn(move || {
            println!("thread {} starting", i);
            thread::sleep(Duration::from_millis(100));
            println!("thread {} done", i);
            i * 10
        });
        handles.push(h);
    }

    for h in handles {
        let result = h.join().unwrap();
        println!("got {}", result);
    }
}
```

Run:

```bash
cargo run
```

Output (order may vary for the starts/dones, not for "got"):

```
thread 0 starting
thread 1 starting
thread 2 starting
thread 3 starting
thread 0 done
thread 1 done
thread 2 done
thread 3 done
got 0
got 10
got 20
got 30
```

The four threads run concurrently; the `join` loop preserves the handle order.

## Step 2 — Channels

Producer-consumer pattern:

```rust
use std::sync::mpsc;
use std::thread;

fn main() {
    let (tx, rx) = mpsc::channel::<u32>();

    // Four producer threads
    for i in 0..4 {
        let tx = tx.clone();
        thread::spawn(move || {
            for j in 0..3 {
                tx.send(i * 100 + j).unwrap();
            }
        });
    }

    // Drop the original tx so the channel closes when all clones finish
    drop(tx);

    // Receive until the channel is closed
    for msg in rx {
        println!("got: {}", msg);
    }
}
```

12 messages from 4 threads, received in a single loop. Order non-deterministic within what each sender produced.

### Why `drop(tx)`?

`rx` stops iterating when the channel closes, which happens when all senders drop. If we forget to `drop(tx)`, the receiver waits forever — the original `tx` is still alive in `main`.

### `recv_timeout`

```rust
use std::time::Duration;

match rx.recv_timeout(Duration::from_millis(50)) {
    Ok(msg) => println!("got {}", msg),
    Err(mpsc::RecvTimeoutError::Timeout) => println!("no message yet"),
    Err(mpsc::RecvTimeoutError::Disconnected) => println!("done"),
}
```

Essential for robust systems — if a worker panics or hangs, the main loop doesn't hang forever.

## Step 3 — Shared state with Arc+Mutex

```rust
use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    let counter = Arc::new(Mutex::new(0));
    let mut handles = vec![];

    for _ in 0..10 {
        let c = Arc::clone(&counter);
        let h = thread::spawn(move || {
            for _ in 0..100 {
                let mut guard = c.lock().unwrap();
                *guard += 1;
            }
        });
        handles.push(h);
    }

    for h in handles {
        h.join().unwrap();
    }

    println!("final count: {}", counter.lock().unwrap());
}
```

Output: `final count: 1000`. Deterministic, because the mutex serializes access.

### Try to cause a data race

Without the mutex, the compiler stops you:

```rust
let counter = Arc::new(0);
let c = Arc::clone(&counter);
thread::spawn(move || {
    *c += 1;                    // ERROR: Arc<i32> doesn't allow mutation
});
```

```
error: cannot assign to data in an `Arc`
```

Good. You can't compile the race.

### Lock scope matters

```rust
let guard = mutex.lock().unwrap();
do_something_slow();           // LOCK STILL HELD — other threads blocked
drop(guard);
do_something_fast();
```

Always hold locks as briefly as possible. If you hold a lock across a blocking call, you've got a scaling problem.

Idiomatic pattern for brief locks:

```rust
let value = {
    let guard = mutex.lock().unwrap();
    *guard
};   // guard dropped here
do_something_with(value);
```

## Step 4 — A* pathfinding (single-threaded)

Now the real work. A* finds the shortest path on a grid from start to goal, given walls.

```rust
use std::collections::{BinaryHeap, HashMap};
use std::cmp::Reverse;

pub type Pos = (i32, i32);

pub fn astar(start: Pos, goal: Pos, walls: &std::collections::HashSet<Pos>, width: i32, height: i32) -> Option<Vec<Pos>> {
    let mut open = BinaryHeap::new();
    // Reverse for min-heap behavior (BinaryHeap is max-heap)
    open.push(Reverse((0i32, start)));
    let mut came_from: HashMap<Pos, Pos> = HashMap::new();
    let mut g_score: HashMap<Pos, i32> = HashMap::new();
    g_score.insert(start, 0);

    while let Some(Reverse((_, current))) = open.pop() {
        if current == goal {
            return Some(reconstruct(came_from, current));
        }

        for &(dx, dy) in &[(1i32, 0i32), (-1, 0), (0, 1), (0, -1)] {
            let neighbor = (current.0 + dx, current.1 + dy);
            if neighbor.0 < 0 || neighbor.0 >= width || neighbor.1 < 0 || neighbor.1 >= height {
                continue;
            }
            if walls.contains(&neighbor) {
                continue;
            }

            let tentative = g_score.get(&current).copied().unwrap_or(i32::MAX) + 1;
            if tentative < g_score.get(&neighbor).copied().unwrap_or(i32::MAX) {
                came_from.insert(neighbor, current);
                g_score.insert(neighbor, tentative);
                let f = tentative + manhattan(neighbor, goal);
                open.push(Reverse((f, neighbor)));
            }
        }
    }

    None
}

fn manhattan(a: Pos, b: Pos) -> i32 {
    (a.0 - b.0).abs() + (a.1 - b.1).abs()
}

fn reconstruct(came_from: HashMap<Pos, Pos>, end: Pos) -> Vec<Pos> {
    let mut path = vec![end];
    let mut cur = end;
    while let Some(&prev) = came_from.get(&cur) {
        path.push(prev);
        cur = prev;
    }
    path.reverse();
    path
}
```

### What's happening

- **`BinaryHeap`** is a max-heap. `Reverse(...)` makes it behave as a min-heap.
- The `open` set holds `(f_score, position)` pairs. We always pop the one with the lowest `f_score`.
- `g_score[n]` is the actual distance from start to `n`.
- `f_score = g_score + manhattan_to_goal` — optimistic estimate of total path length through `n`.
- `came_from` tracks where we came from, for reconstructing the path at the end.

## Step 5 — Test it sequentially

```rust
use std::collections::HashSet;

fn main() {
    let width = 80;
    let height = 40;
    let mut walls = HashSet::new();

    // Border walls
    for x in 0..width {
        walls.insert((x, 0));
        walls.insert((x, height - 1));
    }
    for y in 0..height {
        walls.insert((0, y));
        walls.insert((width - 1, y));
    }

    // A few obstacles
    for y in 5..30 {
        walls.insert((20, y));
    }
    for y in 10..35 {
        walls.insert((40, y));
    }

    let path = astar((5, 20), (70, 20), &walls, width, height);
    match path {
        Some(p) => println!("Path length: {}", p.len()),
        None => println!("No path"),
    }
}
```

```
Path length: 66
```

## Step 6 — A parallel workload

Now: 16 AI snakes, each needing a fresh path every tick. Per tick we compute 16 paths.

Serial version:

```rust
fn run_serial(ais: &[(Pos, Pos)], walls: &HashSet<Pos>, width: i32, height: i32) -> Vec<Option<Vec<Pos>>> {
    ais.iter()
        .map(|(start, goal)| astar(*start, *goal, walls, width, height))
        .collect()
}
```

Parallel version using `std::thread::scope`:

```rust
fn run_parallel(ais: &[(Pos, Pos)], walls: &HashSet<Pos>, width: i32, height: i32) -> Vec<Option<Vec<Pos>>> {
    let results = std::sync::Mutex::new(vec![None; ais.len()]);
    std::thread::scope(|s| {
        for (i, &(start, goal)) in ais.iter().enumerate() {
            let results = &results;
            s.spawn(move || {
                let path = astar(start, goal, walls, width, height);
                results.lock().unwrap()[i] = path;
            });
        }
    });
    results.into_inner().unwrap()
}
```

### `std::thread::scope`

This is a gem. Unlike `thread::spawn`, threads spawned via `scope` are *guaranteed* to finish before `scope` returns. That means inside the scope, the threads can borrow from the surrounding stack — no `'static` bound, no `Arc` cloning for data that outlives the scope.

We still need `Mutex` around `results` because many threads write into it. But `walls` is a shared read — we borrow it normally, no `Arc`.

### Try with mpsc channels instead

Alternative design: workers send path results over a channel; main thread collects.

```rust
fn run_parallel_channels(
    ais: &[(Pos, Pos)],
    walls: &HashSet<Pos>,
    width: i32,
    height: i32,
) -> Vec<Option<Vec<Pos>>> {
    let (tx, rx) = mpsc::channel();

    std::thread::scope(|s| {
        for (i, &(start, goal)) in ais.iter().enumerate() {
            let tx = tx.clone();
            s.spawn(move || {
                let path = astar(start, goal, walls, width, height);
                tx.send((i, path)).unwrap();
            });
        }
        drop(tx);

        let mut results = vec![None; ais.len()];
        while let Ok((i, path)) = rx.recv() {
            results[i] = path;
        }
        results
    })
}
```

Both designs are common. Channels are slightly more code but avoid the `Mutex`.

## Step 7 — Benchmark

```rust
use std::time::Instant;

fn main() {
    let width = 80;
    let height = 40;
    let mut walls = HashSet::new();
    for x in 0..width {
        walls.insert((x, 0));
        walls.insert((x, height - 1));
    }
    for y in 0..height {
        walls.insert((0, y));
        walls.insert((width - 1, y));
    }
    for y in 5..30 { walls.insert((20, y)); }
    for y in 10..35 { walls.insert((40, y)); }

    let ais: Vec<(Pos, Pos)> = (0..16)
        .map(|i| ((2 + (i % 3), 2 + i * 2), (77 - (i % 3), 37 - i * 2)))
        .collect();

    const TICKS: usize = 200;

    let start = Instant::now();
    for _ in 0..TICKS {
        let _ = run_serial(&ais, &walls, width, height);
    }
    let serial_time = start.elapsed();

    let start = Instant::now();
    for _ in 0..TICKS {
        let _ = run_parallel(&ais, &walls, width, height);
    }
    let parallel_time = start.elapsed();

    let start = Instant::now();
    for _ in 0..TICKS {
        let _ = run_parallel_channels(&ais, &walls, width, height);
    }
    let channels_time = start.elapsed();

    println!("Serial:          {:>6.2?}", serial_time);
    println!("Parallel mutex:  {:>6.2?}  ({:.1}x)", parallel_time, serial_time.as_secs_f64() / parallel_time.as_secs_f64());
    println!("Parallel channel: {:>6.2?}  ({:.1}x)", channels_time, serial_time.as_secs_f64() / channels_time.as_secs_f64());
}
```

Run with release:

```bash
cargo run --release
```

On my 8-core laptop:

```
Serial:             1.85s
Parallel mutex:     0.41s  (4.5x)
Parallel channel:   0.44s  (4.2x)
```

Substantial speedup. Not 8x because thread-spawn overhead dominates on short A* runs — for production you'd use a **thread pool** (the `rayon` crate handles this automatically; we meet it on Day 26).

### Release mode is non-negotiable

Debug builds are 10-100× slower than release. Benchmarking in debug is meaningless. Even for quick sanity checks, `cargo run --release`.

## Step 8 — `RwLock` for read-heavy state

If many threads read the same data but few write, `RwLock` is better than `Mutex`:

```rust
use std::sync::RwLock;

let data = Arc::new(RwLock::new(vec![1, 2, 3]));

// Many readers concurrently
for _ in 0..4 {
    let d = Arc::clone(&data);
    thread::spawn(move || {
        let read_guard = d.read().unwrap();
        println!("read: {:?}", *read_guard);
    });
}

// One writer (blocks readers and other writers)
let d = Arc::clone(&data);
thread::spawn(move || {
    let mut write_guard = d.write().unwrap();
    write_guard.push(4);
});
```

`read()` blocks if a writer is active; many readers run concurrently. `write()` blocks all other access until the guard drops.

Good fit for: game worlds (many AIs reading, occasional writer), caches, configuration.

## Common pitfalls

### "`Rc<T>` cannot be sent between threads safely"

You accidentally used `Rc` instead of `Arc`. The fix is one letter.

### "`RefCell<T>` cannot be shared between threads safely"

Same issue with `RefCell` vs `Mutex`/`RwLock`. Replace `Rc<RefCell<T>>` with `Arc<Mutex<T>>` or `Arc<RwLock<T>>`.

### Deadlock

Thread A holds lock L1 and tries to acquire L2. Thread B holds L2 and tries to acquire L1. Neither can proceed. Classic deadlock.

Mitigations:
- **Always acquire locks in the same global order.** If every thread acquires L1 before L2, no deadlock possible.
- **Hold locks briefly.** Short critical sections = short window for deadlock.
- **Try `try_lock`.** Returns `None` instead of blocking; you can back off.

### Poisoning

A thread panics while holding a `Mutex`. Subsequent `lock()` calls return `Err(PoisonError)`. Best handling:

```rust
let guard = match mutex.lock() {
    Ok(g) => g,
    Err(poisoned) => poisoned.into_inner(),   // use the data anyway
};
```

Or panic explicitly if you want to propagate: `.lock().unwrap()`.

### `thread::spawn` vs `thread::scope`

`thread::spawn` creates a thread with a `'static` closure — the closure can't borrow from the stack. Use for long-running background threads.

`thread::scope` scoped threads are guaranteed to finish; can borrow. Use for "fork and join" patterns.

For parallel work, strongly prefer `scope`.

### Forgetting to drop the last sender

If you clone `tx` for workers but forget to drop the original in the main thread, the channel never closes. Receivers hang on `recv()` forever. Always `drop(tx)` after cloning to workers.

### Benchmarking debug mode

Your parallel version looks 2x slower than serial. You built in debug. Add `--release`.

## What you learned

- **`std::thread::spawn`** and **`thread::scope`** — create OS threads, wait for them.
- **Channels (`mpsc`)** — message-passing between threads.
- **`Arc<T>`** — thread-safe reference counting; `Rc`'s cousin.
- **`Mutex<T>`** — exclusive access. **`RwLock<T>`** — many readers or one writer.
- **`Send`** and **`Sync`** marker traits control who's allowed across threads.
- The compiler prevents data races statically — "fearless concurrency."
- **A\* algorithm** — priority queue + heuristic.
- **Mutex poisoning** and how to handle it.
- Always benchmark in release.

## Exercises

1. **Thread pool.** Write a simple fixed-size thread pool: N worker threads pulling jobs from a shared `Arc<Mutex<VecDeque<Job>>>`. Workers use a `Condvar` to sleep when the queue is empty. Measure against per-job spawn.
2. **Rayon.** Install `rayon` and rewrite `run_parallel` as `ais.par_iter().map(...).collect()`. Compare performance. This is a preview of Day 26.
3. **Atomic counter.** Use `std::sync::atomic::AtomicU32` instead of `Mutex<u32>` for a hot counter. Benchmark. Atomics are lock-free and usually faster for simple operations.
4. **Parallel merge sort.** Implement merge sort that splits work across threads for the top N levels of recursion. Use `thread::scope`. Compare to `slice.sort()`.
5. **Deadlock demo.** Intentionally write two threads that deadlock by acquiring mutexes in different orders. Confirm the program hangs. Then fix by enforcing a global lock order.

## What you learned this week

Week 2 done. You've covered:

- **Closures** and the `Fn`/`FnMut`/`FnOnce` hierarchy — Day 8
- **Explicit lifetimes** in practice — Day 9
- **Smart pointers**: `Box`, `Rc`, `RefCell`, `Weak` — Day 10
- **Terminal rendering** with `crossterm` — Day 11
- **Fixed-timestep game loops** — Day 12
- **Testing at every scale** — unit, integration, doc, property — Day 13
- **Threads, channels, `Arc`/`Mutex`** — Day 14

You've also shipped a full terminal arcade game (Snake). Week 3 pivots away from games entirely — you're going to build a real key-value database, from scratch, over seven days.

## What's next

Day 15 introduces **binary I/O** — the foundation for everything database-related. You'll design a byte-level file format with magic bytes, length-prefixed records, and CRC checksums, then use `BufReader`/`BufWriter` to read and write it efficiently.

→ Day 15 — Binary I/O (coming in next installment)
