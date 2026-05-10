# Day 29 — Capstone Design

**Domain:** your choice • **Time:** 1.5 hours • **Difficulty:** design

## What you'll build

A project skeleton for a real, finishable piece of software that showcases what you've learned in the last 28 days. You won't write production logic today. The deliverable is four things:

1. A **track chosen**: Games, Database, or 3D Graphics.
2. A **README** with crisp acceptance criteria — what "done" looks like.
3. A **module scaffold** with `todo!()` stubs — the function signatures are real, the bodies are placeholders.
4. A **hello-world end-to-end path** — the skeleton compiles, runs, and produces *some* output. Even if that output is a single black pixel, an empty database file, or a blank menu screen.

Why spend a whole day designing instead of coding? Because tomorrow is your last day. If you spend the first hour of Day 30 deciding what to build, you won't ship. Today is a design exercise. By the end, you should be able to finish tomorrow without making any architectural decisions.

## What you'll learn

- Breaking a fuzzy "I want to build X" into concrete acceptance criteria
- Identifying the **happy path** for your project and mapping it end-to-end before any implementation
- Using `todo!()` strategically to stub out functions while keeping the compiler happy
- Reading your own code as an API surface — modules, types, public functions first; logic later
- Working backward from "done" rather than forward from "I have an empty `main`"

## Background

### Why the skeleton-first approach

The standard way to fail at a two-day project is to open a blank `main.rs` and start typing. Three hours in, you have a half-working implementation of one feature, no idea how the pieces fit, and no plan for what "shipping" looks like.

Skeleton-first reverses this:

1. Write the README describing the finished project.
2. Write module signatures that match the README.
3. `todo!()` every function body.
4. Write a tiny end-to-end test that exercises the hello-world path.
5. Only now, implement the function bodies one at a time, removing `todo!()`s.

The benefit: at any point in the next 24 hours, if you run out of time, you ship a *reduced-scope* version of the same project. Removing a feature means leaving a `todo!()` in place and not exercising that path — the rest still works. You never end up with a broken half-project.

### What `todo!()` actually is

`todo!()` is a macro in `std` that expands to `panic!("not yet implemented")`. It's typed `!` — the never type — which means it unifies with any return type. You can drop it into any function body and the compiler accepts the signature:

```rust
fn compute_thing(x: i32) -> Result<Vec<u8>, MyError> {
    todo!()
}
```

No unused-parameter warning (the parameter is technically used — `todo!()` is reached). No missing-return-value error. You get exactly what you want: "this function signature is real, the body isn't yet."

`unimplemented!()` is essentially the same macro with a different message. Use `todo!()` for work you plan to do; use `unimplemented!()` for things you may intentionally never implement.

### The happy path

"Happy path" is the one end-to-end scenario that proves your project is real. Not the most interesting scenario. Not the scenario that shows off the most features. The *simplest* flow that exercises every layer.

Examples:

- **Games track**: main menu → start game → take one action → exit cleanly. No fancy AI, no persistence, no polish.
- **Database track**: open empty DB → put one key → get it back → close. No crash recovery, no concurrency, no range queries.
- **Raytracer track**: load one scene → render one pixel → write one PNG. No BVH, no parallelism, no materials.

If the happy path works, you have a project. Everything else is polish.

## Step 1 — Pick a track

Three options. Pick the one that excites you most — motivation matters on a capstone.

### Track A: Games — A dungeon crawl with procedural rooms

**Pitch**: a text + terminal hybrid dungeon crawler. Procedurally generated rooms connected by doors, a player entity with inventory, a handful of enemy types, turn-based combat. Save/load between rooms. Ships as a single binary.

What you bring from the course:

- Days 1–3: character, inventory, save/load
- Day 7: command parser with aliases
- Day 11–12: terminal rendering with crossterm, fixed-timestep input loop
- Day 4–5: entity trait, typed inventory
- Day 2: enums for PlayerStatus, enemy states
- Day 13: tested core logic

What's new:

