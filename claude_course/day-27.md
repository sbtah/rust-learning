# Day 27 — RON Scene Files

**Domain:** 3D graphics • **Time:** 2 hours • **Difficulty:** medium

## What you'll build

A scene loader. Today your renderer stops compiling scene data into the binary. Instead, you write `.ron` files that describe a camera, a list of shapes, and materials, and load them at runtime. You'll define `serde`-derived **descriptor types** (a.k.a. DTOs), use **tagged enums** (`#[serde(tag = "kind")]`) to pick the right material or shape from a string tag, and convert those descriptors into the runtime types your renderer already uses. Then you'll extend the CLI: `raytracer render scenes/three_spheres.ron --samples 500 --output out.png`. Editing a scene file and re-rendering will no longer require `cargo build`.

## What you'll learn

- What RON is and why it's nicer than JSON for hand-written config
- `serde::Deserialize` — derive it on plain structs, free parsing for many formats
- **Descriptor types (DTOs)** vs. runtime types, and why you keep them separate
- **Tagged enums** in serde: picking a variant from a `kind:` string
- `#[serde(default)]`, `#[serde(rename)]`, `#[serde(flatten)]` — when each matters
- Converting descriptors to runtime types via an `impl From` or a `build()` method
- Propagating scene-file errors with `anyhow` context
- Integrating scene loading into the `clap` CLI

## Background

### What RON is

RON = Rusty Object Notation. It's a config format that looks like Rust literals:

```ron
Scene(
    camera: Camera(
        lookfrom: (-2.0, 2.0, 1.0),
        lookat: (0.0, 0.0, -1.0),
        vfov: 30.0,
    ),
    spheres: [
        (center: (0.0, 0.0, -1.0), radius: 0.5, material: Lambertian(albedo: (0.1, 0.2, 0.5))),
    ],
)
```

Compared to JSON it supports:

- Trailing commas (much nicer for editing lists).
- Struct-named tuples: `Vec3(1.0, 0.0, 0.0)`.
- Enum variants as `Variant(fields)` or `"Variant"` directly.
- Comments (`// ...`).

Like JSON, it's round-trippable with serde. We read it with `ron::from_str` or `ron::de::from_reader`.

You could use JSON, YAML, or TOML instead — they all plug into serde the same way. We pick RON because it feels closest to Rust literals, which makes scene files look like data your program already understands.

### What `serde` actually does

`serde` is two traits: `Serialize` and `Deserialize`, plus a pile of derive macros that implement them automatically. When you write:

```rust
#[derive(serde::Deserialize)]
struct Point { x: f32, y: f32 }
```

…the derive macro generates code roughly equivalent to "for each field, call `deserializer.deserialize_f32()` and collect them into `Point`". The derived impls are data-format-agnostic — the *format* is provided by a separate crate (`ron`, `serde_json`, `toml`, `serde_yaml`). You write the struct once, then read it from any of those formats.

This is the same pattern as Python's `dataclass` + `pydantic` or `attrs`, except:

- The parser is generated at **compile time**, so there's no reflection cost at runtime.
- Missing or malformed fields are errors you catch immediately, not silent `None`s.
- Field types are checked statically — you can't accidentally bind a string to an `f32`.

### Why separate "descriptor" types from runtime types

Here's the temptation: slap `#[derive(Deserialize)]` onto `Sphere`, `Camera`, `Lambertian`, and be done.

Don't. There are two reasons:

1. **Runtime types carry non-serializable baggage.** `Sphere::material` is `Arc<dyn Material>`. You can't deserialize a trait object — serde needs a concrete layout. `Camera` caches precomputed basis vectors; those shouldn't be in the file, only the inputs that produce them.
2. **Scene files should outlive code changes.** If you rename a field in `Camera` for a refactor, scene files break. If you keep a descriptor layer, the descriptor is the stable contract; the runtime type is free to evolve.

