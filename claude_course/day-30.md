# Day 30 — Capstone Ship

**Domain:** your choice • **Time:** 2–4 hours • **Difficulty:** varies

## What you'll build

The finished project. Today you remove `todo!()`s one at a time, in priority order, and check off acceptance criteria until the README's checklist is all ticked. No architecture decisions — those are done. Just execution.

By end of day you will have:

- A working binary that meets every "Must ship" criterion from yesterday's README.
- A clean test suite that passes with no `#[ignore]`d tests.
- Zero `unwrap()` in user-facing code paths (tests are fine; internal asserts that can't fire are fine).
- `cargo clippy` clean.
- A commit history that lets you retrace what you did.

This is a different kind of tutorial than the first 29. There's no new Rust concept to teach. Instead, today is about the *rhythm* of shipping — a practice that's as important as any syntax. The steps below are track-agnostic. Where track-specific guidance helps, there are callouts marked **A**, **B**, **C**.

## What you'll learn

- The ship-day loop: pick a `todo!()`, implement, run test, commit, repeat
- Triage: how to cut scope under time pressure without breaking what works
- Reading `cargo clippy` and `cargo fmt` and caring about their output
- Removing `unwrap()` systematically via `?` and `anyhow::Context`
- Writing commit messages that tomorrow-you will thank today-you for
- Knowing when to stop (hint: "it's perfect" is never the reason)

## Background

### The ship-day loop

Here's the exact loop you'll repeat today:

1. Open `NEXT.md`, pick the top unchecked item.
2. Find the `todo!()` it corresponds to.
3. Implement the function. If you need a helper, add a signature for the helper and `todo!()` it.
4. Run the tests that touch this function. They should pass (or you write a new one).
5. Run the full test suite.
6. `cargo clippy` — fix the easy lints now while the change is fresh.
7. Commit. One or two lines in the message. Tick the item in `NEXT.md`.
8. Go back to step 1.

Every step is cheap individually. The commit step matters even if you're not pushing anywhere — it's a checkpoint. When a later change breaks something, `git diff HEAD~1` tells you instantly what you just did. Without commits you're stuck doing mental diffs, which eat hours.

### Triage under pressure

Some point in the middle of today you're going to fall behind. That's normal. It doesn't mean anything about you. It means the project is harder than you estimated, which is true of every project ever.

The wrong move: pick up the pace, cut corners, skip tests.

The right move: **update the README.** Move items from "Must ship" down to "Should ship" or "Not shipping." Commit. Now you're back on schedule. The project is smaller, but it's also shippable. A smaller project that works beats a larger one that doesn't.

Rule: never skip a test or a commit to go faster. Those two things are what *make* you faster across the whole day.

### The "one more thing" trap

The closer to done, the stronger the pull to add "just one more thing." Resist. Every addition after the acceptance criteria are met is risk without reward — a feature you haven't scoped, tested, or commit-isolated. If something seems critical to add, update the README first. If writing it in the acceptance criteria makes you realize it can wait, it can wait.

Your exit criterion for today: the README checkboxes are all ticked. Not "the project is perfect." Done is a checklist.

## Setting up

Before you start, make sure yesterday's skeleton still builds:

```bash
cd capstone
cargo build
cargo test
```

Open three files side by side if your editor allows:

1. `README.md` — the acceptance criteria.
2. `NEXT.md` — the task list.
3. The module you're about to edit.

Have a timer. 25 minutes on, 5 minutes off is a reasonable rhythm (pomodoro). Even if you don't do fixed intervals, periodically step away from the screen — shipping is a marathon, not a sprint, and fresh eyes catch bugs that tired ones miss.

## Step 1 — The first real implementation

Pick item #1 from `NEXT.md`. Implement it. Here are concrete examples of what that looks like for each track.

### A: Games — First item: `Level::new` and `Level::tile_at`

Open `src/map.rs`. Replace:

```rust
impl Level {
    pub fn new(width: u32, height: u32) -> Self {
        todo!("fill with walls, allocate entities vec")
    }

    pub fn tile_at(&self, x: u32, y: u32) -> Tile {
        todo!()
    }
    // ...
}
```

With:

```rust
impl Level {
    pub fn new(width: u32, height: u32) -> Self {
        let tiles = vec![Tile::Wall; (width * height) as usize];
        Self {
            width,
            height,
            tiles,
            entities: Vec::new(),
        }
    }

    pub fn tile_at(&self, x: u32, y: u32) -> Tile {
        let idx = (y * self.width + x) as usize;
        self.tiles[idx]
    }

    pub fn walkable(&self, x: u32, y: u32) -> bool {
        matches!(
            self.tile_at(x, y),
            Tile::Floor | Tile::DoorOpen | Tile::StairsDown
        )
    }

    pub fn set_tile(&mut self, x: u32, y: u32, tile: Tile) {
        let idx = (y * self.width + x) as usize;
        self.tiles[idx] = tile;
    }
}
```

Write a test in the same file:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_level_is_all_walls() {
        let level = Level::new(10, 5);
        for y in 0..5 {
            for x in 0..10 {
                assert_eq!(level.tile_at(x, y), Tile::Wall);
            }
        }
    }

    #[test]
    fn walkable_rules() {
        let mut level = Level::new(3, 3);
        level.set_tile(1, 1, Tile::Floor);
        assert!(level.walkable(1, 1));
        assert!(!level.walkable(0, 0));
    }
}
```

```bash
cargo test level
```

Expected output: `2 passed`. Commit:

```bash
git add -A && git commit -m "Level::new, tile_at, walkable, set_tile"
```

Tick item #1 in `NEXT.md`. Next.

### B: Database — First item: `Record::encode` and `Record::decode`

The binary record format from Day 15 is where you start. Open `src/record.rs`. Replace stubs with concrete impls:

```rust
use std::io::{Read, Write};
use crc32fast::Hasher;