- Room generation (BSP split or grid-based)
- Room-to-room state transitions
- Line-of-sight / visibility

Suggested scope for capstone: 5 room types, 3 enemy types, 10 item types, save/load, win condition (reach final room), lose condition (HP reaches zero).

### Track B: Database — A persistent sorted KV store with range queries

**Pitch**: a log-structured KV store that also supports `range(start..end)` queries efficiently. Bitcask-style append log for writes, an in-memory B-tree index for ordered scans, on-disk snapshots, WAL for crash recovery. Single-binary CLI (`kvdb put foo 42`, `kvdb get foo`, `kvdb range a z`).

What you bring from the course:

- Day 15: binary I/O with magic bytes, CRC
- Day 16: serde + bincode with versioning
- Day 17: Bitcask-style log storage with HashMap index
- Day 18: B-tree with range queries
- Day 19: memmap2 zero-copy reads
- Day 20: WAL and crash recovery
- Day 21: concurrent access with sharded locks

What's new:

- Combining the log (append-only writes) with the B-tree (ordered index)
- Disk snapshots so startup isn't replay-from-scratch

Suggested scope for capstone: put/get/delete/range, restart with WAL replay, serialized background snapshot, command-line tool, at least one benchmark comparing against `sled` or `BTreeMap` for realistic workloads.

### Track C: 3D Graphics — A physically-based renderer with scene editing

**Pitch**: extend the raytracer into a tiny physically-based renderer. Add triangle meshes, texture mapping, environment map skies, and a simple rest framework for iterating scenes (not live — but you can edit a RON file, re-run, and see results in < 10 seconds). Ships as a single binary that produces PNG output.

What you bring from the course:

- Days 22–26: canvas, vec3, ray, hittable, camera, materials, rayon parallel render
- Day 27: RON scene files with tagged enums
- Day 28: BVH acceleration
- Day 14: threads and channels (for a progressive-render TUI preview)

What's new:

- Triangle primitive (`struct Triangle { a, b, c, normal }`) with ray-triangle intersection (Möller–Trumbore)
- OBJ mesh loader
- Textured materials (sample albedo from an image rather than a constant)
- Environment map sky (cube map or equirectangular)

Suggested scope for capstone: render a scene containing a textured OBJ mesh + a few procedural spheres with an environment-mapped sky, BVH enabled, rayon parallel, in under 10 seconds for a preview-quality image.

### Before you continue

Pick one. Write it down. The rest of today is shaped by this choice.

This tutorial is written agnostic to track — the steps work for all three. Where specifics matter, there are three parallel sections (marked **A**, **B**, **C**). Only read the one for your track.

## Step 2 — Set up the project

Fresh `cargo new`:

```bash
cd ~/rust-course
cargo new --bin capstone
cd capstone
```

Open `Cargo.toml` and add dependencies for your track:

### A: Games

```toml
[dependencies]
anyhow = "1"
thiserror = "1"
crossterm = "0.27"
rand = "0.8"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
clap = { version = "4", features = ["derive"] }
```

### B: Database

```toml
[dependencies]
anyhow = "1"
thiserror = "1"
clap = { version = "4", features = ["derive"] }
serde = { version = "1", features = ["derive"] }
bincode = "1"
crc32fast = "1"
memmap2 = "0.9"
parking_lot = "0.12"
```

### C: 3D Graphics

```toml
[dependencies]
anyhow = "1"
thiserror = "1"
clap = { version = "4", features = ["derive"] }
serde = { version = "1", features = ["derive"] }
ron = "0.8"
rand = "0.8"
rayon = "1"
image = "0.24"
obj = "0.10"
```

Run `cargo build` to confirm everything resolves:

```bash
cargo build
```

Expected output: downloads crates, compiles, says "Finished `dev` profile". If any crate fails to resolve, `cargo search <name>` to find the current version.

## Step 3 — Write the README first

Create `README.md` at the root. This is the most important file you'll write today.

Here's a template — adapt it to your track:

```markdown
# [Project Name]

One-sentence pitch describing what the binary does.

## Goals

- Primary goal 1 (the thing that "done" hinges on)
- Primary goal 2
- Primary goal 3

## Non-goals

- Explicitly out of scope 1
- Explicitly out of scope 2

## How to run

\`\`\`
cargo run --release -- <subcommand> [args]
\`\`\`

## Acceptance criteria

The project is "done" when all of these are true:

- [ ] Criterion 1 (concrete, testable)
- [ ] Criterion 2
- [ ] Criterion 3
- [ ] Criterion 4

## Architecture

Brief description of modules and their responsibilities.

## Notes

- Known limitations
- Future work
```

Here's a real example for each track. Use these as starting points; adjust.

### A: Games README

```markdown
# Crawl

A turn-based dungeon crawler in the terminal. You are `@`, you walk around,
you fight monsters, you grab loot, you try to reach the stairs on level 3.

## Goals

- Procedural dungeon generation (BSP, 3 levels deep)
- Turn-based combat vs 3 enemy types
- Inventory with 10 item types (potions, weapons, scrolls)
- Save/load mid-run
- Single-binary ship

## Non-goals

- Graphics beyond terminal cells
- Real-time input
- Pathfinding for enemies (they step toward player greedily)
- Multiplayer

## How to run

\`\`\`
cargo run --release
cargo run --release -- --load saves/last.json
\`\`\`

## Acceptance criteria

- [ ] Opening the binary shows a main menu (New / Load / Quit)
- [ ] "New" enters Level 1 with @ at a random walkable tile
- [ ] Arrow keys (and `hjkl`) move @; movement is blocked by walls
- [ ] Bumping an enemy attacks it; enemy HP and player HP both update
- [ ] Dead enemies disappear from the map
- [ ] Picking up an item adds to inventory; `i` shows inventory screen
- [ ] Stepping on `>` tile descends a level; Level 3 `>` wins the game
- [ ] Player at 0 HP shows a Game Over screen
- [ ] `S` from gameplay saves to saves/<timestamp>.json
- [ ] `cargo run -- --load <path>` restores exact game state
- [ ] No `unwrap()`, no `panic!()` outside tests

## Architecture

- `src/main.rs` — CLI entry, event loop
- `src/menu.rs` — main menu
- `src/game.rs` — `Game` state: level, player, turn counter
- `src/map.rs` — `Level` with tiles, entities, visibility
- `src/gen.rs` — BSP dungeon generator
- `src/entity.rs` — player + monster types, combat logic
- `src/item.rs` — item types, pickup/use
- `src/render.rs` — crossterm rendering
- `src/input.rs` — keystroke → action mapping
- `src/save.rs` — serde JSON persistence
- `tests/` — integration tests for gen, combat, save/load roundtrip
```

### B: Database README

```markdown
# kvdb

A persistent sorted KV store with range queries. Log-structured writes,
B-tree index, WAL for crash safety.

## Goals

- `put`, `get`, `delete` with persistence across restarts
- `range(start..end)` ordered scans
- WAL replay on restart (survives mid-write crash)
- Periodic compaction of the append log
- CLI for all operations

## Non-goals

- Network / client-server protocol
- Transactions / atomicity beyond single ops
- Secondary indexes
- Cross-platform atomic rename guarantees (Linux/macOS only)

## How to run

\`\`\`
cargo run --release -- --db-dir ./data put foo 42
cargo run --release -- --db-dir ./data get foo
cargo run --release -- --db-dir ./data range a z
cargo run --release -- --db-dir ./data bench --ops 100000
\`\`\`

## Acceptance criteria

- [ ] `put k v` writes to WAL, then log, returns success
- [ ] `get k` returns most recent value, reading from log via B-tree offset
- [ ] `delete k` writes a tombstone; subsequent `get` returns "not found"
- [ ] `range a..z` returns keys in sorted order, matching semantics
- [ ] Restart after in-process crash replays WAL and reconstructs index
- [ ] Compaction rewrites live records to new segment, deletes old
- [ ] `bench` subcommand reports ops/sec for mixed put/get
- [ ] All ops < 1ms p99 on 100k-entry workload (single thread, local disk)
- [ ] `cargo test` passes with no `#[ignore]`d tests