The pattern: define `SceneDesc`, `MaterialDesc`, `ShapeDesc` — plain data. Then write a `build()` or `impl From` that constructs the runtime types from the descriptor. This is the same pattern you'd use in Python with a Pydantic `SceneConfig` → `build_scene()` function.

### Tagged enums in serde

You need to express "a material is one of Lambertian, Metal, or Dielectric." Serde handles this with `#[serde(tag = "kind")]` on an enum:

```rust
#[derive(serde::Deserialize)]
#[serde(tag = "kind")]
enum MaterialDesc {
    Lambertian { albedo: [f32; 3] },
    Metal { albedo: [f32; 3], fuzz: f32 },
    Dielectric { ior: f32 },
}
```

In RON this reads as:

```ron
Lambertian(albedo: (0.8, 0.0, 0.0))
```

…or in JSON (just to show the format is truly format-agnostic):

```json
{ "kind": "Lambertian", "albedo": [0.8, 0.0, 0.0] }
```

Serde sees `"kind": "Lambertian"` and picks the right variant to deserialize into. If `kind` is missing or unknown, you get a descriptive error.

There are four tagging styles — `tag`, `tag` + `content`, `untagged`, and external (the default). For our case, "internally tagged" (`tag = "kind"`) is the cleanest because it keeps all fields at the same level.

### `?` with `anyhow::Context`

On Day 7 you wrote `thiserror` and `anyhow`. We'll lean on that again: `ron::from_str` returns a `ron::Error`; converting it via `?` into `anyhow::Result` is one line. Adding `.with_context(|| format!("loading {}", path.display()))?` attaches the filename, so if parsing fails, you see *which* file was malformed.

## Setting up

We'll build on yesterday's raytracer. If you don't have it, copy your `day-26` project directory.

```bash
cd raytracer
cargo add serde --features=derive
cargo add ron
cargo add clap --features=derive      # if you don't already have it
cargo add anyhow
```

We already have `anyhow` from earlier weeks in most setups. Check `Cargo.toml` and add it only if missing.

Also make sure you have a scenes directory:

```bash
mkdir scenes
```

## Step 1 — Write the first scene file

Before any Rust, write `scenes/three_spheres.ron` by hand. This is the target format the parser will accept:

```ron
(
    camera: (
        lookfrom: (-2.0, 2.0, 1.0),
        lookat: (0.0, 0.0, -1.0),
        vup: (0.0, 1.0, 0.0),
        vfov: 30.0,
    ),
    shapes: [
        (
            shape: Sphere(center: (0.0, -100.5, -1.0), radius: 100.0),
            material: Lambertian(albedo: (0.8, 0.8, 0.0)),
        ),
        (
            shape: Sphere(center: (0.0, 0.0, -1.0), radius: 0.5),
            material: Lambertian(albedo: (0.1, 0.2, 0.5)),
        ),
        (
            shape: Sphere(center: (-1.0, 0.0, -1.0), radius: 0.5),
            material: Dielectric(ior: 1.5),
        ),
        (
            shape: Sphere(center: (-1.0, 0.0, -1.0), radius: -0.45),
            material: Dielectric(ior: 1.5),
        ),
        (
            shape: Sphere(center: (1.0, 0.0, -1.0), radius: 0.5),
            material: Metal(albedo: (0.8, 0.6, 0.2), fuzz: 0.1),
        ),
    ],
)
```

A few things to notice:

- The outer `(...)` is a struct literal with no name — `ron::from_str::<SceneDesc>` tells RON what type to expect. You *could* write `SceneDesc(camera: ..., shapes: ...)` with the type name in front, but it's redundant.
- Each shape entry pairs a `shape` and a `material`. We model that as a `ShapeEntry` struct with two enum fields.
- `Sphere(...)`, `Lambertian(...)` are *enum variants* with named fields. Serde's internally-tagged enum picks the variant by name.
- Vec3s are written as `(x, y, z)` tuples — we'll deserialize them into `[f32; 3]` arrays for simplicity.

