# Day 16 — Serde, Bincode, and Versioned Records

**Domain:** databases • **Time:** 2 hours • **Difficulty:** medium

## What you'll build

A versioned save-file system built on top of yesterday's record format. Instead of writing payload bytes by hand, you'll use **serde** with **bincode** to serialize typed Rust structs into and out of the records. Then you'll practice schema evolution — adding fields, removing fields, migrating between versions — without breaking old files.

## What you'll learn

- **`serde`** — what it is, why every serious Rust project uses it
- **`#[derive(Serialize, Deserialize)]`** and the derive ecosystem
- **`bincode`** — compact binary serialization that plays with yesterday's record format
- **Schema evolution**: `#[serde(default)]`, `#[serde(skip)]`, `#[serde(rename)]`
- **Tagged enums** for version discrimination
- Migration functions between schema versions
- When serde *won't* save you: renamed fields, type changes, reordering tuple structs

## Background

### What serde solves

In Python, you call `json.dumps(obj)` and it Just Works — because Python is dynamic, the runtime can walk the object graph and decide what to emit.

Rust is statically typed. There's no runtime object inspection. So how do you generically serialize `struct Player { name: String, hp: u32 }`?

**serde** is the answer. It's a framework with two halves:

1. **The `Serialize` and `Deserialize` traits.** A type that implements these describes how to visit its fields (for serialization) or build itself from visited fields (for deserialization), *independently of any specific format*.
2. **Format crates.** `serde_json`, `bincode`, `ron`, `serde_yaml`, `toml`, `rmp-serde` (MessagePack), and dozens of others. Each one knows how to drive a `Serialize` implementation to produce its output format, and parse input into a `Deserialize` implementation.

You write `#[derive(Serialize, Deserialize)]` on your struct *once*. Now it works with every format. Swap between JSON (for humans) and bincode (for speed) by changing one line.

### What bincode does

`bincode` is the minimalist binary format for serde. Design goals:

- **Compact.** Integers use fixed-width little-endian (no text, no length-prefixing for numbers). Strings and vectors get a 4- or 8-byte length prefix.
- **Fast.** No schema, no field names on the wire. Just fields in order.
- **Stable output.** Same input, same bytes, every time.

Tradeoff: there's no schema in the file. If you change the struct, old bytes are garbage. That's why we pair bincode with **explicit versioning** — we control the schema manually.

As of 2025 the bincode crate has two major lines: `bincode = "1"` uses the classic serde-based API, and `bincode = "2"` introduced a new encoding API that can be used without serde. We'll use v1 here for simplicity and familiarity; the v2 API is slightly different but the ideas are identical.

### Schema evolution

If you ship a game, save its files, and then release a patch that adds a new field to `PlayerSave`, what happens to existing save files?

- **No handling**: deserialization fails. Players lose their save. Angry tweets.
- **With `#[serde(default)]`**: missing fields default to `Default::default()` on the type. Old files load; the new field gets `0` or `""` or whatever the default is.
- **With a version tag**: explicitly dispatch on the file's version. Run a migration function that rewrites v1 data into v2.

We'll practice all three today.

### The derive macro

```rust
#[derive(Serialize, Deserialize)]
struct Player {
    name: String,
    hp: u32,
}
```

The `#[derive(...)]` attribute runs a procedural macro at compile time. The `Serialize` derive generates an `impl Serialize for Player` that walks the fields in order. The `Deserialize` derive generates the complementary builder. You never see the generated code unless you ask — `cargo expand` will print it if you're curious.

Derive macros aren't free at compile time. A project with 100 serde-derived types will see serde take a nontrivial slice of build time. Worth it.

## Setting up

We'll continue using the `rkvs` project from Day 15:

```sh
cd rkvs
cargo add serde --features derive
cargo add bincode@1
```

Your `Cargo.toml` dependencies should now have:

```toml
[dependencies]
crc32fast = "1"
serde = { version = "1", features = ["derive"] }
bincode = "1"
```

## Step 1 — Your first serde struct

Create `src/save.rs`:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct PlayerSave {
    pub name: String,
    pub hp: u32,
    pub level: u32,
    pub inventory: Vec<String>,
}
```

That's it. `#[derive(Serialize, Deserialize)]` on a struct with serializable fields gives you serialization to every format, for free.