## Architecture

- `src/main.rs` — CLI entry, dispatch
- `src/store.rs` — top-level `KvStore` combining WAL, log, index
- `src/wal.rs` — append-only WAL with CRC, fsync
- `src/log.rs` — data log (Bitcask-style) with binary record format
- `src/index.rs` — in-memory B-tree with String keys, u64 offsets
- `src/record.rs` — binary encode/decode of records
- `src/compact.rs` — log compaction
- `tests/` — crash-recovery tests (child process, kill -9, restart)
```

### C: Graphics README

```markdown
# pbr-render

A physically-based path tracer with scene files, triangle meshes, textured
materials, and an environment-map sky. BVH accelerated, rayon parallel.

## Goals

- All of the Day 27-28 raytracer features (scenes, BVH, parallel)
- Triangle primitive with Möller–Trumbore intersection
- OBJ mesh loader
- Textured Lambertian material (albedo from image file)
- Equirectangular environment map sky
- Render 1920x1080 previews in under 10s with BVH+parallel

## Non-goals

- Realtime preview (single PNG output, no live display)
- Bidirectional path tracing
- Volumetric rendering
- Denoising (raw samples only)
- GPU acceleration

## How to run

\`\`\`
cargo run --release -- render scenes/mesh_demo.ron --width 1920 --height 1080 --samples 200 -o out.png
\`\`\`

## Acceptance criteria

- [ ] `cargo run --release -- render scenes/spheres.ron` reproduces Day 27 output
- [ ] A scene with a triangle mesh (OBJ loaded) renders correctly
- [ ] Textured Lambertian material shows distinguishable UV mapping
- [ ] Equirectangular HDR environment map illuminates the scene
- [ ] BVH covers all primitives (triangles + spheres)
- [ ] Rendering scales linearly to physical core count
- [ ] Image output is bit-identical across 1/2/4/8 threads with same seed
- [ ] 1920×1080 / 200 samples / 10k triangles renders in < 10s on the dev machine
- [ ] `cargo test` passes with no `#[ignore]`d tests

## Architecture

- `src/main.rs` — CLI entry
- `src/canvas.rs` — pixel buffer, PNG output
- `src/vec3.rs`, `src/ray.rs` — geometry primitives
- `src/hit.rs` — `Hittable` trait, `HitRecord`, `HittableList`
- `src/aabb.rs`, `src/bvh.rs` — AABB and BVH
- `src/camera.rs` — pinhole camera
- `src/material.rs` — Lambertian, Metal, Dielectric, TexturedLambertian
- `src/texture.rs` — 2D image texture sampling
- `src/sky.rs` — environment map
- `src/mesh/mod.rs` — Triangle primitive, OBJ loader
- `src/scene_file.rs` — RON descriptor + build
- `src/renderer.rs` — parallel render loop
- `tests/` — pixel-equality tests for known scenes
```

Fill out your README now. It should take 20–30 minutes. Tempting to skip, but this is the whole point of today — it's your contract with tomorrow-you.

## Step 4 — Scaffold the modules with `todo!()`

Now write the module structure and stub functions. Every module file, every public type, every public function — signatures are real, bodies are `todo!()`.

Here's the pattern for a function stub:

```rust
pub fn do_the_thing(input: &str) -> Result<u32, ThingError> {
    todo!("implement do_the_thing")
}
```

The `"implement do_the_thing"` argument is optional but helpful — if you hit a `todo!()` at runtime, the panic message tells you which one.

Below is a reasonable starting scaffold for each track. Create each file, copy the contents, and adapt as you see fit.

### A: Games scaffold

**`src/lib.rs`:**

```rust
pub mod entity;
pub mod game;
pub mod gen;
pub mod input;
pub mod item;
pub mod map;
pub mod menu;
pub mod render;
pub mod save;
```

**`src/map.rs`:**

```rust
use crate::entity::Entity;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tile {
    Wall,
    Floor,
    DoorOpen,
    DoorClosed,
    StairsDown,
}