pub const MAGIC: [u8; 4] = *b"KVDB";
const TAG_PUT: u8 = 1;
const TAG_DELETE: u8 = 2;

impl Record {
    pub fn encode<W: Write>(&self, w: &mut W) -> std::io::Result<u64> {
        let mut hasher = Hasher::new();
        let mut written = 0u64;

        w.write_all(&MAGIC)?;
        hasher.update(&MAGIC);
        written += 4;

        match self {
            Record::Put { key, value } => {
                let key_len = key.len() as u32;
                let val_len = value.len() as u32;
                let tag = [TAG_PUT];
                let kl = key_len.to_le_bytes();
                let vl = val_len.to_le_bytes();
                w.write_all(&tag)?; hasher.update(&tag);
                w.write_all(&kl)?;  hasher.update(&kl);
                w.write_all(&vl)?;  hasher.update(&vl);
                w.write_all(key)?;  hasher.update(key);
                w.write_all(value)?; hasher.update(value);
                written += 1 + 4 + 4 + key.len() as u64 + value.len() as u64;
            }
            Record::Delete { key } => {
                let key_len = key.len() as u32;
                let tag = [TAG_DELETE];
                let kl = key_len.to_le_bytes();
                w.write_all(&tag)?; hasher.update(&tag);
                w.write_all(&kl)?;  hasher.update(&kl);
                w.write_all(key)?;  hasher.update(key);
                written += 1 + 4 + key.len() as u64;
            }
        }

        let crc = hasher.finalize().to_le_bytes();
        w.write_all(&crc)?;
        written += 4;
        Ok(written)
    }