Add a simple test at the bottom:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bincode_roundtrip() {
        let original = PlayerSave {
            name: "Alice".to_string(),
            hp: 100,
            level: 5,
            inventory: vec!["sword".to_string(), "potion".to_string()],
        };

        let bytes = bincode::serialize(&original).unwrap();
        let recovered: PlayerSave = bincode::deserialize(&bytes).unwrap();

        assert_eq!(original, recovered);
        println!("serialized to {} bytes", bytes.len());
    }
}
```

Register the module in `src/main.rs`:

```rust
mod format;
mod save;
```

Run:

```sh
cargo test save::tests::bincode_roundtrip -- --nocapture
```

Expected output:

```
test save::tests::bincode_roundtrip ... ok
serialized to 45 bytes
```

Forty-five bytes for a player save — vs probably 100+ bytes of JSON. Bincode is dense.

## Step 2 — Inspect the bytes

How does bincode lay out a struct? Add another test:

```rust
#[test]
fn bincode_layout() {
    let p = PlayerSave {
        name: "AB".to_string(),
        hp: 0x0102_0304,
        level: 0x0506_0708,
        inventory: vec!["X".to_string()],
    };
    let bytes = bincode::serialize(&p).unwrap();

    for b in &bytes {
        print!("{:02x} ", b);
    }
    println!();
    println!("len = {}", bytes.len());
}
```

Run:

```sh
cargo test bincode_layout -- --nocapture
```

Output roughly:

```
02 00 00 00 00 00 00 00 41 42 04 03 02 01 08 07 06 05 01 00 00 00 00 00 00 00 01 00 00 00 00 00 00 00 58
len = 35
```

Reading the layout:

- `02 00 00 00 00 00 00 00` — `u64` string length (bincode uses 8-byte lengths by default) = 2
- `41 42` — `"AB"` (ASCII)
- `04 03 02 01` — `hp` as u32 LE = `0x01020304`
- `08 07 06 05` — `level` as u32 LE = `0x05060708`
- `01 00 00 00 00 00 00 00` — `u64` vec length = 1
- `01 00 00 00 00 00 00 00` — `u64` string length for first inventory item = 1
- `58` — `"X"`

No field names on the wire. Order matters: swap two field declarations in the struct and old files won't parse correctly. That's what versioning is for.

## Step 3 — Wrapping into our record format

Let's serialize a `PlayerSave` into bytes, then write those bytes as a record in yesterday's format.

Add helper functions to `src/save.rs`:

```rust
use crate::format::{read_header, read_record, write_header, write_record};
use std::fs::File;
use std::io::{self, BufReader, BufWriter, Write};
use std::path::Path;

pub fn save_to_file(path: &Path, save: &PlayerSave) -> io::Result<()> {
    let file = File::create(path)?;
    let mut w = BufWriter::new(file);
    write_header(&mut w)?;

    let bytes = bincode::serialize(save)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    write_record(&mut w, &bytes)?;

    w.flush()?;
    Ok(())
}

pub fn load_from_file(path: &Path) -> io::Result<PlayerSave> {
    let file = File::open(path)?;
    let mut r = BufReader::new(file);
    read_header(&mut r)?;

    let bytes = read_record(&mut r)?
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "empty file"))?;

    bincode::deserialize(&bytes)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}
```

Two type-level tricks worth unpacking:

**`.map_err(|e| io::Error::new(...))`.** `bincode::serialize` returns `Result<Vec<u8>, bincode::Error>`. We're in a function that returns `io::Result<T>` — which is `Result<T, io::Error>`. These don't compose automatically, so we convert. `io::Error::new` takes `ErrorKind` and anything that implements `Into<Box<dyn Error>>`. `bincode::Error` does, so this works.

**`.ok_or_else(|| ...)`.** `read_record` returns `Option<Vec<u8>>` — `None` on clean EOF. If the caller expected data and got EOF instead, that's an error, so we convert `None` into an error with `ok_or_else`. The closure form (vs `ok_or(err)`) avoids constructing the error unless it's needed — cheaper when the error path isn't hit.

Add a test:

```rust
#[test]
fn file_roundtrip() {
    let path = std::env::temp_dir().join("rkvs_save_test.rkvs");
    let original = PlayerSave {
        name: "Bob".to_string(),
        hp: 42,
        level: 2,
        inventory: vec!["dagger".to_string()],
    };

    save_to_file(&path, &original).unwrap();
    let recovered = load_from_file(&path).unwrap();
    assert_eq!(original, recovered);

    std::fs::remove_file(&path).ok();
}
```

Run:

```sh
cargo test file_roundtrip
```

Expected: one test passes. Your save file now goes through both layers — bincode for struct layout, record format for framing and corruption detection.

## Step 4 — Adding a field with `#[serde(default)]`