pub struct Level {
    pub width: u32,
    pub height: u32,
    pub tiles: Vec<Tile>,
    pub entities: Vec<Entity>,
}

impl Level {
    pub fn new(width: u32, height: u32) -> Self {
        todo!("fill with walls, allocate entities vec")
    }

    pub fn tile_at(&self, x: u32, y: u32) -> Tile {
        todo!()
    }

    pub fn walkable(&self, x: u32, y: u32) -> bool {
        todo!()
    }
}
```

**`src/entity.rs`:**

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityKind {
    Player,
    Goblin,
    Skeleton,
    Troll,
}

#[derive(Debug, Clone)]
pub struct Entity {
    pub kind: EntityKind,
    pub x: u32,
    pub y: u32,
    pub hp: i32,
    pub max_hp: i32,
    pub damage: u32,
}

impl Entity {
    pub fn player_at(x: u32, y: u32) -> Self {
        todo!()
    }

    pub fn attack(&mut self, other: &mut Entity) {
        todo!("damage to other, print log line")
    }

    pub fn is_dead(&self) -> bool {
        self.hp <= 0
    }
}
```

**`src/game.rs`:**

```rust
use crate::map::Level;
use crate::entity::Entity;

pub struct Game {
    pub levels: Vec<Level>,
    pub current_level: usize,
    pub player: Entity,
    pub turn: u64,
    pub game_over: bool,
    pub won: bool,
}

impl Game {
    pub fn new_run(seed: u64) -> Self {
        todo!("generate 3 levels, place player in level 0")
    }

    pub fn step(&mut self, action: PlayerAction) {
        todo!("resolve action, then step all entities, then check game over")
    }
}

#[derive(Debug, Clone, Copy)]
pub enum PlayerAction {
    Move(i32, i32),
    Pickup,
    Descend,
    Wait,
}
```

**`src/gen.rs`:**

```rust
use crate::map::Level;

pub fn generate(width: u32, height: u32, depth: u32, seed: u64) -> Level {
    todo!("BSP split, carve rooms, connect with corridors, place stairs")
}
```

**`src/input.rs`:**

```rust
use crate::game::PlayerAction;

pub fn keystroke_to_action(c: char) -> Option<PlayerAction> {
    todo!()
}
```

**`src/render.rs`:**

```rust
use crate::game::Game;

pub fn draw_game(game: &Game) -> anyhow::Result<()> {
    todo!("clear screen, draw map tiles, draw entities, draw hud")
}

pub fn draw_menu() -> anyhow::Result<()> {
    todo!()
}

pub fn draw_inventory(game: &Game) -> anyhow::Result<()> {
    todo!()
}

pub fn draw_game_over(won: bool) -> anyhow::Result<()> {
    todo!()
}
```

**`src/menu.rs`, `src/item.rs`, `src/save.rs`:** similar — one or two types and the stub functions the README implies.

**`src/main.rs`:**

```rust
use anyhow::Result;

fn main() -> Result<()> {
    println!("crawl starting (hello-world skeleton)");
    // Tomorrow: show menu, dispatch to new/load/quit
    Ok(())
}
```

### B: Database scaffold

**`src/lib.rs`:**

```rust
pub mod compact;
pub mod index;
pub mod log;
pub mod record;
pub mod store;
pub mod wal;
```

**`src/record.rs`:**

```rust
use std::io::{Read, Write};

pub const MAGIC: [u8; 4] = *b"KVDB";

#[derive(Debug, Clone)]
pub enum Record {
    Put { key: Vec<u8>, value: Vec<u8> },
    Delete { key: Vec<u8> },
}

impl Record {
    pub fn encode<W: Write>(&self, w: &mut W) -> std::io::Result<u64> {
        todo!("magic bytes, tag, lengths, payload, CRC; return bytes written")
    }

    pub fn decode<R: Read>(r: &mut R) -> std::io::Result<Option<Record>> {
        todo!("read magic, tag, lengths, payload, verify CRC; None on EOF")
    }
}
```

