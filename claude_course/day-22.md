# Day 22 — Canvas, Pixels, and PNG Output

**Domain:** 3D graphics • **Time:** 1.5 hours • **Difficulty:** easy

## What you'll build

The foundation for a ray tracer: a `Canvas` struct that holds an array of floating-point RGB pixels, plus code to save it as a PNG file. You'll pick 32-bit floats over 8-bit bytes (you'll see why), implement sRGB gamma correction, and render a simple test pattern to prove the pipeline works end-to-end. By the end of today you won't have any rays yet — but you'll have the canvas, the color math, and the output path that every further day builds on.

## What you'll learn

- Why ray tracers use **floating-point colors** internally, not `u8`
- **Linear vs. sRGB** color space — the gamma problem and why it matters
- The `image` crate for PNG I/O
- Designing a simple `Canvas` API with bounds-checked accessors
- `clap` for command-line arguments on a real tool
- How to structure a render pipeline so future features (rays, materials, BVH) slot in cleanly

## Background

### Why float colors?

A naive pixel buffer is `[u8; 3]` per pixel: 0-255 for red, green, blue. That's what PNGs store on disk, and it works if all you're doing is displaying static images. Ray tracers are different. Mid-render, you might accumulate contributions from 100 light samples per pixel, some extremely bright (the sun), some very dim (indirect bounce). If you quantized each sample to 0-255 and added them, you'd lose precision catastrophically — 100 samples of value 1 would add up to 100 and fit fine, but 100 samples of value 0.02 would each round to 0 and you'd get black.

Floats give you 7-8 decimal digits of precision with huge range (~10^-38 to 10^38). You accumulate in float, and only at the very end — when saving — do you convert to 8-bit integers.

Also: floats handle **high dynamic range**. A pixel might have R=100.0 (a very bright highlight), which we'd later tone-map to a displayable value. You can't even represent that in `u8`.

### Linear vs. sRGB: the gamma problem

Here's a common gotcha. Say you want a pixel that's "50% gray." You write `200, 200, 200` (out of 255) to a PNG. You view it next to pure white — and it looks way brighter than half, maybe 80% of the way to white.

That's because your monitor doesn't display pixel values linearly. A value of 200 actually outputs about 60% of the brightness of 255, not 78% (which is 200/255). Monitors have a roughly **gamma 2.2 curve** built in — historically because CRT electron guns had that nonlinearity, and modern displays preserve the convention.

The sRGB standard formalizes this. Every PNG, JPEG, and image you've seen is in sRGB — pixel values are pre-warped so that when the monitor applies its gamma, the result looks linear to a human.

For a ray tracer, this matters twice:

1. **Input textures** (if you read a JPEG of a wood floor) are in sRGB. You must convert to linear before doing any math with them.
2. **Output colors** need to be converted from linear back to sRGB before saving, or everything looks too dark.

Inside the ray tracer, we do all math in linear space. Physical laws (1/r² falloff, energy conservation, BRDF calculations) only work in linear space.

For today's simple test pattern, we'll apply a cheap gamma approximation: `sqrt(linear)` is close enough to `linear^(1/2.2)`, and it's what most introductory ray tracers (like Peter Shirley's "Ray Tracing in One Weekend") use.

```
linear 0.0  →  sRGB 0
linear 0.25 →  sqrt ≈ 0.5 → sRGB 128
linear 0.5  →  sqrt ≈ 0.71 → sRGB 181
linear 1.0  →  sRGB 255
```

### Coordinate conventions

There are competing conventions in graphics:

- **Y-up vs. Y-down.** Image files store rows top-to-bottom; many frameworks (SDL, raw OpenGL textures) use Y-down. 3D world-space is usually Y-up. We'll use Y-up in the world (makes camera math cleaner) and Y-down for the image grid (makes saving easier), and convert at the boundary.
- **Origin in the center or a corner.** For world space, center. For images, top-left corner.
- **Handedness.** Right-handed coordinate systems (standard math). Left-handed (DirectX-style). Stick with right-handed.

Decide these up front and write them down in a comment. Most bugs in graphics code come from inconsistent conventions.

### The `image` crate

We'll use [`image`](https://docs.rs/image) for PNG encoding. It's the de facto crate for image I/O in Rust. You hand it a buffer of `u8`s laid out as `[r, g, b, r, g, b, ...]` and a width/height, and it writes a PNG.

