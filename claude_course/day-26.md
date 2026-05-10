# Day 26 — Parallel Rendering with Rayon

**Domain:** 3D graphics • **Time:** 2 hours • **Difficulty:** medium

## What you'll build

A parallel renderer. The same three-spheres scene from yesterday, rendered across all your CPU cores at once. You'll refactor the nested-loop render into a flat pixel buffer, use `rayon` to fill it in parallel with one function call, and seed each pixel's RNG from its `(x, y)` coordinates — so the output is **bit-for-bit identical** regardless of whether you run on 1, 4, or 16 threads. Then you'll benchmark the scaling to see how close you get to the theoretical maximum.

## What you'll learn

- How `rayon` replaces `iter` with `par_iter` and the data-parallel model behind it
- Why naive shared-RNG parallelism produces non-reproducible output
- **Deterministic parallelism**: seed each work unit from its index, not a global RNG
- `par_chunks_mut` for per-row or per-pixel parallel writes
- Choosing work granularity (per-pixel vs. per-row vs. per-tile)
- Measuring parallel speedup and reading the scaling curve
- `rayon::current_num_threads()` and `ThreadPoolBuilder` for controlled benchmarks

## Background

### Why parallelism now

Yesterday you hit the wall. At 500 samples and 800x450, rendering takes minutes. A ray tracer is the textbook example of an "embarrassingly parallel" workload: every pixel is independent of every other pixel. No shared mutable state in the inner loop, no data dependencies between pixels. If you have 8 cores, in principle you should get ~8x speedup.

The two obvious approaches:

1. **`std::thread`**: spawn threads, partition the work, join. Verbose, and you have to think about sending data between threads, stitching output together, and managing thread lifecycle.
2. **`rayon`**: data-parallel library that turns `iter` into `par_iter`. One import, one method call, done.

Rayon is the right tool for this.

### The rayon model

Rayon is a *work-stealing* parallel executor. You declare a parallel iterator (e.g., `par_iter_mut` over a slice), and rayon splits the range across a thread pool. Each worker thread takes a chunk, and when it finishes early, it steals work from a busier thread. You don't schedule — you just describe the work.

Minimal example (a toy one, not related to our raytracer):

```rust
use rayon::prelude::*;

fn main() {
    let nums: Vec<i32> = (1..=1_000_000).collect();
    let sum: i64 = nums.par_iter().map(|n| *n as i64).sum();
    println!("sum = {sum}");
}
```

The `par_iter()` call (enabled by the `rayon::prelude::*` import) behaves like `iter()` — it still yields `&i32` — but under the hood, it dispatches the iteration across rayon's thread pool. The `sum` reduction is split and combined automatically.

Rules to know:

- Parallel iterators require their items to be `Send` (safe to move between threads). Our `Vec3`, `Ray`, `HitRecord`, and `Arc<dyn Material>` already are — we designed for this by adding `Send + Sync` to the `Hittable` trait on Day 24.
- Closures passed to `par_iter().map(...)` must be `Sync` (safe to share). Most plain closures over shared data are.
- No need to explicitly spawn or join threads — rayon manages its own pool.

### The determinism problem

Here's the subtle part. Our current renderer has one RNG per render: `rng.gen()` is called many times inside the pixel loop to jitter sample positions and to scatter rays. The *sequence* of values drawn from that RNG depends on the order the code runs.

Serial version: always the same order → same output for the same seed.

Naive parallel version: threads race for the RNG (assuming we wrap it in a Mutex). Thread 3 might grab seed draws 47 and 48 while thread 1 grabs 49. Next run, thread 1 gets there first. Result: different RNG sequence → different pixel colors → different output image. Still statistically correct, but the image is non-reproducible.

This is bad for:

- **Debugging**: if the output changes each run, you can't reproduce bugs.
- **Testing**: snapshot tests fail randomly.
- **Benchmarks**: noise in the output masks real perf changes.

The fix is conceptually simple: **don't share the RNG across work units**. Give each pixel its own RNG, seeded deterministically from the pixel's coordinates.

```rust
let seed = (x as u64) * 0x9E3779B97F4A7C15 ^ (y as u64) * 0xBF58476D1CE4E5B9;
let mut rng = SmallRng::seed_from_u64(seed);
```