    pub fn decode<R: Read>(r: &mut R) -> std::io::Result<Option<Record>> {
        let mut magic = [0u8; 4];
        match r.read_exact(&mut magic) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(None),
            Err(e) => return Err(e),
        }
        if magic != MAGIC {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "bad magic bytes",
            ));
        }

        let mut hasher = Hasher::new();
        hasher.update(&magic);

        let mut tag = [0u8; 1];
        r.read_exact(&mut tag)?;
        hasher.update(&tag);

        let record = match tag[0] {
            TAG_PUT => {
                let mut kl = [0u8; 4];
                let mut vl = [0u8; 4];
                r.read_exact(&mut kl)?; hasher.update(&kl);
                r.read_exact(&mut vl)?; hasher.update(&vl);
                let key_len = u32::from_le_bytes(kl) as usize;
                let val_len = u32::from_le_bytes(vl) as usize;
                let mut key = vec![0u8; key_len];
                let mut value = vec![0u8; val_len];
                r.read_exact(&mut key)?;   hasher.update(&key);
                r.read_exact(&mut value)?; hasher.update(&value);
                Record::Put { key, value }
            }
            TAG_DELETE => {
                let mut kl = [0u8; 4];
                r.read_exact(&mut kl)?; hasher.update(&kl);
                let key_len = u32::from_le_bytes(kl) as usize;
                let mut key = vec![0u8; key_len];
                r.read_exact(&mut key)?; hasher.update(&key);
                Record::Delete { key }
            }
            other => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("unknown tag {other}"),
                ));
            }
        };

        let mut crc_bytes = [0u8; 4];
        r.read_exact(&mut crc_bytes)?;
        let found = u32::from_le_bytes(crc_bytes);
        let expected = hasher.finalize();
        if found != expected {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "CRC mismatch",
            ));
        }

        Ok(Some(record))
    }
}
```

Test:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_put() {
        let rec = Record::Put {
            key: b"foo".to_vec(),
            value: b"hello world".to_vec(),
        };
        let mut buf = Vec::new();
        rec.encode(&mut buf).unwrap();
        let decoded = Record::decode(&mut &buf[..]).unwrap().unwrap();
        match decoded {
            Record::Put { key, value } => {
                assert_eq!(key, b"foo");
                assert_eq!(value, b"hello world");
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn empty_input_returns_none() {
        let buf: &[u8] = &[];
        assert!(Record::decode(&mut &buf[..]).unwrap().is_none());
    }

    #[test]
    fn corrupt_crc_rejected() {
        let rec = Record::Delete { key: b"x".to_vec() };
        let mut buf = Vec::new();
        rec.encode(&mut buf).unwrap();
        let last = buf.len() - 1;
        buf[last] ^= 0xFF;
        assert!(Record::decode(&mut &buf[..]).is_err());
    }
}
```

```bash
cargo test record
```

Expected output: `3 passed`. Commit.

### C: Graphics — First item: `Triangle::hit` (Möller–Trumbore)

The new-feature work for the graphics track hinges on triangles. Get that right first. Open `src/mesh/mod.rs`:

```rust
impl Hittable for Triangle {
    fn hit(&self, ray: &Ray, t_min: f32, t_max: f32) -> Option<HitRecord> {
        const EPS: f32 = 1e-8;
        let edge1 = self.b - self.a;
        let edge2 = self.c - self.a;
        let h = ray.direction.cross(edge2);
        let det = edge1.dot(h);
        if det.abs() < EPS {
            return None;  // Ray parallel to triangle
        }
        let inv_det = 1.0 / det;
        let s = ray.origin - self.a;
        let u = inv_det * s.dot(h);
        if u < 0.0 || u > 1.0 {
            return None;
        }
        let q = s.cross(edge1);
        let v = inv_det * ray.direction.dot(q);
        if v < 0.0 || u + v > 1.0 {
            return None;
        }
        let t = inv_det * edge2.dot(q);
        if t < t_min || t > t_max {
            return None;
        }
        let point = ray.at(t);
        Some(HitRecord::new(
            point,
            self.normal,
            t,
            ray,
            self.material.clone(),
        ))
    }

    fn bounding_box(&self) -> Aabb {
        let min = Vec3::new(
            self.a.x.min(self.b.x).min(self.c.x),
            self.a.y.min(self.b.y).min(self.c.y),
            self.a.z.min(self.b.z).min(self.c.z),
        );
        let max = Vec3::new(
            self.a.x.max(self.b.x).max(self.c.x),
            self.a.y.max(self.b.y).max(self.c.y),
            self.a.z.max(self.b.z).max(self.c.z),
        );
        // Pad slightly to avoid degenerate AABBs for axis-aligned triangles.
        let pad = Vec3::new(1e-4, 1e-4, 1e-4);
        Aabb::new(min - pad, max + pad)
    }
}
```

You'll need `Vec3::cross` and `Vec3::dot` — add them to `vec3.rs` if you don't have them yet:

```rust
impl Vec3 {
    pub fn dot(self, other: Vec3) -> f32 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    pub fn cross(self, other: Vec3) -> Vec3 {
        Vec3::new(
            self.y * other.z - self.z * other.y,
            self.z * other.x - self.x * other.z,
            self.x * other.y - self.y * other.x,
        )
    }
}
```

Quick test:

```rust
// in tests module inside src/mesh/mod.rs
#[cfg(test)]
mod tests {
    use super::*;
    use crate::material::Lambertian;

    #[test]
    fn straight_on_ray_hits_triangle() {
        let mat = Lambertian::new(Vec3::new(1.0, 0.0, 0.0));
        let t = Triangle {
            a: Vec3::new(-1.0, -1.0, -1.0),
            b: Vec3::new(1.0, -1.0, -1.0),
            c: Vec3::new(0.0, 1.0, -1.0),
            normal: Vec3::new(0.0, 0.0, 1.0),
            material: mat,
        };
        let ray = Ray::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.0, 0.0, -1.0));
        let hit = t.hit(&ray, 0.001, 100.0).expect("hit");
        assert!((hit.t - 1.0).abs() < 1e-4);
    }

    #[test]
    fn grazing_ray_misses() {
        let mat = Lambertian::new(Vec3::new(1.0, 0.0, 0.0));
        let t = Triangle {
            a: Vec3::new(-1.0, -1.0, -1.0),
            b: Vec3::new(1.0, -1.0, -1.0),
            c: Vec3::new(0.0, 1.0, -1.0),
            normal: Vec3::new(0.0, 0.0, 1.0),
            material: mat,
        };
        let ray = Ray::new(Vec3::new(5.0, 0.0, 0.0), Vec3::new(0.0, 0.0, -1.0));
        assert!(t.hit(&ray, 0.001, 100.0).is_none());
    }
}
```

```bash
cargo test triangle
```

Expected output: `2 passed`. Commit.

## Step 2 — Keep going

Items 2, 3, 4, … — the rest of the morning is this rhythm. Implement, test, commit. At every step run the full test suite:

```bash
cargo test
```

If a test that wasn't touching your code breaks, stop. You've introduced a regression. Don't move on until it's green again. Usually the fix is fast (a change of signature that broke a caller you forgot about). If it's not fast, `git diff` tells you what you changed; revert selectively until the test goes green again, then reapply more carefully.

### When to write new tests

Every major function deserves a test. You don't need five tests per function — one that exercises the happy path and one that exercises the main failure mode is usually enough. Tests here are a progress ratchet: they tell future-you which behaviors are locked in.

Don't test private internals unless they're complex enough to deserve it (binary record format = yes; a one-line getter = no).

### When not to refactor

Roughly three hours in, you'll notice a cleaner way to structure something. *Do not refactor.* Note it in `NEXT.md` under "Not shipping" or "Future work" and keep going. Refactoring mid-ship is how projects die. Once the checklist is done, you can refactor to your heart's content — but by then you have a shipped project as a safety net.

## Step 3 — The halfway checkpoint

When half your time is gone, stop and audit.

1. Count unchecked "Must ship" items. If more than half remain, you're behind.
2. If behind, open the README. Move one or two items from "Must ship" to "Should ship." Update non-goals.
3. Commit that change. `README: cut scope X`.
4. Continue.

Cutting scope at the halfway mark is a sign of good judgment, not of failure. The worst outcome today is not "I cut a feature," it's "I ran out of time with 80% of features half-implemented."

## Step 4 — `cargo clippy` pass

Once the must-ship features work end-to-end, run:

```bash
cargo clippy --all-targets
```

Expect warnings. Ignore the `cognitive_complexity` ones if they're fake-positive. Fix the real ones:

- `redundant_clone` — delete the `.clone()` and let the borrow checker tell you what you actually need.
- `needless_collect` — skip a `.collect::<Vec<_>>()` between two iterators.
- `single_match` — replace `match x { Foo => bar, _ => () }` with `if let Foo = x { bar }`.
- `useless_let_if_seq` — common in code you wrote fast; clean up.

Don't try to get to zero warnings. Get to **zero unexpected warnings.** If clippy suggests something and you have a reason not to apply it, add `#[allow(clippy::...)]` with a comment explaining why. That's healthy code hygiene.

```bash
cargo fmt
```

Run this too. Every now and then. It changes whitespace in uncommitted files; commit the formatting change separately from logic changes, so `git blame` is still useful.

## Step 5 — Remove `unwrap()` from user paths

Search for all `.unwrap()` calls outside test code:

```bash
rg "\.unwrap\(\)" src/ --type rust
```