## Setting up

Today we start a brand-new project:

```bash
cd ~/rust-course/  # or wherever you keep day projects
cargo new raytracer
cd raytracer
```

Add dependencies:

```bash
cargo add image
cargo add clap --features derive
```

Your `Cargo.toml` should now look roughly like:

```toml
[package]
name = "raytracer"
version = "0.1.0"
edition = "2021"

[dependencies]
image = "0.25"
clap = { version = "4", features = ["derive"] }
```

Check that `cargo build` works before proceeding.

## Step 1 — The `Canvas` struct

Create `src/canvas.rs`:

```rust
use std::path::Path;

/// A rectangular grid of linear-space RGB pixels, each stored as three f32s.
pub struct Canvas {
    width: u32,
    height: u32,
    pixels: Vec<[f32; 3]>,
}

impl Canvas {
    /// Create a canvas filled with black pixels.
    pub fn new(width: u32, height: u32) -> Self {
        let pixels = vec![[0.0, 0.0, 0.0]; (width * height) as usize];
        Self { width, height, pixels }
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    /// Get a pixel by (x, y). Panics on out-of-bounds.
    pub fn get(&self, x: u32, y: u32) -> [f32; 3] {
        self.pixels[self.idx(x, y)]
    }

    /// Set a pixel by (x, y). Panics on out-of-bounds.
    pub fn set(&mut self, x: u32, y: u32, color: [f32; 3]) {
        let i = self.idx(x, y);
        self.pixels[i] = color;
    }

    fn idx(&self, x: u32, y: u32) -> usize {
        assert!(x < self.width, "x ({x}) >= width ({})", self.width);
        assert!(y < self.height, "y ({y}) >= height ({})", self.height);
        (y * self.width + x) as usize
    }
}
```

A few design notes:

- **Pixels stored as `[f32; 3]`**, not a named `Color` struct. Later we'll introduce `Vec3` and use it for both positions and colors, as Shirley does. For today, the plain array is fine.
- **Row-major layout**: `(y * width + x)`. This is the convention for image crates too, so save becomes trivial.
- **Panics on out-of-bounds** rather than `Option`. Rendering code should never access out-of-bounds pixels — if it does, that's a bug worth surfacing loudly, not silently.