**`src/wal.rs`:**

```rust
use std::path::Path;

pub struct Wal {
    // file handle, maybe BufWriter
}

impl Wal {
    pub fn open(path: &Path) -> anyhow::Result<Self> {
        todo!()
    }

    pub fn append(&mut self, record: &crate::record::Record) -> anyhow::Result<()> {
        todo!("encode, write, fsync")
    }

    pub fn replay<F>(path: &Path, mut f: F) -> anyhow::Result<()>
    where
        F: FnMut(crate::record::Record),
    {
        todo!()
    }

    pub fn truncate(&mut self) -> anyhow::Result<()> {
        todo!()
    }
}
```

**`src/log.rs`:**

```rust
use std::path::PathBuf;

pub struct Log {
    path: PathBuf,
    // segments: Vec<Segment>, etc.
}

pub struct RecordLocation {
    pub segment_id: u32,
    pub offset: u64,
    pub length: u32,
}

impl Log {
    pub fn open(dir: &std::path::Path) -> anyhow::Result<Self> {
        todo!()
    }

    pub fn append(&mut self, record: &crate::record::Record) -> anyhow::Result<RecordLocation> {
        todo!()
    }

    pub fn read(&self, loc: &RecordLocation) -> anyhow::Result<crate::record::Record> {
        todo!()
    }

    pub fn all(&self) -> impl Iterator<Item = anyhow::Result<(crate::record::Record, RecordLocation)>> + '_ {
        todo!();
        std::iter::empty()  // unreachable, satisfies return type
    }
}
```

**`src/index.rs`:**

```rust
use std::collections::BTreeMap;
use crate::log::RecordLocation;

pub struct Index {
    map: BTreeMap<Vec<u8>, RecordLocation>,
}

impl Index {
    pub fn new() -> Self {
        Self { map: BTreeMap::new() }
    }

    pub fn put(&mut self, key: Vec<u8>, loc: RecordLocation) {
        todo!()
    }

    pub fn delete(&mut self, key: &[u8]) {
        todo!()
    }

    pub fn get(&self, key: &[u8]) -> Option<&RecordLocation> {
        todo!()
    }

    pub fn range<R: std::ops::RangeBounds<Vec<u8>>>(&self, range: R) -> impl Iterator<Item = (&Vec<u8>, &RecordLocation)> {
        todo!();
        std::iter::empty()
    }
}
```

**`src/store.rs`:**

```rust
use std::path::Path;
use crate::index::Index;
use crate::log::Log;
use crate::wal::Wal;

pub struct KvStore {
    index: Index,
    log: Log,
    wal: Wal,
}

impl KvStore {
    pub fn open(dir: &Path) -> anyhow::Result<Self> {
        todo!("open log, open wal, replay wal into index, return store")
    }

    pub fn put(&mut self, key: &[u8], value: &[u8]) -> anyhow::Result<()> {
        todo!()
    }

    pub fn get(&self, key: &[u8]) -> anyhow::Result<Option<Vec<u8>>> {
        todo!()
    }

    pub fn delete(&mut self, key: &[u8]) -> anyhow::Result<()> {
        todo!()
    }

    pub fn range(&self, start: &[u8], end: &[u8]) -> anyhow::Result<Vec<(Vec<u8>, Vec<u8>)>> {
        todo!()
    }
}
```

**`src/main.rs`:**

```rust
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "kvdb")]
struct Cli {
    #[arg(long, default_value = "./data")]
    db_dir: PathBuf,
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    Put { key: String, value: String },
    Get { key: String },
    Delete { key: String },
    Range { start: String, end: String },
    Bench { #[arg(long, default_value_t = 10000)] ops: u32 },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    println!("kvdb starting, db_dir={:?}", cli.db_dir);
    // Tomorrow: open store, dispatch command.
    Ok(())
}
```

### C: Graphics scaffold