Say the team decides players should have gold. Add a field to `PlayerSave`:

```rust
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct PlayerSave {
    pub name: String,
    pub hp: u32,
    pub level: u32,
    pub inventory: Vec<String>,
    pub gold: u64,  // NEW
}
```

What happens when you try to load an old save file written before `gold` existed?

```rust
#[test]
fn old_save_missing_gold() {
    // Simulate an "old" save by manually writing an old-shaped struct.
    // In the old shape, there was no `gold` field.
    #[derive(Serialize)]
    struct OldPlayerSave {
        name: String,
        hp: u32,
        level: u32,
        inventory: Vec<String>,
    }

    let old = OldPlayerSave {
        name: "Charlie".to_string(),
        hp: 50,
        level: 3,
        inventory: vec![],
    };
    let bytes = bincode::serialize(&old).unwrap();

    let result: Result<PlayerSave, _> = bincode::deserialize(&bytes);
    assert!(result.is_err(), "should fail — missing `gold` field");
}
```

Run the test — it passes, confirming bincode rejects the truncated data. Now fix it with `#[serde(default)]`:

```rust
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct PlayerSave {
    pub name: String,
    pub hp: u32,
    pub level: u32,
    pub inventory: Vec<String>,
    #[serde(default)]
    pub gold: u64,
}
```

This works *for most formats*. JSON, YAML, TOML — yes, `#[serde(default)]` rescues missing fields.

But **bincode doesn't have field names on the wire**. It reads fields positionally. If bytes run out, there's no way to know whether that was "field missing" or "file truncated." Try this test now:

```rust
#[test]
fn old_save_with_default_attr() {
    #[derive(Serialize)]
    struct OldPlayerSave {
        name: String,
        hp: u32,
        level: u32,
        inventory: Vec<String>,
    }

    let old = OldPlayerSave {
        name: "Charlie".to_string(),
        hp: 50, level: 3,
        inventory: vec![],
    };
    let bytes = bincode::serialize(&old).unwrap();

    // With the `#[serde(default)]` attribute on `gold`,
    // does bincode load it? Spoiler: no.
    let result: Result<PlayerSave, _> = bincode::deserialize(&bytes);
    assert!(result.is_err());
}
```

It still fails. Bincode is too compact — `#[serde(default)]` has no hook to fire. For bincode, the only safe way to evolve schema is **explicit versioning**. That's where we go next.

## Step 5 — Versioned save files with a tagged enum

The idea: instead of putting a single `PlayerSave` into the record, put an enum tagged with a version. Each variant is a distinct schema.

```rust
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub enum VersionedSave {
    V1(PlayerSaveV1),
    V2(PlayerSaveV2),
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct PlayerSaveV1 {
    pub name: String,
    pub hp: u32,
    pub level: u32,
    pub inventory: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct PlayerSaveV2 {
    pub name: String,
    pub hp: u32,
    pub level: u32,
    pub inventory: Vec<String>,
    pub gold: u64,
}
```

When bincode serializes an enum, it writes a **discriminant** — a `u32` identifying the variant — followed by that variant's fields. So a V1 save starts with `00 00 00 00` and a V2 save starts with `01 00 00 00`. The reader always knows which it's looking at.

Drop the plain `PlayerSave` struct; we're replacing it.

Migration function:

```rust
impl PlayerSaveV1 {
    pub fn migrate(self) -> PlayerSaveV2 {
        PlayerSaveV2 {
            name: self.name,
            hp: self.hp,
            level: self.level,
            inventory: self.inventory,
            gold: 0,  // old characters start with no gold
        }
    }
}

impl VersionedSave {
    /// Always returns the latest schema, migrating if needed.
    pub fn into_latest(self) -> PlayerSaveV2 {
        match self {
            VersionedSave::V1(v1) => v1.migrate(),
            VersionedSave::V2(v2) => v2,
        }
    }
}
```

