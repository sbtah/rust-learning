# Day 3 — Save and Load

**Domain:** games • **Time:** 60–90 minutes • **Difficulty:** medium

## What you'll build

`save` and `load` commands for the adventure game. The world *changes* as you play (items picked up, enemies killed), so the save file must capture current state, not initial state. You'll design a simple text format, write a parser, handle every error case by value (no exceptions, no panics), and perform atomic saves so a crashed program never leaves a half-written file.

## What you'll learn

- **`Option<T>`** — the type that replaces `None` / `null`
- **`Result<T, E>`** — the type that replaces exceptions
- **The `?` operator** — propagate errors cleanly
- **Custom error enums** with manual `Display` and `From` impls
- **File I/O**: `fs::read_to_string`, `fs::write`, `fs::rename`
- **Atomic file writes** via the write-then-rename pattern

## Background

### There is no `null` in Rust

Python has `None`. You get it, it's a type. Forget to check for it, and you get `AttributeError: 'NoneType' object has no attribute 'foo'` at runtime.

Rust has no null. Instead, optional values are explicit in the type:

```rust
enum Option<T> {
    Some(T),
    None,
}
```

If a function might return no value, it returns `Option<T>`. The caller *must* handle both cases — there's no way to accidentally skip the check, because the `T` isn't directly accessible; it's wrapped.

```rust
let map: HashMap<String, i32> = HashMap::new();
let x: Option<&i32> = map.get("key");

match x {
    Some(n) => println!("got {}", n),
    None => println!("key missing"),
}
```

### There are no exceptions

Python has exceptions. They can come from anywhere, interrupt any line, and are easy to forget about. A function's signature tells you nothing about what might go wrong.

Rust has no exceptions. Fallible operations return `Result<T, E>`:

```rust
enum Result<T, E> {
    Ok(T),
    Err(E),
}
```

`fs::read_to_string(path)` returns `Result<String, io::Error>`. Either you read the file (`Ok(contents)`) or you didn't (`Err(io::Error)`). The type system forces you to deal with both.

```rust
match fs::read_to_string("save.txt") {
    Ok(contents) => println!("loaded: {}", contents),
    Err(e) => println!("load failed: {}", e),
}
```

This is verbose for chains of fallible operations. Enter the `?` operator.

### The `?` operator

`expr?` desugars roughly to:

```rust
match expr {
    Ok(value) => value,
    Err(e) => return Err(e.into()),
}
```

Where `.into()` converts `e` to the return type's error type via a `From` impl — we'll meet that shortly.

Use `?` inside any function that returns `Result<_, _>`:

```rust
fn read_two_files(a: &str, b: &str) -> Result<(String, String), io::Error> {
    let content_a = fs::read_to_string(a)?;
    let content_b = fs::read_to_string(b)?;
    Ok((content_a, content_b))
}
```

If either read fails, the `?` returns the error early. If both succeed, we fall through to `Ok((content_a, content_b))`.

`?` also works on `Option`: in a function returning `Option<T>`, `expr?` unwraps `Some` or returns `None` from the whole function.

### Where `unwrap` and `expect` are acceptable

`.unwrap()` panics on `None` / `Err`. `.expect("msg")` does the same with a custom message.

Acceptable:

- **Invariants.** `world.get(&current_room).expect("player's room must exist")` — a `None` here is a programming bug, not an error condition.
- **Quick prototypes.** Fine for exploration; remove before shipping.

Not acceptable:

- **Anything touching user input or I/O.** Those fail for real reasons; handle them.

## Setting up

```bash
cargo new day-03
cd day-03
cargo add rand@0.8
```

We'll build on yesterday's world structure. Copy the `Room`, `Player`, and world-builder into `main.rs` — we need somewhere to save and load from.