Copy your Day 28 raytracer as a starting point:

```bash
cp -r ../raytracer/src/* src/
cp -r ../raytracer/scenes scenes/
```

Then add new modules. **`src/texture.rs`:**

```rust
use crate::vec3::Vec3;
use image::RgbImage;
use std::path::Path;

pub struct ImageTexture {
    image: RgbImage,
}

impl ImageTexture {
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        todo!()
    }

    pub fn sample(&self, u: f32, v: f32) -> Vec3 {
        todo!("wrap uv, sample nearest or bilinear")
    }
}
```

**`src/sky.rs`:**

```rust
use crate::vec3::Vec3;
use image::RgbImage;
use std::path::Path;

pub enum Sky {
    Gradient,
    EnvMap { tex: RgbImage },
}

impl Sky {
    pub fn load_env(path: &Path) -> anyhow::Result<Self> {
        todo!()
    }

    pub fn sample(&self, direction: Vec3) -> Vec3 {
        todo!("direction → uv → sample")
    }
}
```

**`src/mesh/mod.rs`:**

```rust
use crate::aabb::Aabb;
use crate::hit::{HitRecord, Hittable};
use crate::material::Material;
use crate::ray::Ray;
use crate::vec3::Vec3;
use std::sync::Arc;

pub struct Triangle {
    pub a: Vec3,
    pub b: Vec3,
    pub c: Vec3,
    pub normal: Vec3,
    pub material: Arc<dyn Material>,
}

impl Hittable for Triangle {
    fn hit(&self, ray: &Ray, t_min: f32, t_max: f32) -> Option<HitRecord> {
        todo!("Möller–Trumbore")
    }

    fn bounding_box(&self) -> Aabb {
        todo!()
    }
}

pub fn load_obj(path: &std::path::Path, material: Arc<dyn Material>) -> anyhow::Result<Vec<Triangle>> {
    todo!("parse OBJ, build Triangle list with material clones")
}
```

Then extend `MaterialDesc` and `ShapeDesc` to add variants for `TexturedLambertian { texture: PathBuf }` and `Mesh { path: PathBuf, material: MaterialDesc }`, with `todo!()` in the `build` methods.

**`src/main.rs`**: leave it exactly as Day 28's version. The hello-world path is already working — rendering the old sphere scene.

## Step 5 — Prove the skeleton compiles

```bash
cargo build
```

Every warning about unused code is fine. Every warning about unused `todo!()` is fine. The only thing that matters is that **build succeeds**. If it doesn't, fix the errors now. A compile error on Day 29 is much cheaper than one on Day 30 at hour 10.

Next: run the hello-world happy path.

### A: Games

```bash
cargo run
```

Expected output:

```
crawl starting (hello-world skeleton)
```

It exits cleanly. Nothing panics. The binary is real.

### B: Database

```bash
cargo run -- put foo 42
```

Expected output:

```
kvdb starting, db_dir="./data"
```

It parses the CLI, runs main, exits. No `put` logic yet, but the skeleton works.

### C: Graphics

```bash
cargo run --release -- render scenes/three_spheres.ron
```

Expected output:

```
rendered in X.XXs
wrote out.png
```

The old scene still renders because we copied Day 28's code wholesale. New features (`triangle`, `texture`, `sky`) are stubbed but not yet wired into the scene loader, so they don't break the existing happy path.

## Step 6 — One real test

Even today, write one integration test. It'll be a `todo!()`-free smoke test that the binary starts and stops cleanly.

Create `tests/smoke.rs`:

```rust
#[test]
fn binary_runs_with_help() {
    // Just confirms the binary compiles and exits 0.
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_capstone"))
        .arg("--help")
        .output();
    // Binary might not even have a --help yet; if it errors, that's OK.
    // We just want the link-and-load path to work.
    assert!(output.is_ok() || output.is_err());
}
```

```bash
cargo test
```

The test passes. Whenever you refactor tomorrow, you'll want a quick check that nothing fundamental broke — a smoke test is 30 seconds of setup and saves real debugging time.