(Use `grep -rn '\.unwrap()' src/` if you don't have `rg`.)

For each one, ask: "Could this realistically happen under user input or environment?" If yes, it has to become a `?` or a `context`:

```rust
// Bad: if the file doesn't exist, we crash.
let content = std::fs::read_to_string("config.toml").unwrap();

// Good: user sees a useful error, exit code is non-zero.
let content = std::fs::read_to_string("config.toml")
    .with_context(|| "reading config.toml")?;
```

For `.unwrap()`s that genuinely can't fail (like `b"fixed"[0]` or indexing a vec you just pushed to), leave them, or prefer `.expect("some invariant message")`, which gives a better panic message if the invariant somehow gets violated.

Tests keep `.unwrap()`. That's idiomatic.

## Step 6 — Run the acceptance checklist

Open the README. For each unchecked item, manually walk through the exact scenario and confirm it works. For each one that does, tick it. For each one that doesn't, either fix it (if you're within time budget) or cut it (if you're not).

### A: Games — walkthrough

Run `cargo run --release`. Does the main menu appear? Can you press "N" for new game? Does the dungeon appear? Do arrow keys move you? Does walking into a wall block? Walking into a goblin — does it attack? Does the goblin die after enough hits? Does `i` open the inventory? Does `S` save? Does `cargo run -- --load saves/latest.json` restore state? Does descending on `>` take you to level 2? Does reaching level 3's exit win? Does dying trigger game over?

If every answer is "yes", every checkbox ticks.

### B: Database — walkthrough

```bash
cargo run --release -- --db-dir /tmp/kvdb1 put foo 42
cargo run --release -- --db-dir /tmp/kvdb1 get foo
# expect: 42
cargo run --release -- --db-dir /tmp/kvdb1 delete foo
cargo run --release -- --db-dir /tmp/kvdb1 get foo
# expect: not found
cargo run --release -- --db-dir /tmp/kvdb1 put a 1
cargo run --release -- --db-dir /tmp/kvdb1 put b 2
cargo run --release -- --db-dir /tmp/kvdb1 put c 3
cargo run --release -- --db-dir /tmp/kvdb1 range a c
# expect: a=1, b=2 (c is the exclusive end)
```

Crash test — start a `put` in a shell, kill the process with Ctrl-C mid-operation, restart, `get` the key; it should either have the old value or the new, never a corrupted partial record.

```bash
cargo run --release -- --db-dir /tmp/kvdb1 bench --ops 10000
# expect: XXX ops/sec, p99 latency in ms
```

### C: Graphics — walkthrough

```bash
cargo run --release -- render scenes/three_spheres.ron -o day27.png
# image diff with Day 27 output (or spot-check)

cargo run --release -- render scenes/mesh_demo.ron -o mesh.png
# a mesh-containing scene renders

cargo run --release -- render scenes/textured.ron -o textured.png
# textured material is visible
```

Run the bench in parallel with 1/2/4/8 threads and confirm linear scaling. Output bit-identity across thread counts means your random seeding per pixel still works (Day 26).

## Step 7 — The "stop" decision

Two things cause the day to end:

1. **All "Must ship" items are checked.** You've shipped. Stop.
2. **Time's up** (pick a hard stop at the start of the day — say, 10 PM, or 4 hours in). Whatever's done is done. Update the README one last time with what shipped vs. what didn't. Commit.

In either case, don't keep going. The codebase at "done" is far more valuable than the codebase at "almost done plus three half-written features." Future-you will have time later for polish. Today-you is on the clock.

Write a closing commit:

```bash
git add -A && git commit -m "ship: capstone complete"
```

And a closing entry at the top of the README:

```markdown
## Status

Shipped on <today's date>. Acceptance criteria: X/Y met. See Notes for any
items deferred.
```

## Common pitfalls

### You start the day by tinkering with the build

"Let me upgrade this dependency first" or "Let me re-organize the module layout." Don't. Yesterday's skeleton compiles; that's the premise today runs on. Start by implementing the first function on your list, nothing else.

### The test suite starts failing randomly

Flaky tests on ship day are a nightmare. Usually the cause is one of:

- Tests depend on filesystem state (they write to `./data`); one test leaves files the next reads.
- Tests depend on time (a sleep, a timeout, anything with wall-clock).
- Tests depend on global state (a lazy_static registry, an env var, a port).

Quick fixes: use `tempfile::tempdir()` for filesystem tests (you'll need to `cargo add tempfile --dev`). For time-dependent tests, use `Duration::from_millis(...)` with values large enough for your slowest machine. For globals, ideally don't — but if you must, serialize with `#[serial_test::serial]` or a mutex.

### `cargo clippy` is producing hundreds of warnings

Filter to the important ones:

```bash
cargo clippy -- -W clippy::pedantic -A clippy::module_name_repetitions
```

Or just `cargo clippy 2>&1 | head -40` and triage the first screen. Perfect clippy is a two-day project on its own — today you want "no embarrassing lints," not "zero warnings."

### You find a genuine architectural bug at hour 6

Happens. Your first instinct is "let me refactor the whole thing." Don't. Instead:

1. Add a comment at the site of the bug explaining what's wrong.
2. Add a `TODO.md` entry: "architecture: X doesn't actually work as designed; fixed by doing Y."
3. Write a workaround that's ugly but correct.
4. Tick the acceptance criterion.

After shipping you can refactor. Today you're shipping. The workaround is fine.

### You want to push the scope back up at hour 7

"Actually I think I can add feature Z in time." No. You moved it to "Not shipping" for a reason, and that reason is still true. If you really think you can add it, add it to "Future work" and bookmark it for next weekend.

### You're running out of time on a single function

Rule: if a function takes more than 45 minutes of focused work, it's the wrong function. Either the scope is too big (split it into three `todo!()`s), the approach is wrong (stepping back and rethinking usually saves time net), or the function belongs in the "Not shipping" bucket. Make a decision and move on.

## What you learned

- **The ship-day loop**: pick, implement, test, commit. Repeat. Every step is cheap; the repetition is what ships.
- **Triage is a skill.** Cutting scope mid-day is a sign of good judgment, not failure.
- **Commit often.** Not to push anywhere — as a checkpoint for yourself. `git diff HEAD~1` is free; mental diffs cost hours.
- **`cargo clippy`** and **`cargo fmt`** aren't bureaucracy. They're quick wins for code quality that cost you almost nothing when done incrementally.
- **Removing `.unwrap()`** from user paths is about turning crashes into useful errors. `anyhow::Context` is the shortest route.
- **"Done" is a checklist.** Not "perfect," not "complete," not "everything I imagined." The README checkboxes, all ticked. That's it.

## You're done

You did it.

Thirty days ago you wrote "hello world" in `main.rs`. Today you shipped a real piece of software. Along the way:

- Days 1–7 — ownership, borrowing, enums, traits, error handling. The non-negotiable fundamentals. Python doesn't make you earn these, but Rust does, and you did.
- Days 8–14 — closures, lifetimes, smart pointers, tests, concurrency. The stuff that separates "I can read Rust" from "I can write Rust."
- Days 15–21 — binary I/O, serde, storage engines, WAL, concurrent access. A database from scratch.
- Days 22–28 — a ray tracer from first principles: canvas, rays, materials, parallelism, scene files, BVH.
- Days 29–30 — design, ship, iterate.

The skills you have now:

- You can take an unfamiliar crate, read its types, and use it correctly.
- You can diagnose lifetime errors without guessing.
- You can write concurrent code that's actually correct.
- You can build on-disk formats that survive crashes.
- You can parallelize CPU-bound work and measure your speedup.
- You can ship a project against a deadline.

That last one is the one that compounds. You now have a repeatable *shipping practice*. Every future project is easier than this one.

### What to do next

1. **Put your capstone on GitHub.** Real code in the world beats code on disk. Add a LICENSE (MIT is fine). Write a good commit to the README.
2. **Contribute to a Rust project.** Pick something you used during this course — `serde`, `rayon`, `crossterm`, `image`. Open an issue, fix a bug, submit a PR. You'll learn more about Rust from one PR review than from a week of solo coding.
3. **Rewrite something.** Take a Python project you maintain, rewrite it in Rust. You'll hit problems the course didn't cover, which means you'll learn what the course couldn't teach.
4. **Read real code.** Clone `ripgrep`, `fd`, `tokio`, `hyper`. Read source. Note patterns you don't understand, look them up. A working Rust library is the best textbook after the basics.

### A closing note

You learned Rust. That's uncommon. Most people who try, stop. You kept going, for thirty days, and shipped something.

The feeling at the end of a day like today — code on disk, tests green, `cargo clippy` clean, README ticked — is the whole point of software. It doesn't get bigger than this. What changes is the *size* of the project, not the feeling at its end. You now know what that feeling is. Go make more of it.

→ (no more links; you've reached the end of the course)