Register the module in `src/main.rs` (we'll flesh out `main` later, but for now):

```rust
mod canvas;
```

## Step 2 — Linear-to-sRGB conversion

Still in `src/canvas.rs`, add a conversion helper. We'll put the whole save path here for now:

```rust
use image::{Rgb, RgbImage};

/// Convert a linear [0, 1] float channel to sRGB [0, 255] u8.
/// Uses the `sqrt` gamma 2.0 approximation — good enough for test renders.
fn linear_to_srgb(linear: f32) -> u8 {
    let clamped = linear.clamp(0.0, 1.0);
    let gamma = clamped.sqrt();
    (gamma * 255.0).round() as u8
}

impl Canvas {
    /// Save the canvas as an 8-bit sRGB PNG.
    pub fn save_png(&self, path: impl AsRef<Path>) -> image::ImageResult<()> {
        let mut img = RgbImage::new(self.width, self.height);
        for y in 0..self.height {
            for x in 0..self.width {
                let [r, g, b] = self.pixels[(y * self.width + x) as usize];
                let r = linear_to_srgb(r);
                let g = linear_to_srgb(g);
                let b = linear_to_srgb(b);
                img.put_pixel(x, y, Rgb([r, g, b]));
            }
        }
        img.save(path)
    }
}
```

### Why clamp?

If a color channel is bigger than 1.0 — say a very bright highlight — `sqrt(1.5)` is 1.22, and multiplying by 255 gives 312. Casting to `u8` wraps around (in debug it panics, in release it saturates). `.clamp(0.0, 1.0)` defines the behavior: anything above 1.0 becomes pure white.

Future days will replace this with proper **tone mapping** (e.g., Reinhard or ACES) that compresses high values smoothly. For today, clamp is fine.

### Why `.round()` then `as u8`?

`(x * 255.0) as u8` just truncates, biasing colors slightly dark. `.round()` does banker's rounding. A subtle difference but the kind of detail that shows up as faint banding in gradients.

## Step 3 — Write a test pattern

Time to render something. Edit `src/main.rs`:

```rust
mod canvas;

use canvas::Canvas;
use clap::Parser;

#[derive(Parser)]
#[command(about = "A toy ray tracer")]
struct Args {
    /// Output image path.
    #[arg(short, long, default_value = "out.png")]
    output: String,

    /// Image width in pixels.
    #[arg(long, default_value_t = 400)]
    width: u32,

    /// Image height in pixels.
    #[arg(long, default_value_t = 300)]
    height: u32,
}

fn main() {
    let args = Args::parse();

    let mut canvas = Canvas::new(args.width, args.height);

    // Classic "uv test pattern":
    //   red increases to the right,
    //   green increases downward,
    //   blue is a fixed moderate value.
    for y in 0..args.height {
        for x in 0..args.width {
            let u = x as f32 / (args.width - 1) as f32;
            let v = y as f32 / (args.height - 1) as f32;
            canvas.set(x, y, [u, v, 0.25]);
        }
    }

    canvas
        .save_png(&args.output)
        .expect("failed to save PNG");

    println!("Wrote {}x{} image to {}", args.width, args.height, args.output);
}
```

### Why "uv"?

In graphics, `u` and `v` are conventional names for normalized coordinates in the range `[0, 1]`. Textures use `(u, v)` to index themselves independently of their pixel resolution. We're using the same convention here.

Run it:

```bash
cargo run --release -- --output uv.png
```

Expected output:

```
Wrote 400x300 image to uv.png
```

Open `uv.png`. You should see:

- Black in the upper-left corner (u=0, v=0 → red=0, green=0).
- Yellowish in the bottom-right (u=1, v=1 → red=1, green=1, so yellow).
- Red gradient along the top, green gradient down the left, with a constant blue tint (0.25 → 128 in sRGB gamma, so a noticeable but muted blue).

If the image looks too dark or bands visibly, your gamma conversion might be off. If it's upside-down, you've confused Y-up and Y-down — check that `(0, 0)` is the top-left.

### Why `--release`?

The debug build is noticeably slower for pixel loops — float math, array indexing, and panics on bounds checks all get optimized away in release. Every example from now on assumes `--release`.

## Step 4 — A sky gradient

Let's render something more like a real ray tracer's "hello world." Add a function to `src/main.rs`:

```rust
/// Return the sky color for a given vertical position (v: 0 = top, 1 = bottom).
/// Blends from soft blue at the top to near-white at the horizon.
fn sky_color(v: f32) -> [f32; 3] {
    // t = 0 at top, 1 at bottom
    let t = v;
    // lerp(a, b, t) = a + (b - a) * t
    let top = [0.5, 0.7, 1.0];      // sky blue
    let bottom = [1.0, 1.0, 1.0];   // near-white horizon
    [
        top[0] + (bottom[0] - top[0]) * t,
        top[1] + (bottom[1] - top[1]) * t,
        top[2] + (bottom[2] - top[2]) * t,
    ]
}
```

Then replace the body of `main`'s rendering loop:

```rust
    for y in 0..args.height {
        for x in 0..args.width {
            let v = y as f32 / (args.height - 1) as f32;
            canvas.set(x, y, sky_color(v));
        }
    }
```

Re-run:

```bash
cargo run --release -- --output sky.png
```

Open `sky.png`. You should see a vertical gradient from a nice sky blue at top to white at the bottom. Each row is a solid color (we ignored `x` in `sky_color`). This will be the default background whenever a ray misses all geometry — it's going to look exactly like this on Day 23 when we fire real rays.

## Step 5 — Subcommands with clap

A real tool has more than one mode. Let's restructure into subcommands, which we'll keep extending over the next six days.

Replace `src/main.rs` with:

```rust
mod canvas;

use canvas::Canvas;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(about = "A toy ray tracer", version)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Render a UV test pattern (diagnostic).
    Uv {
        #[arg(short, long, default_value = "uv.png")]
        output: String,
        #[arg(long, default_value_t = 400)]
        width: u32,
        #[arg(long, default_value_t = 300)]
        height: u32,
    },
    /// Render a plain sky gradient.
    Sky {
        #[arg(short, long, default_value = "sky.png")]
        output: String,
        #[arg(long, default_value_t = 400)]
        width: u32,
        #[arg(long, default_value_t = 300)]
        height: u32,
    },
}

fn main() {
    match Cli::parse().command {
        Command::Uv { output, width, height } => render_uv(&output, width, height),
        Command::Sky { output, width, height } => render_sky(&output, width, height),
    }
}

fn render_uv(output: &str, width: u32, height: u32) {
    let mut canvas = Canvas::new(width, height);
    for y in 0..height {
        for x in 0..width {
            let u = x as f32 / (width - 1) as f32;
            let v = y as f32 / (height - 1) as f32;
            canvas.set(x, y, [u, v, 0.25]);
        }
    }
    canvas.save_png(output).expect("save failed");
    println!("Wrote {width}x{height} image to {output}");
}

fn render_sky(output: &str, width: u32, height: u32) {
    let mut canvas = Canvas::new(width, height);
    for y in 0..height {
        for x in 0..width {
            let v = y as f32 / (height - 1) as f32;
            canvas.set(x, y, sky_color(v));
        }
    }
    canvas.save_png(output).expect("save failed");
    println!("Wrote {width}x{height} image to {output}");
}

fn sky_color(v: f32) -> [f32; 3] {
    let t = v;
    let top = [0.5, 0.7, 1.0];
    let bottom = [1.0, 1.0, 1.0];
    [
        top[0] + (bottom[0] - top[0]) * t,
        top[1] + (bottom[1] - top[1]) * t,
        top[2] + (bottom[2] - top[2]) * t,
    ]
}
```

Run it two ways:

```bash
cargo run --release -- uv --output uv.png
cargo run --release -- sky --output sky.png --width 800 --height 600
```

`clap` handles flag parsing, `--help` generation, argument validation, and subcommand dispatch. Try `cargo run -- --help` and `cargo run -- sky --help`.

## Step 6 — A roundtrip test

Let's verify the canvas preserves data correctly. Add this at the bottom of `src/canvas.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_and_get() {
        let mut c = Canvas::new(4, 3);
        c.set(2, 1, [0.5, 0.25, 0.75]);
        let px = c.get(2, 1);
        assert!((px[0] - 0.5).abs() < 1e-6);
        assert!((px[1] - 0.25).abs() < 1e-6);
        assert!((px[2] - 0.75).abs() < 1e-6);
    }

    #[test]
    fn default_is_black() {
        let c = Canvas::new(2, 2);
        for y in 0..2 {
            for x in 0..2 {
                assert_eq!(c.get(x, y), [0.0, 0.0, 0.0]);
            }
        }
    }

    #[test]
    #[should_panic(expected = "x (5) >= width (4)")]
    fn out_of_bounds_x_panics() {
        let c = Canvas::new(4, 3);
        c.get(5, 0);
    }

    #[test]
    fn gamma_roundtrip() {
        // Pure white stays pure white.
        assert_eq!(linear_to_srgb(1.0), 255);
        // Black stays black.
        assert_eq!(linear_to_srgb(0.0), 0);
        // Out-of-range gets clamped.
        assert_eq!(linear_to_srgb(2.0), 255);
        assert_eq!(linear_to_srgb(-0.5), 0);
        // sqrt(0.25) = 0.5 → 128
        assert_eq!(linear_to_srgb(0.25), 128);
    }
}
```

Run `cargo test`:

```
running 4 tests
test canvas::tests::default_is_black ... ok
test canvas::tests::gamma_roundtrip ... ok
test canvas::tests::out_of_bounds_x_panics ... ok
test canvas::tests::set_and_get ... ok