(From now on I'll stop repeating this boilerplate. Assume you have yesterday's world + today's error types in `main.rs`. If you want a starting template, see the end of this file.)

## Step 1 — Design the save format

Our save file will be line-based plain text, so you can inspect it with `cat`. Design:

```
V1
PLAYER;library;42;sword,lantern
ROOM;library;Dusty Library;Moth-eaten books.;east=hall,north=garden;scroll
ROOM;hall;Vaulted Hall;Sunlight streams.;west=library,south=dungeon;
```

Fields are semicolon-separated. First line is a version marker. `PLAYER` captures current room, HP, and inventory (comma-separated). Each `ROOM` captures id, name, description, exits (comma-separated `dir=dest` pairs), and current items.

We're not worrying about escaping semicolons in descriptions — for real tools we'd use a proper format (that's Day 16).

## Step 2 — Write the save function

Add to `main.rs`:

```rust
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::Path;

fn save_game(path: &Path, world: &HashMap<String, Room>, player: &Player) -> io::Result<()> {
    let mut out = String::new();
    out.push_str("V1\n");

    out.push_str(&format!(
        "PLAYER;{};{};{}\n",
        player.current_room,
        player.hp,
        player.inventory.join(","),
    ));

    for (id, room) in world {
        let exits: Vec<String> = room.exits
            .iter()
            .map(|(dir, dest)| format!("{}={}", dir, dest))
            .collect();
        out.push_str(&format!(
            "ROOM;{};{};{};{};{}\n",
            id, room.name, room.description,
            exits.join(","),
            room.items.join(","),
        ));
    }

    // Atomic write: tmp file then rename
    let tmp = path.with_extension("sav.tmp");
    fs::write(&tmp, out)?;
    fs::rename(&tmp, path)?;

    Ok(())
}
```

This assumes `Player` has `hp: i32` and `inventory: Vec<String>`, and `Room` has `items: Vec<String>`. Add those fields now if you haven't already.

### Why `io::Result<()>`

`io::Result<T>` is a type alias for `Result<T, io::Error>`. `()` is the unit type (like Python's `None` or C's `void`). We return `Ok(())` to signal success with no data.

### Atomic save

`fs::write(&tmp, out)?` writes to `path.with_extension("sav.tmp")`. If the program crashes halfway, the actual save file is untouched. `fs::rename(&tmp, path)?` atomically replaces the old save — on POSIX, the rename is a single kernel operation.

Without this, a Ctrl-C during save could leave a zero-byte or half-written `.sav` file, and your next `load` would find a corrupt save. This pattern is how every tool you trust (git, editors, databases) handles file writes.

### Why `format!("{}", ...)` in a loop instead of `write!`?

We could write to the file incrementally. Building up a `String` and writing once is simpler for now and lets `fs::write` handle the single atomic syscall. For 1000 rooms this would matter; for 5 rooms it's invisible.

## Step 3 — Design a proper error type

For `save`, `io::Error` is enough — every failure is an I/O failure. For `load`, we have two *kinds* of failure:

1. I/O error (file missing, permission denied).
2. Parse error (the file exists but is malformed).

Combining them into one `io::Error` would lose information. Instead, define our own error enum:

```rust
#[derive(Debug)]
pub enum SaveError {
    Io(io::Error),
    UnknownVersion(String),
    Parse { line: usize, message: String },
    MissingPlayer,
}
```

`#[derive(Debug)]` lets us print it with `{:?}`. For user-facing display we want cleaner output — implement `std::fmt::Display` manually:

```rust
use std::fmt;

impl fmt::Display for SaveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SaveError::Io(e) => write!(f, "i/o error: {}", e),
            SaveError::UnknownVersion(v) => write!(f, "unknown save version: {}", v),
            SaveError::Parse { line, message } => {
                write!(f, "parse error on line {}: {}", line, message)
            }
            SaveError::MissingPlayer => write!(f, "save file has no PLAYER record"),
        }
    }
}

impl std::error::Error for SaveError {}
```

### Automatic conversion: `From<io::Error>`

For `?` to work when `fs::read_to_string(path)?` returns `io::Error` but our function returns `SaveError`, we need the compiler to know how to convert. Implement `From`:

```rust
impl From<io::Error> for SaveError {
    fn from(e: io::Error) -> SaveError {
        SaveError::Io(e)
    }
}
```

Now anywhere you have a `Result<T, io::Error>` and a surrounding function returning `Result<_, SaveError>`, you can use `?` and the conversion happens automatically.

(Tomorrow we'll learn `thiserror`, which derives all this boilerplate away. Today we write it by hand so you understand what's generated.)

## Step 4 — Write the load function

```rust
fn load_game(path: &Path) -> Result<(HashMap<String, Room>, Player), SaveError> {
    let content = fs::read_to_string(path)?;   // io::Error -> SaveError via From

    let mut lines = content.lines().enumerate();

    // First line: version
    let (_, first) = lines.next()
        .ok_or_else(|| SaveError::Parse { line: 0, message: "empty file".to_string() })?;
    if first.trim() != "V1" {
        return Err(SaveError::UnknownVersion(first.to_string()));
    }

    let mut world: HashMap<String, Room> = HashMap::new();
    let mut player: Option<Player> = None;

    for (idx, line) in lines {
        let line_num = idx + 1;
        let parts: Vec<&str> = line.split(';').collect();

        match parts.as_slice() {
            ["PLAYER", room_id, hp, inventory] => {
                let hp = hp.parse::<i32>().map_err(|_| SaveError::Parse {
                    line: line_num,
                    message: format!("invalid hp: {}", hp),
                })?;
                let inventory = if inventory.is_empty() {
                    Vec::new()
                } else {
                    inventory.split(',').map(String::from).collect()
                };
                player = Some(Player {
                    current_room: room_id.to_string(),
                    hp,
                    max_hp: 50,
                    inventory,
                });
            }
            ["ROOM", id, name, desc, exits, items] => {
                let mut exits_map = HashMap::new();
                if !exits.is_empty() {
                    for pair in exits.split(',') {
                        let (dir, dest) = pair.split_once('=').ok_or_else(|| {
                            SaveError::Parse {
                                line: line_num,
                                message: format!("bad exit pair: {}", pair),
                            }
                        })?;
                        exits_map.insert(dir.to_string(), dest.to_string());
                    }
                }
                let items = if items.is_empty() {
                    Vec::new()
                } else {
                    items.split(',').map(String::from).collect()
                };
                world.insert(id.to_string(), Room {
                    name: name.to_string(),
                    description: desc.to_string(),
                    exits: exits_map,
                    items,
                });
            }
            _ => {
                return Err(SaveError::Parse {
                    line: line_num,
                    message: format!("unrecognized record: {}", line),
                });
            }
        }
    }

    let player = player.ok_or(SaveError::MissingPlayer)?;
    Ok((world, player))
}
```

That's a lot. Let's unpack.

### Structure: parse line by line

- `content.lines()` gives an iterator of `&str` (no allocation).
- `.enumerate()` attaches a zero-based index.
- For each subsequent line, match on the parts.

### The pattern `parts.as_slice()` returning `[..., ...]`

`parts` is a `Vec<&str>`. `.as_slice()` turns it into a `&[&str]`. We then use *slice patterns* (a Rust-specific feature) to destructure by length:

```rust
match parts.as_slice() {
    ["PLAYER", room_id, hp, inventory] => { /* exactly 4 parts */ }
    ["ROOM", id, name, desc, exits, items] => { /* exactly 6 parts */ }
    _ => { /* anything else */ }
}
```

If a line has 3 parts, it matches none of our specific patterns and falls through to the error arm. Elegant and precise.

### `.ok_or_else` and `.ok_or`

Convert `Option<T>` to `Result<T, E>`:

- `opt.ok_or(err)` — takes a pre-made error.
- `opt.ok_or_else(|| err)` — takes a closure that computes the error on demand. Use this when constructing the error allocates, so you don't pay the cost on the happy path.

### `.parse::<i32>()`

Parses a string to an integer. Returns `Result<i32, ParseIntError>`. We convert the failure to our `SaveError` with `.map_err`.

### `.split_once('=')`

Splits a string on the first occurrence. Returns `Option<(&str, &str)>` — `Some` if the delimiter is present, `None` otherwise.

## Step 5 — Wire into the REPL

Back in your `main` loop, add cases for `save` and `load`:

```rust
["save", filename] => {
    let path = Path::new(*filename);
    match save_game(path, &world, &player) {
        Ok(()) => println!("Saved to {}.", path.display()),
        Err(e) => println!("Save failed: {}", e),
    }
}
["load", filename] => {
    let path = Path::new(*filename);
    match load_game(path) {
        Ok((new_world, new_player)) => {
            world = new_world;
            player = new_player;
            println!("Loaded from {}.", path.display());
        }
        Err(e) => println!("Load failed: {}", e),
    }
}
```

Since we're now reassigning `world`, `main` must declare it `mut`:

```rust
let mut world = build_world();
let mut player = Player::new("library");
```

### `Path::new(*filename)`

Why `*filename`? In our `match`, `filename` destructures a slice element, so it's `&&str`. `*filename` dereferences to `&str`, which `Path::new` accepts.

`Path` vs `&str`: Rust has dedicated path types (`Path`, `PathBuf`) to handle platform-specific separators, extensions, etc. You can usually treat `&str` as a path implicitly via `AsRef<Path>`, but `Path::new(s)` is explicit.

## Step 6 — Test it

Put it all together and test:

```
> take scroll
You take the scroll.

> save game.sav
Saved to game.sav.

> quit
Farewell.
```

Inspect the file:

```bash
cat game.sav
```

```
V1
PLAYER;library;50;scroll
ROOM;library;Dusty Library;Moth-eaten books.;east=hall,north=garden;
ROOM;hall;Vaulted Hall;Sunlight streams.;west=library,south=dungeon;
...
```

Run again, load, verify:

```
> load game.sav
Loaded from game.sav.

Dusty Library
Moth-eaten books.
Exits: east, north

> inventory
You carry: scroll
```

Test the error paths:

```
> load missing.sav
Load failed: i/o error: No such file or directory (os error 2)
```

Make a corrupt file:

```bash
echo -e "V1\nPLAYER;library;not-a-number;" > broken.sav
```

Load it:

```
> load broken.sav
Load failed: parse error on line 2: invalid hp: not-a-number
```

### Test atomic save

Force a filesystem that won't let you write:

```
> save /root/nope.sav
Save failed: i/o error: Permission denied (os error 13)
```

No `.sav.tmp` left behind (well, possibly in `/root`, but you can't create it either — the test isn't perfect on read-only dirs, but the point is the original save is untouched in a real scenario).

## Common pitfalls

### "The trait `From<ParseIntError>` is not implemented"

You wrote `hp.parse::<i32>()?` in a function returning `Result<_, SaveError>`. `?` wants `From<ParseIntError> for SaveError`, which doesn't exist. Options:

1. Add `impl From<ParseIntError> for SaveError`.
2. Use `.map_err(|_| SaveError::Parse { ... })?` — we did this above.

Option 2 is better here because it lets us keep the line number.

### "Borrowed value does not live long enough"

Often appears when you return a `&str` that points into a local `String`. The `String` drops at end of function; your `&str` would dangle. The compiler prevents it. Solution: return `String`, not `&str`, from anything that allocates.

### Forgetting `Path::new` or `as_ref`

Different file APIs want different types. When in doubt: `Path::new(&string)` gets you a `&Path`. `path.to_path_buf()` gets you an owned `PathBuf`.

### Lossy string conversions

Our format will break if a room description contains a literal semicolon or newline. That's a real limitation of ad-hoc formats. On Day 16, we replace all of this with `serde` + `bincode` and stop caring.

## What you learned

- **`Option<T>`** for missing values; **`Result<T, E>`** for fallible operations.
- **`?` operator** propagates errors, converting with `From` automatically.
- **Custom error enums** with `Debug`, `Display`, and `Error` implementations.
- **`impl From<A> for B`** to make `?` work across error types.
- **Slice patterns** (`parts.as_slice() -> [...]`) for precise matching.
- **`.parse::<T>()`** for strings-to-values; `.map_err` to reshape errors.
- **Atomic file writes** via tmp-then-rename.
- **Never `.unwrap()`** on user input or I/O.

## Exercises

1. **Versioned format.** Add `V2` which includes a `TIMESTAMP` line. Your loader reads the first line, matches on version, dispatches to `parse_v1` or `parse_v2`. Loading V1 under V2 code uses `Option::None` for the timestamp.
2. **Meta records.** Add an optional `META` line that includes `author=<name>` and `turn_count=<n>`. Extend your loader but keep V1 saves readable.
3. **Error recovery.** Instead of failing on the first parse error, collect all errors and return them as `Vec<SaveError>` — common in compilers and linters. You'll need a new variant `Multiple(Vec<SaveError>)`.
4. **JSON pretty-print.** Add a `--format json` option to `save` that writes the same data as JSON. Use `serde_json` — you'll write a proper version tomorrow, but try it quickly today as a preview.

## Starting template (for reference)

If you want a clean starting point:

```rust
use rand::Rng;
use std::collections::HashMap;
use std::io::{self, Write, BufRead};

struct Room {
    name: String,
    description: String,
    exits: HashMap<String, String>,
    items: Vec<String>,
}

impl Room {
    fn new(name: &str, description: &str) -> Room {
        Room {
            name: name.to_string(),
            description: description.to_string(),
            exits: HashMap::new(),
            items: Vec::new(),
        }
    }
    fn exit(mut self, direction: &str, destination: &str) -> Room {
        self.exits.insert(direction.to_string(), destination.to_string());
        self
    }
    fn item(mut self, name: &str) -> Room {
        self.items.push(name.to_string());
        self
    }
}

struct Player {
    current_room: String,
    hp: i32,
    max_hp: i32,
    inventory: Vec<String>,
}

impl Player {
    fn new(starting_room: &str) -> Player {
        Player {
            current_room: starting_room.to_string(),
            hp: 50,
            max_hp: 50,
            inventory: Vec::new(),
        }
    }
}

fn build_world() -> HashMap<String, Room> {
    let mut world = HashMap::new();
    world.insert("library".to_string(),
        Room::new("Dusty Library", "Moth-eaten books.")
            .exit("east", "hall").exit("north", "garden").item("scroll"));
    world.insert("hall".to_string(),
        Room::new("Vaulted Hall", "Sunlight streams.")
            .exit("west", "library").exit("south", "dungeon"));
    world.insert("garden".to_string(),
        Room::new("Overgrown Garden", "Vines choke.")
            .exit("south", "library").item("lantern"));
    world.insert("dungeon".to_string(),
        Room::new("Damp Dungeon", "Mildew.")
            .exit("north", "hall").exit("east", "crypt"));
    world.insert("crypt".to_string(),
        Room::new("Forgotten Crypt", "Cracked coffins.")
            .exit("west", "dungeon").item("gold"));
    world
}
```

## What's next

Day 4 moves beyond concrete types to **traits** — Rust's way of expressing shared behaviour. You'll refactor combat so players, goblins, and trolls share one `Entity` interface, and learn the tradeoff between static (generics) and dynamic (`Box<dyn Trait>`) dispatch.

→ [Day 4 — Entity trait system](day-04.md)