Those magic numbers are primes from [SplitMix64](https://prng.di.unimi.it/splitmix64.c) — they spread values well across the seed space so neighboring pixels don't get correlated RNGs. This works because `SmallRng::seed_from_u64` is cheap (low microseconds). We create a fresh RNG per pixel, use it for `samples` draws, then drop it. The overhead is negligible compared to the work per pixel.

Now: parallel execution order doesn't matter. Pixel `(42, 17)` always gets the same seed, draws the same RNG sequence, computes the same color.

### Work granularity

Rayon lets you pick granularity: per-item, per-chunk, per-tile. Smaller chunks = better load balancing but more scheduling overhead. For a raytracer:

- **Per-pixel**: `buffer.par_iter_mut().enumerate()`. Perfectly balanced, but a bit of scheduling overhead since pixels are cheap compared to the scheduler's bookkeeping.
- **Per-row**: `buffer.par_chunks_mut(width * 3).enumerate()`. A row is typically ~1000 pixels → ~1ms of work at low sample counts, ~100ms at high. Good granularity.
- **Per-tile** (e.g., 16x16 blocks): better cache locality but awkward to index.

We'll start with per-row. It's the standard choice and matches how image files are laid out.

## Setting up

### Add rayon to `Cargo.toml`

```toml
[dependencies]
clap = { version = "4", features = ["derive"] }
image = "0.25"
rand = "0.8"
rayon = "1.10"

[dev-dependencies]
criterion = "0.5"
```

Quick sanity check:

```bash
cargo build
```

Should pick up `rayon` and compile without issue.

## Step 1 — A minimal rayon warmup

Before touching the renderer, convince yourself rayon works. Add a throwaway file `examples/parallel_warmup.rs`:

```rust
use rayon::prelude::*;
use std::time::Instant;

fn heavy(n: u64) -> u64 {
    let mut x = n;
    for _ in 0..10_000 {
        x = x.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    }
    x
}

fn main() {
    let data: Vec<u64> = (0..100_000).collect();

    let t0 = Instant::now();
    let serial: u64 = data.iter().map(|n| heavy(*n)).sum();
    let t_serial = t0.elapsed();

    let t0 = Instant::now();
    let parallel: u64 = data.par_iter().map(|n| heavy(*n)).sum();
    let t_parallel = t0.elapsed();

    assert_eq!(serial, parallel);
    println!("serial:   {:.2?}", t_serial);
    println!("parallel: {:.2?}", t_parallel);
    println!("speedup:  {:.2}x", t_serial.as_secs_f64() / t_parallel.as_secs_f64());
    println!("threads:  {}", rayon::current_num_threads());
}
```

Run it:

```bash
cargo run --release --example parallel_warmup
```

Expected output on a 4-core machine:

```
serial:   850ms
parallel: 230ms
speedup:  3.70x
threads:  8
```

Exact numbers vary. What should happen:

- `parallel` is faster than `serial`.
- Speedup is below your logical core count (4-8x is realistic on an 8-thread machine).
- `serial == parallel`: both sums agree, because `heavy` is deterministic per input.

If this works, rayon is ready.

## Step 2 — Refactor render to return a buffer

Right now, `render` in `src/main.rs` has this shape:

```rust
for y in 0..height {
    for x in 0..width {
        let color = ...;
        canvas.set(x, y, [color.x, color.y, color.z]);
    }
}
canvas.save_png(output)?;
```

The `canvas.set(x, y, ...)` writes into a `Vec<[f32; 3]>` indexed by `y * width + x`. To parallelize, we need to index into that `Vec` from parallel threads. Rayon's `par_chunks_mut` does exactly that on a slice, splitting the slice across threads so each thread writes into a disjoint range.

First, expose the raw buffer on `Canvas`. Open `src/canvas.rs`:

```rust
pub struct Canvas {
    width: u32,
    height: u32,
    pixels: Vec<[f32; 3]>,
}

impl Canvas {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            pixels: vec![[0.0; 3]; (width * height) as usize],
        }
    }

    pub fn width(&self) -> u32 { self.width }
    pub fn height(&self) -> u32 { self.height }

    pub fn set(&mut self, x: u32, y: u32, color: [f32; 3]) {
        let i = (y * self.width + x) as usize;
        self.pixels[i] = color;
    }

    // NEW: expose the buffer as a mutable slice for rayon.
    pub fn pixels_mut(&mut self) -> &mut [[f32; 3]] {
        &mut self.pixels
    }

    // save_png stays as-is.
    pub fn save_png(&self, path: &str) -> Result<(), image::ImageError> {
        // ... existing code
    }
}
```

The `pixels_mut` getter exposes the contiguous pixel array. We'll hand that to rayon.

## Step 3 — Seed per pixel deterministically

Extract the seed computation into a helper. Put it at the top of `src/main.rs`:

```rust
/// Deterministic per-pixel seed. Pixel `(x, y)` always produces
/// the same seed regardless of thread count.
fn pixel_seed(x: u32, y: u32) -> u64 {
    // SplitMix64 prime constants, ensure neighboring pixels
    // get well-separated seeds.
    let a = (x as u64).wrapping_mul(0x9E3779B97F4A7C15);
    let b = (y as u64).wrapping_mul(0xBF58476D1CE4E5B9);
    a ^ b ^ 0xD6E8FEB86659FD93
}
```

Two tests to anchor this, at the bottom of `src/main.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pixel_seed_is_deterministic() {
        assert_eq!(pixel_seed(42, 17), pixel_seed(42, 17));
    }

    #[test]
    fn pixel_seed_differs_by_neighbor() {
        let a = pixel_seed(42, 17);
        let b = pixel_seed(43, 17);
        let c = pixel_seed(42, 18);
        assert_ne!(a, b);
        assert_ne!(a, c);
        assert_ne!(b, c);
    }
}
```

```bash
cargo test pixel_seed
```

Both should pass.

The `wrapping_mul` is important — overflow on `u64` with `*` panics in debug. `wrapping_mul` lets it wrap around cleanly, which is what hash-style seeding wants.

## Step 4 — Parallelize with par_chunks_mut

Now the main refactor. In `src/main.rs`, rewrite the render function:

```rust
use rayon::prelude::*;
use rand::Rng;
use rand::rngs::SmallRng;
use rand::SeedableRng;

fn render(output: &str, width: u32, height: u32, samples: u32) {
    let aspect = width as f32 / height as f32;

    let camera = Camera::new(
        Vec3::new(-2.0, 2.0, 1.0),
        Vec3::new(0.0, 0.0, -1.0),
        Vec3::new(0.0, 1.0, 0.0),
        30.0,
        aspect,
    );

    let mat_ground = Lambertian::new(Vec3::new(0.8, 0.8, 0.0));
    let mat_center = Lambertian::new(Vec3::new(0.1, 0.2, 0.5));
    let mat_left = Dielectric::new(1.5);
    let mat_right = Metal::new(Vec3::new(0.8, 0.6, 0.2), 0.1);

    let mut world = HittableList::new();
    world.add(Box::new(Sphere::new(Vec3::new(0.0, -100.5, -1.0), 100.0, mat_ground)));
    world.add(Box::new(Sphere::new(Vec3::new(0.0,    0.0, -1.0),   0.5, mat_center)));
    world.add(Box::new(Sphere::new(Vec3::new(-1.0,   0.0, -1.0),   0.5, Arc::clone(&mat_left))));
    world.add(Box::new(Sphere::new(Vec3::new(-1.0,   0.0, -1.0), -0.45, mat_left)));
    world.add(Box::new(Sphere::new(Vec3::new(1.0,    0.0, -1.0),   0.5, mat_right)));

    let mut canvas = Canvas::new(width, height);
    let inv_samples = 1.0 / samples as f32;
    let max_depth = 50;

    // Grab shared immutable references first. These can be sent
    // across threads because &T is Send when T is Sync.
    let camera_ref = &camera;
    let world_ref: &dyn Hittable = &world;

    let row_bytes = width as usize;
    canvas
        .pixels_mut()
        .par_chunks_mut(row_bytes)
        .enumerate()
        .for_each(|(y, row)| {
            let y = y as u32;
            for (x, pixel) in row.iter_mut().enumerate() {
                let x = x as u32;
                let mut rng = SmallRng::seed_from_u64(pixel_seed(x, y));

                let mut accum = Vec3::ZERO;
                for _ in 0..samples {
                    let du: f32 = rng.gen();
                    let dv: f32 = rng.gen();
                    let s = (x as f32 + du) / (width - 1) as f32;
                    let t = 1.0 - (y as f32 + dv) / (height - 1) as f32;
                    let ray = camera_ref.ray(s, t);
                    accum = accum + ray_color(&ray, world_ref, max_depth, &mut rng);
                }

                let color = accum * inv_samples;
                *pixel = [color.x, color.y, color.z];
            }
        });

    canvas.save_png(output).expect("save failed");
    eprintln!("Wrote {width}x{height} image to {output}");
}
```

Three things changed from yesterday:

1. **`par_chunks_mut(row_bytes)`**: splits the pixel slice into rows; rayon dispatches each row to a worker thread.
2. **Per-pixel seeded RNG**: `SmallRng::seed_from_u64(pixel_seed(x, y))` inside the pixel loop. No shared RNG.
3. **No progress print**: rows are being done out of order on different threads, so "row 5 done" isn't meaningful. We'll fix progress in Step 6.

Note the `world_ref: &dyn Hittable = &world;` — this upcasts `&HittableList` to `&dyn Hittable`. We need `dyn Hittable` because `ray_color` takes `&dyn Hittable`, and it needs to work for the upcast to `Sync` via the trait's `Send + Sync` bound.

Run it:

```bash
cargo run --release -- render --output three_par.png --samples 100
```

Expected: 5-10x faster than yesterday's render (depending on core count). Image should look identical to `three.png` — we'll verify that next.

## Step 5 — Verify bit-identical output

This is the important test. Render the same scene twice, once serial-style (single-threaded), once parallel, and confirm they produce identical files.

Single-thread run:

```bash
RAYON_NUM_THREADS=1 cargo run --release -- render --output three_st.png --samples 50
```

Rayon respects `RAYON_NUM_THREADS`: if you set it to 1, rayon uses exactly one worker, effectively running the code serially.

Multi-thread run:

```bash
cargo run --release -- render --output three_mt.png --samples 50
```

Compare:

```bash
cmp three_st.png three_mt.png && echo "IDENTICAL" || echo "DIFFERENT"
```

Expected output:

```
IDENTICAL
```

If you get "DIFFERENT", you have a determinism bug. The most common culprits:

- RNG passed from the outer closure instead of seeded per-pixel.
- Floating-point accumulation order (but `accum + ...` inside a single pixel runs on one thread, so this shouldn't bite).
- A subtle bug where the `for_each` closure captures a mutable variable.

### Why this matters

Deterministic parallelism is a property you pay roughly nothing for (the per-pixel `seed_from_u64` call is microseconds vs milliseconds of render work), and it transforms your debugging experience. When a pixel looks wrong, you can run serial and parallel, confirm they match, then pinpoint the issue in serial with `println!` debugging — without worrying that threading added confusion. You can also checksum renders as regression tests.

## Step 6 — Progress reporting across threads

Rows finish out of order. To show progress, use an `AtomicU32` counter — threads increment it when they finish a row, and any thread can print based on the current count.

Add at the top of `src/main.rs`:

```rust
use std::sync::atomic::{AtomicU32, Ordering};
```

In `render`, before `par_chunks_mut`:

```rust
let rows_done = AtomicU32::new(0);
let rows_done_ref = &rows_done;
```

Inside the `for_each` closure, after the row is filled:

```rust
let done = rows_done_ref.fetch_add(1, Ordering::Relaxed) + 1;
if height >= 10 && done % (height / 10) == 0 {
    eprintln!("{done}/{height} rows");
}
```

Full closure now:

```rust
.for_each(|(y, row)| {
    let y = y as u32;
    for (x, pixel) in row.iter_mut().enumerate() {
        // ... pixel rendering, as before ...
    }
    let done = rows_done_ref.fetch_add(1, Ordering::Relaxed) + 1;
    if height >= 10 && done % (height / 10) == 0 {
        eprintln!("{done}/{height} rows");
    }
});
```

`Ordering::Relaxed` is correct here — we don't need happens-before guarantees, just atomicity of the increment. The `+ 1` converts the `fetch_add`'s pre-increment return value into the post-increment count.

Run again and watch the output:

```
80/800 rows
160/800 rows
240/800 rows
...
```

They might not come exactly at the 10% marks (two threads could increment to `done = 800` almost simultaneously, and only the one whose increment lands on an exact decile prints), but it's a reasonable proxy.

## Step 7 — Benchmark thread scaling

Measure speedup as you vary thread count. Rayon exposes `ThreadPoolBuilder`, which lets you build a custom pool with a fixed number of threads and run code on it.

Add `num_cpus` as a dependency? You don't need to — use `std::thread::available_parallelism`. Or use `rayon::current_num_threads()` from inside.

Create `examples/scaling.rs`:

```rust
use std::time::Instant;

fn main() {
    // Build a miniature scene: 50x50 pixels, 100 samples.
    // Small enough to iterate quickly, large enough for real timings.
    let width = 200u32;
    let height = 150u32;
    let samples = 100u32;

    for threads in [1, 2, 4, 8] {
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .expect("pool build failed");

        let t0 = Instant::now();
        pool.install(|| {
            // Call the render function here.
            // But render is private! See next paragraph.
        });
        let elapsed = t0.elapsed();

        println!("{threads:2} threads: {:.2?}", elapsed);
    }
}
```

To call `render` from an example, it needs to be exposed from `lib.rs`, or we factor out just the pixel loop into a library function. The simplest move: make the `render` function public by moving it into `src/lib.rs` or exposing a `render_to_buffer` helper.

Quick fix — add to `src/lib.rs`:

```rust
pub mod renderer;
```

Create `src/renderer.rs`:

```rust
use crate::{
    camera::Camera,
    canvas::Canvas,
    hit::{Hittable, HittableList},
    material::{Dielectric, Lambertian, Metal},
    ray::Ray,
    scene::Sphere,
    vec3::Vec3,
};
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use rayon::prelude::*;
use std::sync::Arc;

pub fn pixel_seed(x: u32, y: u32) -> u64 {
    let a = (x as u64).wrapping_mul(0x9E3779B97F4A7C15);
    let b = (y as u64).wrapping_mul(0xBF58476D1CE4E5B9);
    a ^ b ^ 0xD6E8FEB86659FD93
}

fn ray_color(ray: &Ray, world: &dyn Hittable, depth: u32, rng: &mut SmallRng) -> Vec3 {
    if depth == 0 {
        return Vec3::ZERO;
    }
    if let Some(hit) = world.hit(ray, 0.001, f32::INFINITY) {
        if let Some((scattered, attenuation)) = hit.material.scatter(ray, &hit, rng) {
            return attenuation * ray_color(&scattered, world, depth - 1, rng);
        }
        return Vec3::ZERO;
    }
    let unit = ray.direction.normalize();
    let t = 0.5 * (unit.y + 1.0);
    Vec3::new(1.0, 1.0, 1.0) * (1.0 - t) + Vec3::new(0.5, 0.7, 1.0) * t
}

pub fn three_spheres_scene() -> (Camera, HittableList) {
    let aspect = 16.0 / 9.0;
    let camera = Camera::new(
        Vec3::new(-2.0, 2.0, 1.0),
        Vec3::new(0.0, 0.0, -1.0),
        Vec3::new(0.0, 1.0, 0.0),
        30.0,
        aspect,
    );

    let mat_ground = Lambertian::new(Vec3::new(0.8, 0.8, 0.0));
    let mat_center = Lambertian::new(Vec3::new(0.1, 0.2, 0.5));
    let mat_left = Dielectric::new(1.5);
    let mat_right = Metal::new(Vec3::new(0.8, 0.6, 0.2), 0.1);

    let mut world = HittableList::new();
    world.add(Box::new(Sphere::new(Vec3::new(0.0, -100.5, -1.0), 100.0, mat_ground)));
    world.add(Box::new(Sphere::new(Vec3::new(0.0,    0.0, -1.0),   0.5, mat_center)));
    world.add(Box::new(Sphere::new(Vec3::new(-1.0,   0.0, -1.0),   0.5, Arc::clone(&mat_left))));
    world.add(Box::new(Sphere::new(Vec3::new(-1.0,   0.0, -1.0), -0.45, mat_left)));
    world.add(Box::new(Sphere::new(Vec3::new(1.0,    0.0, -1.0),   0.5, mat_right)));

    (camera, world)
}

pub fn render_to_canvas(
    canvas: &mut Canvas,
    camera: &Camera,
    world: &dyn Hittable,
    samples: u32,
    max_depth: u32,
) {
    let width = canvas.width();
    let height = canvas.height();
    let inv_samples = 1.0 / samples as f32;

    canvas
        .pixels_mut()
        .par_chunks_mut(width as usize)
        .enumerate()
        .for_each(|(y, row)| {
            let y = y as u32;
            for (x, pixel) in row.iter_mut().enumerate() {
                let x = x as u32;
                let mut rng = SmallRng::seed_from_u64(pixel_seed(x, y));
                let mut accum = Vec3::ZERO;
                for _ in 0..samples {
                    let du: f32 = rng.gen();
                    let dv: f32 = rng.gen();
                    let s = (x as f32 + du) / (width - 1) as f32;
                    let t = 1.0 - (y as f32 + dv) / (height - 1) as f32;
                    let ray = camera.ray(s, t);
                    accum = accum + ray_color(&ray, world, max_depth, &mut rng);
                }
                let color = accum * inv_samples;
                *pixel = [color.x, color.y, color.z];
            }
        });
}
```

Now `main.rs` becomes much simpler — its `render` function delegates to `render_to_canvas`. Update `main.rs` accordingly (keep the CLI; delete the body that duplicates what moved to `renderer.rs`).

With the renderer as a library function, `examples/scaling.rs` can call it:

```rust
use raytracer::canvas::Canvas;
use raytracer::renderer::{render_to_canvas, three_spheres_scene};
use std::time::Instant;

fn main() {
    let width = 200u32;
    let height = 150u32;
    let samples = 100u32;
    let max_depth = 50;
    let (camera, world) = three_spheres_scene();

    for threads in [1, 2, 4, 8] {
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .expect("pool build failed");

        let t0 = Instant::now();
        pool.install(|| {
            let mut canvas = Canvas::new(width, height);
            render_to_canvas(&mut canvas, &camera, &world, samples, max_depth);
        });
        let elapsed = t0.elapsed();
        println!("{threads:2} threads: {:>8.2?}", elapsed);
    }
}
```

Run it:

```bash
cargo run --release --example scaling
```

Expected output on a 4-core (8-thread) machine:

```
 1 threads:    8.20s
 2 threads:    4.30s
 4 threads:    2.40s
 8 threads:    1.80s
```

Things to notice:

- **1→2 threads**: usually ~2x. Very little scheduling overhead at this size.
- **2→4 threads**: still close to linear if you have 4 physical cores.
- **4→8 threads**: less than 2x. Hyperthreading shares ALU resources, so you get maybe 1.2-1.4x from it, not 2x.
- Past the physical core count, returns diminish fast.

This is **Amdahl's law** in the wild. Our raytracer is close to 100% parallel, so we're limited only by the overhead (rayon's work queue, cache contention, memory bandwidth). A real production renderer gets ~90-95% parallel efficiency on 8 cores, which matches what we see.

## Step 8 — Determinism across thread counts

Run the CLI at two different thread counts and diff the outputs:

```bash
RAYON_NUM_THREADS=1 cargo run --release -- render --output one.png --samples 50
RAYON_NUM_THREADS=4 cargo run --release -- render --output four.png --samples 50
cmp one.png four.png && echo IDENTICAL || echo DIFFERENT
```

Expected: `IDENTICAL`. If you see `DIFFERENT`, something is non-deterministic. Common cause: using `rng` as a mutable closure capture instead of seeding fresh per pixel.

### Why it's identical despite out-of-order execution

Float addition is *not* associative in general: `(a + b) + c` can differ from `a + (b + c)` at the last bit. But our pixel loop accumulates samples *sequentially within one pixel*, and the per-pixel thread never changes mid-pixel. So within a pixel the add order is fixed. Across pixels, the buffer writes are to disjoint indices, so the order of writes doesn't matter.

The only non-determinism would be if one thread wrote to a pixel that another thread also read/wrote. Our `par_chunks_mut` partition guarantees that never happens.

## Common pitfalls

### `&mut world` doesn't implement `Send`

Error: `&mut HittableList` is not `Send` by default in older Rust, and passing it into a rayon closure fails. Our closures take `&dyn Hittable` (not mutable), which is `Sync` via the trait's `Send + Sync` bound. If you see this error, double-check you're not accidentally taking `&mut world` — the world is read-only during render.

### "cannot borrow `rows_done` as mutable"

Error: `cannot borrow data in an `&` reference as mutable`. You tried `rows_done.fetch_add(...)` where `rows_done` was not defined as an `AtomicU32`. Double-check the type — atomics expose `fetch_add` through `&self`, not `&mut self`.

### par_iter works but output is garbled

Symptom: the render runs, but pixels look shuffled or have stripes of black rows. You forgot to use `enumerate` correctly — the row index passed from `par_chunks_mut` starts at 0 and counts chunks, so it equals `y`. If you instead did `par_iter().enumerate()` on the full buffer treating each pixel as an item, your `x` and `y` computation must extract them from the linear index: `y = idx / width; x = idx % width`.

### Output differs at 1 vs 4 threads

Almost always a shared RNG. Grep your render for `rng.gen` and confirm every call is to a pixel-local RNG created inside the closure. If you shared an RNG as a closure capture wrapped in a Mutex, you get contention *and* non-determinism.

### Performance doesn't improve

Check that you're in `--release`. Debug builds disable inlining of trait methods, and the per-ray overhead swamps parallelism benefits. A release build is non-negotiable.

Also: did you accidentally render a very tiny image? If the total work is under ~50ms, rayon overhead dominates. Scale up to a few seconds of work before benchmarking.

### `pixels_mut` not found

Error: `no method named `pixels_mut` found`. You added the method body but forgot `pub`, or you added it inside the wrong `impl` block. Check `src/canvas.rs` — the method must be inside `impl Canvas { ... }` and marked `pub`.

### Stack overflow in threads

Rayon's worker threads have a default stack size (typically 2 MB). Our recursion is bounded by `max_depth = 50`, well within. If you set depth to thousands or have a deeply nested scene graph, increase via `ThreadPoolBuilder::new().stack_size(8 * 1024 * 1024)`.

### `ThreadPoolBuilder` error: "global pool already initialized"

If you build one pool and later try to modify the global pool, you get this. Use `pool.install(|| { ... })` — it runs your closure on the specific pool, leaving the global default alone. This is what the benchmark example does.

## What you learned

- **Rayon** turns `iter` into `par_iter` with one prelude import. Work-stealing thread pool handles scheduling for you.
- **`par_chunks_mut(chunk_size)`** partitions a mutable slice into parallel chunks. Perfect for per-row rendering.
- **Deterministic parallelism**: seed each work unit from its index, never share a mutable RNG. Cheap to create per pixel; reproducible output.
- **Work granularity** matters: per-row is usually right for rendering — per-pixel has too much overhead, per-tile is awkward.
- **`AtomicU32`** with `Ordering::Relaxed` gives cross-thread counters for progress reporting.
- **Speedup caps out** around your physical core count. Hyperthreading adds 20-40%, not 100%.
- **Amdahl's law**: parallel speedup is bounded by the serial fraction of work. Our renderer is ~98% parallel, so we scale very well.
- **Bit-identical output across thread counts** is a feature you get free when you seed per pixel. Use it for snapshot tests.

## Exercises

1. **Per-tile parallelism.** Instead of per-row, render 16x16 tiles. Measure whether the cache locality helps (probably yes for large images). Hint: you can't use `par_chunks_mut` directly — flatten tile indices and use `par_iter_mut().enumerate()`, then compute `(x, y)` from the tile index.
2. **Progress bar.** Replace the plain `eprintln!` with the `indicatif` crate's `ProgressBar::new(height).set_position(done)`. It gives you a live-updating bar with ETA.
3. **Stratified sampling.** Instead of pure random samples, partition each pixel into a `sqrt(samples) x sqrt(samples)` subgrid; draw one random sample per subcell. Reduces variance for the same sample count. Implement and compare noise between grainy-vs-stratified at 16 samples.
4. **Early termination by variance.** After every 16 samples, compute the running variance of the accumulated color. If it's below a threshold, stop sampling. Pixels in flat regions (sky, ground) converge fast; detailed regions take longer. Measure total time saved on the three-spheres scene.
5. **GPU port.** Research `wgpu` or `rust-gpu`. Sketch (don't fully implement) how you'd port the render loop to the GPU. Which parts transfer? Which (trait objects, `Arc<dyn Material>`) don't?

## What's next

Your renderer is fast. But adding a new scene means recompiling the binary — scene data is still hard-coded in `three_spheres_scene()`. **Day 27 fixes that with RON scene files.** You'll define `serde`-derived descriptors for materials, shapes, and cameras, parse them from `.ron` files at runtime with tagged enums (`#[serde(tag = "kind")]`), and extend the CLI: `raytracer render scenes/three_spheres.ron --samples 500`. Now you can iterate on scene design without a rebuild.

→ [Day 27 — RON Scene Files](day-27.md)