test result: ok. 4 passed; 0 failed
```

Four small tests, but together they pin down the most error-prone parts: indexing, default value, bounds, and gamma conversion.

## Step 7 — Organize for the week ahead

Before wrapping up, let's create the module files we'll fill in over the next six days. This saves editing `mod` declarations every morning.

```bash
touch src/vec3.rs src/ray.rs src/hit.rs src/material.rs src/camera.rs src/scene.rs
```

And register them in `src/main.rs`. Since they're empty, add `#![allow(dead_code)]` so clippy doesn't shout until we use them:

```rust
#![allow(dead_code)]

mod camera;
mod canvas;
mod hit;
mod material;
mod ray;
mod scene;
mod vec3;
```

Verify with `cargo build` — no errors, maybe some warnings that we've already silenced.

Also expose them via a library so future benchmarks and integration tests can reach in:

```bash
touch src/lib.rs
```

In `src/lib.rs`:

```rust
#![allow(dead_code)]

pub mod camera;
pub mod canvas;
pub mod hit;
pub mod material;
pub mod ray;
pub mod scene;
pub mod vec3;
```

And update `Cargo.toml` to expose both binary and library:

```toml
[lib]
name = "raytracer"
path = "src/lib.rs"

[[bin]]
name = "raytracer"
path = "src/main.rs"
```

