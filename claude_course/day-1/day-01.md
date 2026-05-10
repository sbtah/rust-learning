# Day 1 — Text Adventure Skeleton

**Domain:** games • **Time:** 60–90 minutes • **Difficulty:** easy

## What you'll build

A classic zork-style text adventure. The player stands in a room, reads a description, types commands like `go north`, and moves between five interconnected rooms until they type `quit`. By the end you'll have a working REPL and — more importantly — a mental model of Rust's ownership system built from writing actual code.

## What you'll learn

- How a Cargo project is organized
- The `String` vs `&str` distinction — Rust's most confusing pair
- Ownership, borrowing, and the three ways to pass data to functions
- Using `HashMap` for keyed lookup
- Reading from stdin
- When to reach for `match` vs `if let`

## Background

You need to understand a few things before writing the adventure, or the code won't make sense. Don't skim — these concepts are the foundation for every day that follows.

### Ownership: Rust has no garbage collector

In Python, you create an object, stop using it, and the garbage collector eventually frees it. You never think about memory.

Rust has no garbage collector. Instead, **every value has exactly one owner**. When the owner goes out of scope, the value is freed immediately — at a precise, predictable point.

```rust
fn main() {
    let greeting = String::from("hello");  // `greeting` owns a heap-allocated String
    println!("{}", greeting);
}                                          // end of scope: String is freed right here
```

That deterministic cleanup is the whole point. It's why Rust programs don't need a runtime, why they're as fast as C, and why they're memory-safe.

But it comes with a consequence: when you pass a value to a function, you have to be explicit about whether you're giving it away, lending it temporarily, or letting the function change it.

### `String` vs `&str`

`String` is an owned, growable, heap-allocated string. `&str` is a borrowed view into a string — a pointer and a length, nothing else.

```rust
let owned: String = String::from("hello");   // owns its memory
let borrowed: &str = &owned;                  // points into owned's memory
let literal: &str = "world";                  // points into the program binary
```

**The rule to memorize:** take `&str` in function arguments unless you need to own the string (store it somewhere lasting) or mutate it. This is the Rust equivalent of "take the narrowest type that works."

```rust
fn greet(name: &str) {              // ✓ good: just reads the string
    println!("hello, {}", name);
}

fn greet_bad(name: String) {        // ✗ bad: takes ownership for no reason
    println!("hello, {}", name);
}                                    // and frees it here — caller can't use `name` anymore
```

### Borrowing: `&T` and `&mut T`

Instead of giving up ownership, you can *borrow* a value. Two kinds of borrow:

- `&T` — **shared borrow**: read-only, many allowed at once.
- `&mut T` — **exclusive borrow**: can modify, only one allowed at a time, and no shared borrows may coexist.

```rust
let mut v = vec![1, 2, 3];

let a = &v;           // shared borrow #1
let b = &v;           // shared borrow #2 — fine, both read-only
println!("{} {}", a.len(), b.len());

let c = &mut v;       // exclusive borrow — ok because a, b no longer used
c.push(4);
```

The compiler enforces this. If you try to hold a `&mut` while a `&` is still active, it refuses to compile. This prevents an entire class of bugs (iterator invalidation, data races, use-after-free) at compile time.

### Three ways to pass a value to a function

Every function signature tells you exactly what the function will do with your data:

```rust
fn takes_ownership(s: String)    { /* I own it now. Original is gone. */ }
fn borrows(s: &str)              { /* I'm just reading it. */ }
fn borrows_mut(s: &mut String)   { /* I'm modifying it in place. */ }
```

Pick the narrowest one that works. This isn't politeness, it's the compiler's language.

### Structs

Structs are like Python dataclasses: named collections of fields.

```rust
struct Room {
    name: String,
    desc: String,
}

let library = Room {
    name: String::from("Library"),
    desc: String::from("Dusty books line the walls."),
};

println!("{}: {}", library.name, library.desc);
```

You can attach methods to structs via `impl` blocks:

```rust
impl Room {
    fn describe(&self) {         // `&self` is like `self` in Python but borrowed
        println!("{}\n{}", self.name, self.desc);
    }
}

library.describe();
```

`&self` means the method borrows the struct read-only. `&mut self` borrows it mutably. Plain `self` consumes it (rare).

### `HashMap`

The Rust equivalent of a Python dict. Lives in `std::collections`.

```rust
use std::collections::HashMap;

let mut rooms: HashMap<String, Room> = HashMap::new();
rooms.insert(String::from("library"), library);

if let Some(r) = rooms.get("library") {   // `get` returns Option<&V>
    r.describe();
}
```

`get` returns `Option<&V>` — either `Some(&value)` if the key exists or `None` if not. There's no KeyError; you handle both cases explicitly.

Enough background. Let's build.

## Setting up

Create your course folder if you haven't already, then the day-1 project inside it:

```bash
mkdir -p rust-30
cd rust-30
cargo new day-01
cd day-01
```

Cargo creates this layout:

```
day-01/
├── Cargo.toml      # project manifest (like pyproject.toml)
└── src/
    └── main.rs     # entry point
```

Open `Cargo.toml`:

```toml
[package]
name = "day-01"
version = "0.1.0"
edition = "2021"

[dependencies]
```

Run it now to make sure everything works:

```bash
cargo run
```

You should see `Hello, world!`. If you don't, fix your install before continuing.

## Step 1 — Define a Room

Open `src/main.rs` and replace its contents:

```rust
use std::collections::HashMap;

struct Room {
    name: String,
    description: String,
    exits: HashMap<String, String>,   // direction name -> destination room id
}

fn main() {
    let mut exits = HashMap::new();
    exits.insert(String::from("east"), String::from("hall"));
    exits.insert(String::from("north"), String::from("garden"));

    let library = Room {
        name: String::from("Dusty Library"),
        description: String::from("Moth-eaten books line the walls."),
        exits,
    };

    println!("{}", library.name);
    println!("{}", library.description);
    for (dir, dest) in &library.exits {
        println!("  {} -> {}", dir, dest);
    }
}
```

Run it:

```bash
cargo run
```

Expected output (the exit order may vary because `HashMap` iteration order is unspecified):

```
Dusty Library
Moth-eaten books line the walls.
  east -> hall
  north -> garden
```

### What just happened

- `use std::collections::HashMap;` imports `HashMap` into scope. Same as `from collections import ...` in Python.
- `struct Room { ... }` declares our data type.
- `HashMap::new()` creates an empty hashmap. The types get inferred from what we insert.
- `String::from("...")` converts a `&str` literal into an owned `String`. We need owned values because the hashmap will own them.
- `exits` (note: no `exits: exits` needed — field-shorthand works when the variable name matches the field).
- `for (dir, dest) in &library.exits` borrows the hashmap immutably and iterates. Without the `&`, we'd *consume* the hashmap, which we'd see next time we try to use it.

## Step 2 — Build a world with multiple rooms

Writing `HashMap::new()` + inserts is tedious. Let's make a helper and build five rooms.

Replace `main.rs`:

```rust
use std::collections::HashMap;

struct Room {
    name: String,
    description: String,
    exits: HashMap<String, String>,
}

impl Room {
    fn new(name: &str, description: &str) -> Room {
        Room {
            name: name.to_string(),
            description: description.to_string(),
            exits: HashMap::new(),
        }
    }

    fn exit(mut self, direction: &str, destination: &str) -> Room {
        self.exits.insert(direction.to_string(), destination.to_string());
        self
    }
}

fn build_world() -> HashMap<String, Room> {
    let mut world = HashMap::new();

    world.insert("library".to_string(),
        Room::new("Dusty Library", "Moth-eaten books line the walls.")
            .exit("east", "hall")
            .exit("north", "garden"));

    world.insert("hall".to_string(),
        Room::new("Vaulted Hall", "Sunlight streams through stained glass.")
            .exit("west", "library")
            .exit("south", "dungeon"));

    world.insert("garden".to_string(),
        Room::new("Overgrown Garden", "Vines choke the statues.")
            .exit("south", "library"));

    world.insert("dungeon".to_string(),
        Room::new("Damp Dungeon", "The smell of mildew is overwhelming.")
            .exit("north", "hall")
            .exit("east", "crypt"));

    world.insert("crypt".to_string(),
        Room::new("Forgotten Crypt", "Stone coffins lie cracked open.")
            .exit("west", "dungeon"));

    world
}

fn main() {
    let world = build_world();
    println!("Built world with {} rooms.", world.len());
}
```