This file doesn't do anything yet. But having the target format in front of you makes designing the descriptor types much easier.

## Step 2 — Add the scene module

Create `src/scene_file.rs`:

```rust
use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct SceneDesc {
    pub camera: CameraDesc,
    pub shapes: Vec<ShapeEntry>,
}

#[derive(Deserialize, Debug)]
pub struct CameraDesc {
    pub lookfrom: [f32; 3],
    pub lookat: [f32; 3],
    #[serde(default = "default_vup")]
    pub vup: [f32; 3],
    pub vfov: f32,
}

fn default_vup() -> [f32; 3] {
    [0.0, 1.0, 0.0]
}

#[derive(Deserialize, Debug)]
pub struct ShapeEntry {
    pub shape: ShapeDesc,
    pub material: MaterialDesc,
}

#[derive(Deserialize, Debug)]
pub enum ShapeDesc {
    Sphere { center: [f32; 3], radius: f32 },
}

#[derive(Deserialize, Debug)]
pub enum MaterialDesc {
    Lambertian { albedo: [f32; 3] },
    Metal { albedo: [f32; 3], fuzz: f32 },
    Dielectric { ior: f32 },
}
```

Add to `src/lib.rs`:

```rust
pub mod scene_file;
```

A few things worth naming:

- Descriptor types are **plain data**: all fields `pub`, `Debug` for error messages, no methods yet.
- `CameraDesc::vup` has `#[serde(default = "default_vup")]`. If `vup:` is missing from the file, it defaults to Y-up `(0, 1, 0)`. This is cheap forward compat.
- `ShapeDesc` and `MaterialDesc` are externally-tagged enums (no `#[serde(tag)]`). In RON, externally tagged enums are written as `Variant(field: value, ...)`, which is the natural RON syntax and looks cleanest in practice. If you were using JSON, you'd switch to `#[serde(tag = "kind")]` here to get `{"kind": "Lambertian", ...}`.

### Quick sanity check

Add a tiny test to verify parsing works end-to-end:

```rust
// bottom of src/scene_file.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_minimal_scene() {
        let src = r#"(
            camera: (lookfrom: (0.0, 0.0, 0.0), lookat: (0.0, 0.0, -1.0), vfov: 60.0),
            shapes: [
                (shape: Sphere(center: (0.0, 0.0, -1.0), radius: 0.5),
                 material: Lambertian(albedo: (1.0, 0.0, 0.0))),
            ],
        )"#;
        let desc: SceneDesc = ron::from_str(src).expect("parse");
        assert_eq!(desc.shapes.len(), 1);
    }
}
```

Run it:

```bash
cargo test parses_minimal_scene
```

Expected output: `1 passed`. Notice we didn't write *any* parsing code — `serde::Deserialize` plus the `ron` crate covered it. That's the whole point of the derive macros.

## Step 3 — Convert descriptors to runtime types

The descriptors can't render anything yet — they're just shape-of-data. We need a `build()` that produces `(Camera, HittableList)`.

Add to `src/scene_file.rs` (at the end, before the `#[cfg(test)]`):

```rust
use crate::{
    camera::Camera,
    hit::{Hittable, HittableList},
    material::{Dielectric, Lambertian, Material, Metal},
    scene::Sphere,
    vec3::Vec3,
};
use std::sync::Arc;

impl CameraDesc {
    pub fn build(&self, aspect: f32) -> Camera {
        Camera::new(
            Vec3::from(self.lookfrom),
            Vec3::from(self.lookat),
            Vec3::from(self.vup),
            self.vfov,
            aspect,
        )
    }
}

impl MaterialDesc {
    pub fn build(&self) -> Arc<dyn Material> {
        match self {
            MaterialDesc::Lambertian { albedo } => Lambertian::new(Vec3::from(*albedo)),
            MaterialDesc::Metal { albedo, fuzz } => Metal::new(Vec3::from(*albedo), *fuzz),
            MaterialDesc::Dielectric { ior } => Dielectric::new(*ior),
        }
    }
}

impl ShapeEntry {
    pub fn build(&self) -> Box<dyn Hittable> {
        let material = self.material.build();
        match &self.shape {
            ShapeDesc::Sphere { center, radius } => Box::new(Sphere::new(
                Vec3::from(*center),
                *radius,
                material,
            )),
        }
    }
}

impl SceneDesc {
    pub fn build(&self, aspect: f32) -> (Camera, HittableList) {
        let camera = self.camera.build(aspect);
        let mut world = HittableList::new();
        for entry in &self.shapes {
            world.add(entry.build());
        }
        (camera, world)
    }
}
```