Update `src/main.rs` imports to use the library:

```rust
use raytracer::canvas::Canvas;
// Remove: mod canvas;
```

Now anything reusable is in the library, and `main.rs` is just a thin CLI shell. Run `cargo build` and `cargo test` — both should still pass.

## Common pitfalls

### The image looks upside down

Sky color is at the top of the screen, but you drew it at the bottom. You indexed `y` from the bottom. Fix: ensure `y=0` is the top row (matches the `image` crate's convention). If you want world-space "Y up," convert at the save boundary only.

### The image looks too dark

You forgot gamma. Linear `0.5` with no gamma correction becomes `0.5 * 255 = 128`, but a sRGB value of 128 renders as roughly **22%** brightness, not 50%. `sqrt(0.5) = 0.71`, which gives sRGB 181 (~50%). Double-check `linear_to_srgb` applies `.sqrt()`.

### `as u8` wrapping bug

```rust
let v = (1.5 * 255.0) as u8;  // = 126, not 255!
```

Casting a float above 255 to `u8` wraps in older Rust, saturates in newer Rust (Rust 1.45+). Either way you lose information. Always clamp before cast.

### Rayon / threads / complexity too early

Don't parallelize yet! Day 26 will add parallelism with `rayon`. If you try to use `rayon::par_iter_mut` today, the API will look ugly because `Canvas::set(x, y, ...)` needs `&mut self` and you can't share that across threads. Tomorrow's code will use a `Vec<[f32; 3]>` directly for the pixel buffer — cleaner for parallelization.

### Forgetting to build the lib target

If you split into `src/lib.rs` + `src/main.rs` but forget to `use raytracer::canvas::Canvas` in main, you get "unresolved import `canvas`" — because main no longer has a `mod canvas` declaration. Always update both.

### PNG save fails silently

`image::save` returns `Result<()>`. If you `let _ = canvas.save_png(...)`, a permission error will vanish. Always `.expect("save failed")` or propagate with `?`.

## What you learned

- **Float colors, not u8**: ray tracers accumulate in linear-space `f32` and convert at the very end.
- **Gamma correction matters**: `sqrt` is a cheap approximation of proper sRGB.
- The `image` crate handles PNG encoding with a simple `RgbImage` API.
- `clap` subcommands give you a clean multi-mode CLI with auto-generated `--help`.
- A `Canvas` with bounds-checked `get`/`set` accessors keeps the API safe while panicking loudly on bugs.
- A UV test pattern catches coordinate-system confusion (up/down, origin, handedness) before real rendering starts.
- Layering `lib.rs` + `main.rs` lets tests, benches, and integration tests reach into render code.

## Exercises

1. **Solid fill.** Add a `canvas fill --color "1.0 0.5 0.0"` subcommand that renders a solid orange canvas. Good practice parsing multi-value args in clap.
2. **Checkerboard.** Add a `checker` subcommand that draws alternating 16x16 squares. Prove you can do rich patterns with just `(x, y)`.
3. **sRGB proper.** Replace the `sqrt` approximation with the real sRGB OETF (piecewise linear at low values, power for high). Compare outputs — can you see the difference?
4. **PPM output.** Add `save_ppm(path)` that writes a P3 ASCII PPM. PPM is a trivial format (`"P3\n{w} {h}\n255\n{r} {g} {b} ...\n"`) and some graphics tools accept it without the `image` crate.
5. **16-bit output.** Use `image::Rgb<u16>` for a 16-bit PNG. Subtle gradients will look smoother. Verify with a render of the sky gradient.

## What's next

Tomorrow you implement **`Vec3`** — a 3-dimensional vector with overloaded `+`, `-`, `*`, `Neg`, and dot/cross/length helpers. Then the **`Ray`** struct (origin + direction) and, yes, the famous **ray-sphere intersection**: expanding the quadratic `(P(t) - C)·(P(t) - C) = r²` to find where a ray hits a sphere. By end of Day 23, you'll render your first real 3D scene: a sphere against the sky gradient.

→ [Day 23 — Vec3, Rays, and the First Sphere](day-23.md)