Update `save_to_file` and `load_from_file` to use `VersionedSave`. Writer always emits the latest version:

```rust
pub fn save_to_file(path: &Path, save: &PlayerSaveV2) -> io::Result<()> {
    let file = File::create(path)?;
    let mut w = BufWriter::new(file);
    write_header(&mut w)?;

    let versioned = VersionedSave::V2(save.clone());
    let bytes = bincode::serialize(&versioned)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    write_record(&mut w, &bytes)?;

    w.flush()?;
    Ok(())
}

pub fn load_from_file(path: &Path) -> io::Result<PlayerSaveV2> {
    let file = File::open(path)?;
    let mut r = BufReader::new(file);
    read_header(&mut r)?;

    let bytes = read_record(&mut r)?
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "empty file"))?;

    let versioned: VersionedSave = bincode::deserialize(&bytes)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    Ok(versioned.into_latest())
}
```

Writer signature takes `PlayerSaveV2` — always the latest. Reader signature returns `PlayerSaveV2` — the rest of the code doesn't need to know about old versions. The enum is internal plumbing.

## Step 6 — Testing the migration

```rust
#[test]
fn v1_file_migrates_to_v2() {
    let path = std::env::temp_dir().join("rkvs_v1_test.rkvs");

    // Manually write a V1 save
    {
        let file = File::create(&path).unwrap();
        let mut w = BufWriter::new(file);
        write_header(&mut w).unwrap();

        let v1 = VersionedSave::V1(PlayerSaveV1 {
            name: "Dora".to_string(),
            hp: 80,
            level: 4,
            inventory: vec!["wand".to_string()],
        });
        let bytes = bincode::serialize(&v1).unwrap();
        write_record(&mut w, &bytes).unwrap();
        w.flush().unwrap();
    }

    // Load — should come back as V2 with gold = 0
    let loaded = load_from_file(&path).unwrap();
    assert_eq!(loaded.name, "Dora");
    assert_eq!(loaded.gold, 0);  // migrated default
    assert_eq!(loaded.inventory, vec!["wand".to_string()]);

    std::fs::remove_file(&path).ok();
}

#[test]
fn v2_file_loads_directly() {
    let path = std::env::temp_dir().join("rkvs_v2_test.rkvs");

    let original = PlayerSaveV2 {
        name: "Eva".to_string(),
        hp: 100,
        level: 7,
        inventory: vec![],
        gold: 500,
    };

    save_to_file(&path, &original).unwrap();
    let loaded = load_from_file(&path).unwrap();
    assert_eq!(loaded, original);

    std::fs::remove_file(&path).ok();
}
```

Run:

```sh
cargo test
```

Both pass. You just successfully migrated a schema without breaking old files. Players keep their saves. No angry tweets.

## Step 7 — Demo in `main.rs`

Rewrite `src/main.rs` to show off the save system:

```rust
mod format;
mod save;

use save::{load_from_file, save_to_file, PlayerSaveV2};
use std::path::Path;

fn main() -> std::io::Result<()> {
    let path = Path::new("player.rkvs");

    let before = PlayerSaveV2 {
        name: "Frodo".to_string(),
        hp: 60,
        level: 12,
        inventory: vec![
            "ring".to_string(),
            "sting".to_string(),
            "lembas".to_string(),
        ],
        gold: 42,
    };

    println!("saving: {:?}", before);
    save_to_file(path, &before)?;

    let after = load_from_file(path)?;
    println!("loaded: {:?}", after);

    assert_eq!(before, after);
    println!("roundtrip ok");

    Ok(())
}
```

Run:

```sh
cargo run
```

Expected output:

```
saving: PlayerSaveV2 { name: "Frodo", hp: 60, level: 12, inventory: ["ring", "sting", "lembas"], gold: 42 }
loaded: PlayerSaveV2 { name: "Frodo", hp: 60, level: 12, inventory: ["ring", "sting", "lembas"], gold: 42 }
roundtrip ok
```

Now do the hex dump:

```sh
xxd player.rkvs | head -5
```