## Step 7 — Plan tomorrow

Make a prioritized task list. Example:

```markdown
# Day 30 plan

## Must ship
1. implement X
2. implement Y
3. wire X+Y into main.rs
4. smoke test still passes
5. update README checklist items to ticked

## Should ship
6. ...

## Nice to have
7. ...

## Not shipping (explicitly cut)
- Feature Z (out of scope, noted in README non-goals)
```

Put this in `NEXT.md` at the project root. Tomorrow-you opens `NEXT.md`, picks item 1, implements it, runs the test, moves to item 2. No decisions, just execution.

## Common pitfalls

### The README is vague

If "should be fast" or "should handle errors gracefully" is in your acceptance criteria, you don't have acceptance criteria. Replace with: "1920×1080 scene renders in <10s on my machine" or "all ops return a typed error, no `panic!()` outside tests." Specific. Testable. Can check off.

### Scope explosion

Every skeleton feels a little too small. That's the right feeling. If you find yourself adding a fifth subsection to your goals list, stop. The whole point of today is resisting scope creep. Tomorrow-you will thank you.

### Stubbed iterator returning `todo!()`

The `std::iter::empty()` trick in the scaffold is a workaround — `todo!()` returns the never type, but the compiler can't unify that with `impl Iterator<Item = T>` in a way that lets you leave the function returning something traversable. Using `todo!(); std::iter::empty()` is fine. The `todo!()` panics at runtime if called; the `empty()` is only there for the compiler's sake.

Alternatively, `fn thing(...) -> Box<dyn Iterator<Item = T>> { todo!() }` works cleanly — `Box<dyn Iterator>` unifies with the never type via the return-type coercion.

### "But I want to start coding now!"

Coding today means shipping less tomorrow. The skeleton phase doesn't feel like progress, but it's the difference between a project that ships and one that doesn't. Trust the process for 24 hours.

### Your module split feels arbitrary

It probably is, and that's fine — modules are scaffolding, not load-bearing decisions. You can move code between files freely as implementations settle. What matters is that *something* is in each file so your first instinct tomorrow is "edit this file" not "where should this go?"

## What you learned

- **Skeleton-first** means writing the README, module signatures, and smoke test before any real logic. This lets you ship a reduced-scope version at any point.
- **Acceptance criteria** must be specific and testable. "Survives crash" is bad; "kill -9 during put, restart, range returns all prior keys" is good.
- **`todo!()`** is your friend: it's typed `!`, unifies with any return type, and panics with a location-rich message if reached at runtime.
- **The happy path** is the dumbest end-to-end flow that touches every layer. Make that work first. Everything else is polish.
- **Goals *and* non-goals**. Writing down what you're *not* building is as important as what you are.

## Exercises

There are no exercises today — tomorrow is the exercise. Instead, these are reflection questions to help you commit to the plan:

1. **If you had only 4 hours tomorrow**, which of your acceptance criteria would you drop? Which would you keep? Write the answer next to the checkboxes in your README.
2. **What's the one thing most likely to go wrong tomorrow?** A subtle lifetime issue in the index? A dependency that won't install? A flaky crate? Note it in `NEXT.md`. Forewarned is forearmed.
3. **How will you know you're ahead of schedule?** Pick a concrete milestone: "by hour 2, item 3 is done and the smoke test still passes." If you're behind at that checkpoint, cut from the "Should ship" list.
4. **How will you know you're about to get stuck?** Usually it's: you've been staring at the same function for 45 minutes. Rule for tomorrow: if that happens, commit whatever state, push, go for a walk, come back with fresh eyes.

## What's next

Tomorrow you finish. The day is structured around removing `todo!()`s one at a time, in priority order, running the smoke test between each, and ticking items off the README. By the end of Day 30 you will have shipped a real project that showcases every major Rust skill from the course — ownership, traits, generics, lifetimes, iterators, error handling, concurrency, serde, and whatever domain-specific craft your track demanded.

→ [Day 30 — Capstone Ship](day-30.md)