Run it:

```bash
cargo run
```

Expected:

```
Built world with 5 rooms.
```

### The builder pattern

Look at `exit`. It takes `mut self` (consumes the room, mutably), modifies it, and returns it. That lets us chain:

```rust
Room::new(...).exit("east", "hall").exit("north", "garden")
```

Each `.exit(...)` consumes the previous room and produces a new one. This is the **consuming builder** pattern, common in Rust because of ownership.

The method signatures we saw:
- `&self` — borrow (read only)
- `&mut self` — mutable borrow (read + write, no consume)
- `self` — consume (used in builders, in `drop`)
- `mut self` — consume and allow internal mutation (what `exit` uses)

### Why `name.to_string()` in `Room::new`

The parameter `name: &str` is a borrowed view. But the `Room` struct's `name` field is an owned `String` — it has to be, because the `Room` needs to outlive the function call. `to_string()` allocates a new owned `String` containing a copy of the characters. This is the moment of ownership transfer: the `&str` becomes a `String` the room owns.

## Step 3 — A player and a starting position

Now we need someone to move through the world.

Add this below `Room`'s `impl` block, before `build_world`:

```rust
struct Player {
    current_room: String,   // room id
}

impl Player {
    fn new(starting_room: &str) -> Player {
        Player { current_room: starting_room.to_string() }
    }
}
```

Update `main`:

```rust
fn main() {
    let world = build_world();
    let player = Player::new("library");

    let room = world.get(&player.current_room).expect("starting room must exist");
    println!("{}", room.name);
    println!("{}", room.description);
    for (dir, _) in &room.exits {
        print!("Exits: {} ", dir);
    }
    println!();
}
```

Run it. You should see something like:

```
Dusty Library
Moth-eaten books line the walls.
Exits: east north
```

### What `.expect("...")` does

`world.get(&player.current_room)` returns `Option<&Room>`: either `Some(&Room)` if the key exists or `None` if not.

`.expect("msg")` unwraps the `Option`: if `Some`, it returns the inner `&Room`; if `None`, it panics with your message. Use `expect` when you're sure the value must be there (an invariant) and a `None` really would be a bug. Use better error handling (Day 3) everywhere else.

### `&player.current_room`

`get` takes a borrowed key. `player.current_room` is a `String`; `&player.current_room` is a `&String`, which auto-derefs to `&str` (what `get` actually wants). This works because `HashMap<String, V>::get` accepts any `&K` where `K` is borrowed — in practice, `&str` works even though the key is `String`.

## Step 4 — Read a line from stdin

Let's turn this into an actual REPL.

Add near the top:

```rust
use std::io::{self, Write, BufRead};
```

And create a helper function:

```rust
fn read_line() -> Option<String> {
    let stdin = io::stdin();
    let mut line = String::new();
    match stdin.lock().read_line(&mut line) {
        Ok(0) => None,                          // EOF (Ctrl-D)
        Ok(_) => Some(line.trim().to_string()),
        Err(_) => None,
    }
}
```

Replace `main` with:

```rust
fn main() {
    let world = build_world();
    let mut player = Player::new("library");

    loop {
        let room = world.get(&player.current_room).expect("current room must exist");
        println!("\n{}", room.name);
        println!("{}", room.description);

        print!("> ");
        io::stdout().flush().ok();

        let input = match read_line() {
            Some(s) if !s.is_empty() => s,
            Some(_) => continue,     // empty input, loop again
            None => break,           // EOF, quit
        };

        if input == "quit" {
            break;
        } else {
            println!("I don't know that command.");
        }
    }

    println!("Farewell.");
}
```

Run it:

```bash
cargo run
```

```
Dusty Library
Moth-eaten books line the walls.
> hello
I don't know that command.

Dusty Library
Moth-eaten books line the walls.
> quit
Farewell.
```

### What's new here

- `io::stdout().flush().ok()` forces the `> ` prompt to show *before* waiting for input. Without it the prompt gets buffered until a newline appears. `.ok()` converts the `Result` to an `Option` we ignore — acceptable here because flush rarely fails and we don't need to react.
- `read_line` takes `&mut line` because `read_line` *appends* the read data to the string. That's why we pass a mutable borrow.
- `Ok(0)` from `read_line` means EOF. `Ok(n)` means `n` bytes were read.
- `match` with guards: `Some(s) if !s.is_empty() => s` matches when the option is `Some` *and* the guard predicate holds.
- `continue` and `break` are just like Python's.

## Step 5 — Implement commands

Now let's handle real commands: `look`, `go <direction>`, `quit`.

Replace the command-handling block inside your loop:

```rust
let words: Vec<&str> = input.split_whitespace().collect();
match words.as_slice() {
    ["quit"] => break,
    ["look"] => {
        // The prompt already printed the room description,
        // but `look` should re-print it on demand.
        continue;   // loop top will re-print the room
    }
    ["go", direction] => {
        let room = world.get(&player.current_room).expect("current room");
        match room.exits.get(*direction) {
            Some(dest) => player.current_room = dest.clone(),
            None => println!("You can't go that way."),
        }
    }
    _ => println!("I don't know that command."),
}
```

Full `main.rs` so far:

```rust
use std::collections::HashMap;
use std::io::{self, Write, BufRead};

struct Room {
    name: String,
    description: String,
    exits: HashMap<String, String>,
}

impl Room {
    fn new(name: &str, description: &str) -> Room {
        Room {
            name: name.to_string(),
            description: description.to_string(),
            exits: HashMap::new(),
        }
    }

    fn exit(mut self, direction: &str, destination: &str) -> Room {
        self.exits.insert(direction.to_string(), destination.to_string());
        self
    }
}

struct Player {
    current_room: String,
}

impl Player {
    fn new(starting_room: &str) -> Player {
        Player { current_room: starting_room.to_string() }
    }
}

fn build_world() -> HashMap<String, Room> {
    let mut world = HashMap::new();
    world.insert("library".to_string(),
        Room::new("Dusty Library", "Moth-eaten books line the walls.")
            .exit("east", "hall").exit("north", "garden"));
    world.insert("hall".to_string(),
        Room::new("Vaulted Hall", "Sunlight streams through stained glass.")
            .exit("west", "library").exit("south", "dungeon"));
    world.insert("garden".to_string(),
        Room::new("Overgrown Garden", "Vines choke the statues.")
            .exit("south", "library"));
    world.insert("dungeon".to_string(),
        Room::new("Damp Dungeon", "The smell of mildew is overwhelming.")
            .exit("north", "hall").exit("east", "crypt"));
    world.insert("crypt".to_string(),
        Room::new("Forgotten Crypt", "Stone coffins lie cracked open.")
            .exit("west", "dungeon"));
    world
}

fn read_line() -> Option<String> {
    let stdin = io::stdin();
    let mut line = String::new();
    match stdin.lock().read_line(&mut line) {
        Ok(0) => None,
        Ok(_) => Some(line.trim().to_string()),
        Err(_) => None,
    }
}

fn main() {
    let world = build_world();
    let mut player = Player::new("library");

    loop {
        let room = world.get(&player.current_room).expect("current room must exist");
        println!("\n{}", room.name);
        println!("{}", room.description);
        let exits: Vec<&String> = room.exits.keys().collect();
        println!("Exits: {}", exits.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", "));

        print!("> ");
        io::stdout().flush().ok();

        let input = match read_line() {
            Some(s) if !s.is_empty() => s,
            Some(_) => continue,
            None => break,
        };

        let words: Vec<&str> = input.split_whitespace().collect();
        match words.as_slice() {
            ["quit"] => break,
            ["look"] => continue,
            ["go", direction] => {
                match room.exits.get(*direction) {
                    Some(dest) => player.current_room = dest.clone(),
                    None => println!("You can't go that way."),
                }
            }
            _ => println!("I don't know that command."),
        }
    }

    println!("Farewell.");
}
```