You'll see:
- The `RKVS` magic
- A length prefix and CRC
- `01 00 00 00` — the V2 enum discriminant
- Then `PlayerSaveV2`'s bytes

Try editing `player.rkvs` manually (change a byte in the name region) — the CRC check from Day 15 will reject it on load.

## Step 8 — What about JSON?

Change one line in `load_from_file`:

```rust
let versioned: VersionedSave = serde_json::from_slice(&bytes)
    .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
```

Then `cargo add serde_json` and do the same in `save_to_file` with `serde_json::to_vec`. Your saves are now JSON-encoded inside the record format.

This is the magic of serde. The same structs work with every format. Use JSON when you want humans to read saves; use bincode when you want them small and fast.

## Common pitfalls

### Changing field order in a struct breaks bincode

```rust
// V1
struct PlayerSave { name: String, hp: u32 }

// V2 — silent disaster
struct PlayerSave { hp: u32, name: String }
```

Bincode is positional. Old bytes will deserialize into nonsense — your HP becomes a string length and your name becomes random bytes. Never reorder fields; only add at the end.

### Reusing a struct name across versions

If you just edit `PlayerSave` by adding a field, your code doesn't compile against old files at all — there's no way to read them. Keep `PlayerSaveV1`, `PlayerSaveV2`, etc., as separate frozen types. Only migration functions connect them.

### `#[serde(default)]` won't help with bincode

It's a self-describing-format feature. Use versioned enums for bincode.

### Bincode's 8-byte default lengths

Bincode v1 uses `u64` for every collection length, even small ones. A `Vec<u8>` of 2 bytes costs you 8 + 2 = 10 bytes. If you care about this, `bincode` offers configuration: `bincode::options().with_varint_encoding().serialize(&x)` uses variable-length integers (1 byte for values under 128). bincode v2 has a similar `config` API. For prototypes, don't worry about it.

### `serde(rename)` isn't safe for bincode either

```rust
#[derive(Serialize, Deserialize)]
struct Foo {
    #[serde(rename = "hitpoints")]
    hp: u32,
}
```

This is invisible to bincode — it cares about position, not name. Only JSON/TOML-like formats respect `rename`. If you want aliases for bincode, you have to write a custom `Deserialize` impl.

## What you learned

- **serde** is the universal serialization trait framework; every format plugs in.
- **`#[derive(Serialize, Deserialize)]`** gets you for-free encoding for any format.
- **bincode** is dense, fast, and positional — no field names on the wire.
- You can wrap bincode output in yesterday's record format — two layers, clean separation.
- **`#[serde(default)]`** rescues missing fields, but only in self-describing formats.
- **Versioned enums** are the bincode-compatible way to evolve schema.
- **Migration functions** keep old saves loading after schema changes.
- Never reorder fields, never change field types — those are breaking changes.
- Bincode uses 8-byte lengths by default; use varint config if size matters.

## Exercises

1. **Three versions.** Add `PlayerSaveV3` with a new `class: String` field. Write the migration from V2 → V3 (default: `"warrior"`). Write a test that loads a V1 file, confirms it migrates through V2 to V3.
2. **Human-readable dump.** Add a CLI subcommand `rkvs dump <file>` that loads any versioned save and prints it as pretty JSON using `serde_json::to_string_pretty`. Same code path; just switch formats.
3. **Varint encoding.** Configure bincode to use variable-length integers. Re-run the layout test. How much smaller is a typical `PlayerSaveV2`?
4. **Enum exhaustiveness.** Remove the `V1` arm from the `into_latest` match. What does the compiler say? Now add `#[non_exhaustive]` to `VersionedSave`. What changes when external crates try to match on it?
5. **Backup before migrate.** On load, if a file is V1 (older than the current version), save a `.bak` copy before migrating and rewriting. Test that the backup matches the pre-migration file byte-for-byte.

## What's next

Day 17 builds a real **Bitcask-style append-only key-value store** on top of the primitives you now have. Every `put` appends a record. Every `get` reads from disk at a known offset. Deletes are tombstone records. An in-memory `HashMap<Key, Offset>` is the "index" — a full database, in about 300 lines.

→ [Day 17 — Bitcask: an Append-Only KV Store](day-17.md)