This needs `Vec3: From<[f32; 3]>`. Check if you have that already; if not, add to `src/vec3.rs`:

```rust
impl From<[f32; 3]> for Vec3 {
    fn from(v: [f32; 3]) -> Self {
        Vec3::new(v[0], v[1], v[2])
    }
}
```

Now `SceneDesc::build` gives you the exact same `(Camera, HittableList)` pair that `three_spheres_scene()` used to return. The rest of the renderer doesn't care that the scene came from a file.

### Aspect ratio is a runtime concern

Notice `build(aspect)` takes the aspect ratio as an argument, not as a field on `CameraDesc`. Aspect is width divided by height of the *output image*, which is a CLI flag, not a scene property. Same scene should render correctly at 800x450 or at 1920x1080.

## Step 4 — Loading from a file

Add a helper that reads the file and parses it, with good errors:

```rust
use anyhow::{Context, Result};
use std::path::Path;

pub fn load_scene(path: &Path) -> Result<SceneDesc> {
    let src = std::fs::read_to_string(path)
        .with_context(|| format!("reading scene file {}", path.display()))?;
    let desc: SceneDesc = ron::from_str(&src)
        .with_context(|| format!("parsing scene file {}", path.display()))?;
    Ok(desc)
}
```

The `.with_context(...)` calls attach a human-readable string to the error. If parsing fails, the user sees:

```
Error: parsing scene file scenes/three_spheres.ron

Caused by:
    0: 4:17: Expected ')'
```

…rather than just the raw parse error with no filename. This is `anyhow` at its most helpful — you get error chains for free.

## Step 5 — Integrate into the CLI

Open `src/main.rs`. It currently renders the hardcoded `three_spheres_scene()`. Rewire the CLI.

Replace the CLI definition with subcommands:

```rust
use clap::{Parser, Subcommand};
use raytracer::canvas::Canvas;
use raytracer::renderer::render_to_canvas;
use raytracer::scene_file::load_scene;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "raytracer")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Render a scene to a PNG file.
    Render {
        /// Path to a .ron scene file.
        scene: PathBuf,

        /// Output PNG path.
        #[arg(short, long, default_value = "out.png")]
        output: PathBuf,

        /// Image width in pixels.
        #[arg(long, default_value_t = 800)]
        width: u32,

        /// Image height in pixels.
        #[arg(long, default_value_t = 450)]
        height: u32,

        /// Samples per pixel.
        #[arg(short, long, default_value_t = 100)]
        samples: u32,

        /// Max recursion depth for scattered rays.
        #[arg(long, default_value_t = 50)]
        depth: u32,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Render { scene, output, width, height, samples, depth } => {
            let desc = load_scene(&scene)?;
            let aspect = width as f32 / height as f32;
            let (camera, world) = desc.build(aspect);

            let mut canvas = Canvas::new(width, height);
            let t0 = std::time::Instant::now();
            render_to_canvas(&mut canvas, &camera, &world, samples, depth);
            eprintln!("rendered in {:.2}s", t0.elapsed().as_secs_f32());

            canvas.save_png(&output)?;
            eprintln!("wrote {}", output.display());
        }
    }
    Ok(())
}
```

Build and run:

```bash
cargo run --release -- render scenes/three_spheres.ron --width 800 --height 450 --samples 100
```

Expected output:

```
rendered in 4.87s
wrote out.png
```

Open `out.png`. It should be the same three-sphere scene from Day 26.

### Why subcommands

We used `#[command(subcommand)]` + `enum Command` instead of a single flat set of flags. This pays off in the next section when we add other modes (`validate`, `show-info`). Each subcommand gets its own flags and its own help text:

```bash
cargo run -- render --help
```

## Step 6 — Add a `validate` subcommand

A nice ergonomic win: let the user check a scene file without rendering.

Add to the `Command` enum:

```rust
    /// Parse a scene file and report if it's valid.
    Validate {
        scene: PathBuf,
    },
```

And in `main`:

```rust
        Command::Validate { scene } => {
            let desc = load_scene(&scene)?;
            println!("OK: {} shapes", desc.shapes.len());
        }
```

Run:

```bash
cargo run -- validate scenes/three_spheres.ron
# OK: 5 shapes
```

Introduce a typo in the scene file (`Lambertan` instead of `Lambertian`) and run it again:

```
Error: parsing scene file scenes/three_spheres.ron

Caused by:
    0: 15:27: Unknown variant: Lambertan
```

The line/column and variant name tell you exactly where to look. This is the payoff for writing out-of-process config.

## Step 7 — A second scene, for variety

Create `scenes/two_balls.ron`:

```ron
(
    camera: (
        lookfrom: (0.0, 1.0, 2.0),
        lookat: (0.0, 0.0, -1.0),
        vfov: 45.0,
    ),
    shapes: [
        (shape: Sphere(center: (0.0, -100.5, -1.0), radius: 100.0),
         material: Lambertian(albedo: (0.5, 0.7, 0.5))),
        (shape: Sphere(center: (-0.5, 0.0, -1.0), radius: 0.5),
         material: Metal(albedo: (0.9, 0.9, 0.9), fuzz: 0.0)),
        (shape: Sphere(center: (0.5, 0.0, -1.0), radius: 0.5),
         material: Metal(albedo: (0.8, 0.4, 0.2), fuzz: 0.3)),
    ],
)
```

```bash
cargo run --release -- render scenes/two_balls.ron -o two_balls.png
```

You didn't touch Rust code. That's the whole point.

## Step 8 — Forward-compatible additions

Say next week you want to add a `sky_color` field to control the background:

```rust
#[derive(Deserialize, Debug)]
pub struct SceneDesc {
    pub camera: CameraDesc,
    pub shapes: Vec<ShapeEntry>,
    #[serde(default = "default_sky")]
    pub sky_top: [f32; 3],
    #[serde(default = "default_ground_sky")]
    pub sky_bottom: [f32; 3],
}

fn default_sky() -> [f32; 3] { [0.5, 0.7, 1.0] }
fn default_ground_sky() -> [f32; 3] { [1.0, 1.0, 1.0] }
```

With `#[serde(default)]`, **old scene files that don't mention sky still parse.** This is the whole forward-compat story. You add a field with a default; old files keep working; new files can set it explicitly.

Compare to Day 16 where we did the same thing for versioned save files.

If you want to *require* a field (no default), just don't mark it — parsing an old file will then fail with `missing field 'sky_top'`, which is sometimes what you want: a hard version break.

You don't need to wire the sky colors into the renderer today — just verify that adding the field doesn't break existing scenes.

## Common pitfalls

### "expected tuple variant but found struct variant"

If you write `ShapeDesc::Sphere(center, radius)` (positional) and the RON file says `Sphere(center: (...), radius: ...)` (named), serde complains. Keep the enum variants as *struct variants* (`Sphere { center, radius }`) to match the named-field RON style. The extra `{}` vs `()` distinction matters.

### Unknown field silently ignored?

By default, serde **accepts unknown fields and ignores them**. If you add a typo like `raduis: 0.5`, it parses fine and uses the default (error or zero). Usually not what you want. Add `#[serde(deny_unknown_fields)]` to any descriptor where strictness matters:

```rust
#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct CameraDesc { ... }
```

Now unknown fields are errors with line numbers. For scene files (hand-edited, easy to typo), this is almost always what you want.

### `#[serde(tag = "kind")]` fails on tuple variants

Internally-tagged enums don't work with tuple variants — only struct variants and unit variants. If you try `#[serde(tag = "kind")] enum Material { Lambertian(Vec3) }`, you get a compile error. Use named fields (`Lambertian { albedo: Vec3 }`) or switch to externally-tagged (remove the `#[serde(tag)]`).

### RON parser reports wrong line number

RON's error messages are sometimes off by a line because of comments and multiline strings. The fix is usually "look a line or two above the reported line." Annoying but not fatal. For JSON-based configs the line numbers tend to be more accurate — one reason JSON still has its place.

### The file exists but `read_to_string` still fails

On some shells, trailing newlines or BOM bytes trip up parsers. `ron` handles trailing whitespace fine. BOM (UTF-8 byte order mark at the start) can break parsing — if your editor wrote one, strip it: `sed -i $'1s/^\\xef\\xbb\\xbf//' scenes/foo.ron`.

## What you learned

- **RON** is a config format that looks like Rust literals — trailing commas, struct syntax, comments. Nicer than JSON for hand-edited files.
- **`#[derive(serde::Deserialize)]`** generates parsing code at compile time. No runtime reflection.
- **Descriptor types** (plain data) separate from **runtime types** (with `Arc<dyn Trait>`, caches, etc.) keeps scene files stable as the renderer evolves.
- **Externally-tagged enums** render naturally in RON as `Variant(field: ...)`. For JSON you'd switch to `#[serde(tag = "kind")]`.
- **`#[serde(default = "...")]`** gives you forward compat: new fields with defaults don't break old files.
- **`#[serde(deny_unknown_fields)]`** catches typos early — essential for hand-edited configs.
- **`.with_context(...)`** on `anyhow::Result` attaches filenames to error chains. Much more debuggable than raw parse errors.
- **`clap` subcommands** (`Subcommand` enum) scale better than flat flags once you have >2 modes.

## Exercises

1. **More shapes.** Add a `Plane { normal, d, ... }` variant to `ShapeDesc`. Implement `Hittable` for a new `Plane` runtime type. Test with a scene that has one large plane instead of the huge ground sphere.
2. **Named materials.** Let the scene file define a materials table by name, and reference them by string in shapes: `material: "red_matte"`. Requires a second pass over the parsed `SceneDesc` to resolve names. Think about error handling if a name is missing.
3. **Image output format.** Add an `--output-format {png,jpg}` flag to the `render` subcommand. Save PNG or JPEG accordingly. The `image` crate handles both — check its API.
4. **Fuzz test.** Use the `arbitrary` crate to generate random `SceneDesc` values, serialize them, parse them back, and assert equality. Catches roundtrip bugs if you ever add `Serialize`.
5. **Scene generator.** Write a small program (`examples/gen_random_scene.rs`) that creates a random scene (100 spheres with random positions, radii, materials) and writes it to RON. Render it. You've just rebuilt the famous RTIOW "final scene."

## What's next

At 500 spheres, your renderer crawls. Every ray tests against every sphere — that's O(rays × spheres) per frame, roughly 800 × 450 × 100 × 500 = 18 billion intersection tests. Most of those are wasted: the ray misses the sphere entirely.

**Day 28 adds a bounding volume hierarchy (BVH).** You'll build an axis-aligned bounding box type, implement the slab method for ray-AABB tests, then recursively partition your shapes into a binary tree where each node's AABB encloses its children. Ray traversal becomes O(log n) per ray instead of O(n). For 500 spheres that's a ~40x speedup on top of the parallelism you already have.

→ [Day 28 — BVH Acceleration](day-28.md)