Run it and play:

```
Dusty Library
Moth-eaten books line the walls.
Exits: east, north
> go east

Vaulted Hall
Sunlight streams through stained glass.
Exits: west, south
> go south

Damp Dungeon
The smell of mildew is overwhelming.
Exits: north, east
> go up
You can't go that way.

Damp Dungeon
The smell of mildew is overwhelming.
Exits: north, east
> quit
Farewell.
```

You have a working text adventure.

## Common pitfalls

### "Cannot move out of borrowed content"

You tried `player.current_room = room.exits.get("east").unwrap()`. That returns `Option<&String>` — you can't take ownership of the string that the hashmap still owns. Solution: `.clone()` to make your own copy (we did this above), or restructure so you use the borrow briefly and drop it.

### "Borrowed value does not live long enough"

Usually you've stored a reference that outlives what it points to. For example:

```rust
let exits = {
    let room = world.get("library").unwrap();
    &room.exits                     // &exits borrows from `room`
};                                  // `room` dropped here, but `exits` still borrows it
println!("{:?}", exits);            // compile error
```

Solution: don't let the reference escape the scope that produced it. Either work inside that scope, or clone the data out.

### "Cannot borrow `world` as mutable because it is also borrowed as immutable"

You held an `&Room` from `world.get(...)` and then tried to `world.insert(...)` elsewhere. Shared and exclusive borrows can't coexist. Solution: let the shared borrow end (put it in a smaller scope) before the mutation.

### Splitting an empty line

`input.split_whitespace().collect::<Vec<_>>()` on an empty string gives an empty vec, which would match `_` and print "I don't know that command." Our `Some(s) if !s.is_empty() => s` guard handles this, but watch for it.

## What you learned

- Rust's **ownership** model: one owner, freed at scope exit.
- **Borrows**: `&T` shared (many readers), `&mut T` exclusive (one writer).
- **`String` vs `&str`**: owned vs borrowed, pick narrowest argument type.
- **Structs and methods** via `impl` blocks.
- **`HashMap`** with `.insert`, `.get` (returns `Option<&V>`).
- **`Option`** and basic unwrapping with `.expect("...")`.
- **`match`** with array patterns and guards — more powerful than Python's.
- **Reading from stdin** with `io::stdin().lock().read_line`.

## Exercises

These are optional but recommended — they push you past just typing along.

1. **Inventory.** Add `inventory: Vec<String>` to `Player` and `items: Vec<String>` to `Room`. Commands `take <item>` moves an item from the current room into the inventory; `drop <item>` moves it back. Print `Items: ...` in the room display.
2. **Aliases.** Accept `north` and `n` as the same. Do it with a match arm or a helper function.
3. **Locked doors.** A room exit can be locked; typing `go east` prints "The door is locked." Add a `key` item; if the player has it in inventory, the door opens. This forces you to work with two mutable borrows on `world` + `player` — a good exercise.

## What's next

Day 2 introduces **enums with data** — the single best feature of Rust's type system. We'll use them to build a combat system where a poisoned player, a fireball spell, and a fleeing troll are all modelled cleanly through sum types. No more `status == "poisoned"` strings.

→ [Day 2 — Combat system](day-02.md)
