# Day 7 — Rich Errors

**Domain:** games • **Time:** 90 minutes • **Difficulty:** medium

## What you'll build

A proper command parser for the adventure game. Input like `"pick up the rusty sword"` is tokenized, the verb identified (with aliases), preconditions checked against the current world state, and either a typed `Action` or a structured error is returned. Errors carry context: not just "unknown verb", but which verb, on which line, from which input. You'll finish with **zero** `.unwrap()` / `.expect()` calls in user-reachable paths.

## What you'll learn

- **`thiserror`** crate for library-style error enums
- **`anyhow`** crate for application-level error propagation
- **Error composition** via `From` impls
- **Adding context** to errors with `anyhow::Context`
- Why two crates exist for what seems like one problem
- Multi-word verb matching and alias handling

## Background

### Why two crates?

Rust's error story has two tiers:

- **Library authors** want to expose typed errors their callers can match on. `struct StorageError`, `enum ParseError { ... }`. Rich, specific, composable via `From`.
- **Applications** want to just *propagate* errors, add some context, and print a nice trace at the top. They don't care about the exact type — they care that errors bubble up and get reported.

`thiserror` serves the first need. It's a `#[derive]` macro that auto-generates `Display`, `Error`, and `From` impls for your enum. You write the error type; it writes the boilerplate.

`anyhow` serves the second. `anyhow::Error` is a type-erased box that can hold any error implementing `std::error::Error`. Use it as the return type in `main` and similar application-glue code. Add context with `.with_context(|| format!("loading config {path}"))`.

### The Rule of Thumb

- **Library crates**: typed errors via `thiserror`. Callers can handle specific variants.
- **Binary crates / application code**: `anyhow::Result<T>` everywhere. Fast and flexible.

Today you'll use both — the command parser (`thiserror`) returns typed errors; `main` (`anyhow`) catches and decorates them.

### `thiserror` basics

Without `thiserror` (yesterday's style):

```rust
#[derive(Debug)]
pub enum ParseError {
    UnknownVerb(String),
    MissingObject { verb: String },
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::UnknownVerb(v) => write!(f, "unknown verb '{}'", v),
            ParseError::MissingObject { verb } => {
                write!(f, "what do you want to {}?", verb)
            }
        }
    }
}

impl std::error::Error for ParseError {}
```

With `thiserror`:

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("unknown verb '{0}'")]
    UnknownVerb(String),

    #[error("what do you want to {verb}?")]
    MissingObject { verb: String },
}
```

Same code, 75% less typing. The `#[error("...")]` attributes specify the `Display` output — tuple variant fields are `{0}`, `{1}`; struct variant fields are `{name}`.

### `anyhow::Context`

```rust
use anyhow::Context;

fn load_config(path: &Path) -> anyhow::Result<Config> {
    let bytes = std::fs::read(path)
        .with_context(|| format!("reading {}", path.display()))?;
    let cfg = parse_config(&bytes)
        .with_context(|| format!("parsing {}", path.display()))?;
    Ok(cfg)
}
```

If reading fails, the resulting error chain is:

```
Error: reading config/settings.ron

Caused by:
    0: No such file or directory (os error 2)
```

The top line is your context, the "caused by" list is the underlying error. For debugging real apps, this is gold.

## Setting up

```bash
cargo new day-07
cd day-07
cargo add thiserror
cargo add anyhow
```

## Step 1 — Clean error types with thiserror

Start `main.rs` fresh. First, define the domain types we'll parse into:

```rust
use thiserror::Error;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    North, South, East, West, Up, Down,
}

#[derive(Debug)]
pub enum Action {
    Look,
    Go(Direction),
    Take(String),
    Drop(String),
    Inventory,
    Attack(String),
    Save(PathBuf),
    Load(PathBuf),
    Quit,
}
```

Then the error type:

```rust
#[derive(Debug, Error)]
pub enum CommandError {
    #[error("type something")]
    Empty,

    #[error("unknown verb '{0}'")]
    UnknownVerb(String),

    #[error("what do you want to {verb}?")]
    MissingObject { verb: String },

    #[error("'{0}' is not a direction")]
    BadDirection(String),

    #[error("expected a filename after '{verb}'")]
    MissingFilename { verb: String },
}
```

We could add an `Io(#[from] io::Error)` variant later if the parser ever reads files. For now, keep it pure.

### The `#[from]` attribute (preview)

`#[from]` on a tuple variant tells `thiserror` to generate `From<InnerType> for MyError`:

```rust
#[derive(Debug, Error)]
pub enum MyError {
    #[error("i/o: {0}")]
    Io(#[from] std::io::Error),   // <-- generates From<io::Error>
}
```

Now `?` on a `Result<_, io::Error>` inside a function returning `Result<_, MyError>` just works.

## Step 2 — Direction::from_str

```rust
impl std::str::FromStr for Direction {
    type Err = CommandError;
    fn from_str(s: &str) -> Result<Direction, CommandError> {
        match s.to_lowercase().as_str() {
            "n" | "north" => Ok(Direction::North),
            "s" | "south" => Ok(Direction::South),
            "e" | "east"  => Ok(Direction::East),
            "w" | "west"  => Ok(Direction::West),
            "u" | "up"    => Ok(Direction::Up),
            "d" | "down"  => Ok(Direction::Down),
            _             => Err(CommandError::BadDirection(s.to_string())),
        }
    }
}
```

Implementing `FromStr` lets us use `"north".parse::<Direction>()`.

### Multiple patterns with `|`

`"n" | "north" => ...` matches either. Same idea as OR-patterns in Python's `match`.

## Step 3 — Tokenization & verb matching

Now the parser:

```rust
pub fn parse_command(input: &str) -> Result<Action, CommandError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(CommandError::Empty);
    }

    let lower = trimmed.to_lowercase();

    // Multi-word verbs — check longest prefixes first
    if let Some(rest) = lower.strip_prefix("pick up ") {
        return Ok(Action::Take(rest.trim().to_string()));
    }

    let mut parts = lower.split_whitespace();
    let verb = parts.next().ok_or(CommandError::Empty)?;
    let rest: Vec<&str> = parts.collect();

    match verb {
        "look" | "l" => Ok(Action::Look),

        "go" | "move" | "walk" => {
            let dir_str = rest.first().ok_or(CommandError::MissingObject {
                verb: "go".to_string(),
            })?;
            let dir = dir_str.parse::<Direction>()?;
            Ok(Action::Go(dir))
        }

        // A direction alone means "go <dir>"
        "n" | "north" | "s" | "south" | "e" | "east" | "w" | "west" | "u" | "up" | "d" | "down" => {
            let dir = verb.parse::<Direction>()?;
            Ok(Action::Go(dir))
        }

        "take" | "grab" | "get" => {
            let obj = rest.join(" ");
            if obj.is_empty() {
                return Err(CommandError::MissingObject { verb: "take".to_string() });
            }
            Ok(Action::Take(obj))
        }

        "drop" => {
            let obj = rest.join(" ");
            if obj.is_empty() {
                return Err(CommandError::MissingObject { verb: "drop".to_string() });
            }
            Ok(Action::Drop(obj))
        }

        "inventory" | "inv" | "i" => Ok(Action::Inventory),

        "attack" | "kill" | "fight" => {
            let obj = rest.join(" ");
            if obj.is_empty() {
                return Err(CommandError::MissingObject { verb: "attack".to_string() });
            }
            Ok(Action::Attack(obj))
        }

        "save" => {
            let name = rest.first().ok_or(CommandError::MissingFilename {
                verb: "save".to_string(),
            })?;
            Ok(Action::Save(PathBuf::from(name)))
        }

        "load" => {
            let name = rest.first().ok_or(CommandError::MissingFilename {
                verb: "load".to_string(),
            })?;
            Ok(Action::Load(PathBuf::from(name)))
        }

        "quit" | "q" | "exit" => Ok(Action::Quit),

        other => Err(CommandError::UnknownVerb(other.to_string())),
    }
}
```

### How `?` and `From` work together

Inside `parse_command`, `dir_str.parse::<Direction>()?` returns `Result<Direction, CommandError>` — our `FromStr` impl's error type matches the function's error type, so `?` works directly.

If `parse` returned a different error type, we'd need either:
- `impl From<OtherError> for CommandError`, or
- `.map_err(|e| CommandError::Something(e))?`

### Why `rest.first()` not `rest[0]`?

`rest[0]` panics if `rest` is empty. `rest.first()` returns `Option<&&str>` — we then chain `.ok_or(...)` to convert to our error. Safe by default.

## Step 4 — Test it

```rust
fn main() {
    let inputs = [
        "look",
        "l",
        "go north",
        "n",
        "pick up the rusty sword",
        "take scroll",
        "attack goblin",
        "save game.sav",
        "quit",
        "",
        "dance",
        "go nowhere",
        "take",
        "save",
    ];

    for input in inputs {
        print!("{:>32}  =>  ", format!("{:?}", input));
        match parse_command(input) {
            Ok(action) => println!("{:?}", action),
            Err(e) => println!("ERROR: {}", e),
        }
    }
}
```

Run it:

```
                        "look"  =>  Look
                           "l"  =>  Look
                    "go north"  =>  Go(North)
                           "n"  =>  Go(North)
      "pick up the rusty sword"  =>  Take("the rusty sword")
                  "take scroll"  =>  Take("scroll")
                 "attack goblin"  =>  Attack("goblin")
                "save game.sav"  =>  Save("game.sav")
                         "quit"  =>  Quit
                            ""  =>  ERROR: type something
                        "dance"  =>  ERROR: unknown verb 'dance'
                   "go nowhere"  =>  ERROR: 'nowhere' is not a direction
                         "take"  =>  ERROR: what do you want to take?
                         "save"  =>  ERROR: expected a filename after 'save'
```

Every error has a specific, helpful message. The structured variants (with fields) were built by the compiler from your `#[error("...")]` attributes.

## Step 5 — Unit tests

Add at the bottom of `main.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_look() {
        assert!(matches!(parse_command("look"), Ok(Action::Look)));
        assert!(matches!(parse_command("l"), Ok(Action::Look)));
    }

    #[test]
    fn parses_directions_bare() {
        assert!(matches!(
            parse_command("north"),
            Ok(Action::Go(Direction::North))
        ));
        assert!(matches!(parse_command("n"), Ok(Action::Go(Direction::North))));
    }

    #[test]
    fn parses_multi_word_pickup() {
        let r = parse_command("pick up rusty sword");
        assert!(matches!(r, Ok(Action::Take(ref s)) if s == "rusty sword"));
    }

    #[test]
    fn rejects_unknown_verb() {
        assert!(matches!(
            parse_command("dance"),
            Err(CommandError::UnknownVerb(_))
        ));
    }

    #[test]
    fn rejects_bad_direction() {
        assert!(matches!(
            parse_command("go nowhere"),
            Err(CommandError::BadDirection(_))
        ));
    }

    #[test]
    fn rejects_missing_object() {
        assert!(matches!(
            parse_command("take"),
            Err(CommandError::MissingObject { .. })
        ));
    }

    #[test]
    fn rejects_empty() {
        assert!(matches!(parse_command(""), Err(CommandError::Empty)));
        assert!(matches!(parse_command("   "), Err(CommandError::Empty)));
    }
}
```

Run:

```bash
cargo test
```

```
running 7 tests
test tests::parses_look ... ok
test tests::parses_directions_bare ... ok
test tests::parses_multi_word_pickup ... ok
test tests::rejects_unknown_verb ... ok
test tests::rejects_bad_direction ... ok
test tests::rejects_missing_object ... ok
test tests::rejects_empty ... ok

test result: ok. 7 passed; 0 failed
```

We meet testing properly on Day 13 — today's a preview.

### The `matches!` macro

`matches!(value, pattern)` returns `true` if the pattern matches. Cleaner than `if let Some(_) = ...` when you just need a yes/no.

`matches!(r, Ok(Action::Take(ref s)) if s == "rusty sword")` — you can put a `match` guard inside `matches!`. `ref s` binds `s` by reference, letting us compare without moving.

## Step 6 — anyhow in main

Now the application glue. `main` should return `anyhow::Result<()>` so we can use `?` for anything that goes wrong.

```rust
use anyhow::Context;
use std::io::{self, Write, BufRead};

fn read_line() -> Option<String> {
    let stdin = io::stdin();
    let mut line = String::new();
    match stdin.lock().read_line(&mut line) {
        Ok(0) => None,
        Ok(_) => Some(line.trim().to_string()),
        Err(_) => None,
    }
}

fn main() -> anyhow::Result<()> {
    println!("Adventure REPL. Type commands. Type 'quit' to exit.\n");

    loop {
        print!("> ");
        io::stdout().flush().ok();

        let input = match read_line() {
            Some(s) => s,
            None => break,
        };

        match parse_command(&input) {
            Ok(Action::Quit) => break,
            Ok(action) => handle_action(action)
                .with_context(|| format!("while handling command: {:?}", input))?,
            Err(e) => println!("Error: {}", e),
        }
    }

    println!("Farewell.");
    Ok(())
}

fn handle_action(action: Action) -> anyhow::Result<()> {
    match action {
        Action::Look => println!("You look around."),
        Action::Go(dir) => println!("You go {:?}.", dir),
        Action::Take(obj) => println!("You take the {}.", obj),
        Action::Drop(obj) => println!("You drop the {}.", obj),
        Action::Inventory => println!("Inventory: (empty)"),
        Action::Attack(target) => println!("You attack the {}.", target),
        Action::Save(path) => {
            std::fs::write(&path, "dummy save").with_context(|| {
                format!("could not save to {}", path.display())
            })?;
            println!("Saved to {}.", path.display());
        }
        Action::Load(path) => {
            let contents = std::fs::read_to_string(&path).with_context(|| {
                format!("could not load from {}", path.display())
            })?;
            println!("Loaded: {}", contents);
        }
        Action::Quit => unreachable!("handled in main"),
    }
    Ok(())
}
```

### What's happening in `main`

- `main() -> anyhow::Result<()>` lets `?` propagate any error, printing a nice trace on program exit.
- `with_context(|| ...)` attaches a descriptive message to any error that bubbles up.
- Parsing errors (`CommandError`) are handled *in the loop* — they don't crash the REPL, they just print a message and continue.
- Action errors (I/O failures) bubble up, get context attached, and propagate. If I/O fails, the REPL exits and main prints the chain.

### Try it

```
> save /root/no-perms.sav
Error: while handling command: "save /root/no-perms.sav"

Caused by:
    0: could not save to /root/no-perms.sav
    1: Permission denied (os error 13)
```

Three levels of context. The user sees what they did, what was attempted, and what actually failed.

## Step 7 — Refactoring: move parsing into a module

Parser code is a great candidate for its own module. Create `src/parser.rs`:

```rust
// src/parser.rs

use thiserror::Error;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    North, South, East, West, Up, Down,
}

impl std::str::FromStr for Direction {
    type Err = CommandError;
    fn from_str(s: &str) -> Result<Direction, CommandError> {
        // ... as before
    }
}

#[derive(Debug)]
pub enum Action { /* ... as before */ }

#[derive(Debug, Error)]
pub enum CommandError { /* ... as before */ }

pub fn parse_command(input: &str) -> Result<Action, CommandError> {
    // ... as before
}

#[cfg(test)]
mod tests { /* ... as before */ }
```

In `main.rs`:

```rust
mod parser;
use parser::{parse_command, Action, Direction};
// ...
```

Now `parser` is a clean, focused module with its own typed error. `main.rs` consumes it as if it were an external library. This separation is how you'd structure a real crate.

## Common pitfalls

### Forgetting `#[derive(Error)]`

Without the derive, `thiserror`'s magic doesn't happen — you'll get "CommandError doesn't implement Display" from the compiler. The fix is just to remember the derive.

### `#[from]` requires exactly one field

```rust
#[error("...")]
Io(#[from] io::Error),          // ok
Both(#[from] A, B),             // ERROR: From impl has one input
```

If you want `From` for a wrapped error that needs extra data, write the `impl From` yourself.

### `anyhow::Result<T>` is just `Result<T, anyhow::Error>`

Don't be confused by it looking like a separate type. It's a type alias.

### `?` moves the error

```rust
let r = some_fallible();
match r { ... }    // r is still usable
let x = some_fallible()?;    // r's ownership is gone
```

`?` both unwraps on success and returns on error; the original `Result` is consumed. Usually what you want.

### Forgetting `to_string()` in tests

```rust
assert!(matches!(
    parse_command("dance"),
    Err(CommandError::UnknownVerb(s)) if s == "dance"
));
```

Compile error: `CommandError::UnknownVerb` holds a `String`, but the literal `"dance"` is `&str`. Fix with `&s == "dance"` (deref via PartialEq), or `s == "dance".to_string()`, or `s.as_str() == "dance"`. The `&s == "dance"` form is idiomatic.

## What you learned

- **`thiserror`** for library-style, typed, derivable error enums.
- **`anyhow`** for application-level error propagation with context.
- **`#[error("...")]`** for inline Display messages.
- **`#[from]`** for automatic `From` impls to compose errors.
- **`with_context(|| ...)`** to add a descriptive message to errors.
- Multi-word verb parsing with `strip_prefix`.
- `FromStr` impl to enable `.parse::<Type>()`.
- Module organization for clean separation of parsing from main.

## Exercises

1. **`Io` variant.** Add a `Io(#[from] std::io::Error)` variant to `CommandError`. Write a `parse_command_from_file(path)` that reads a file and parses the first line, demonstrating `?` converting `io::Error` to `CommandError` for free.
2. **`NoSuchExit` error.** The current parser accepts `go north` without checking whether the current room has a north exit. Add a `validate_action(action: &Action, state: &GameState) -> Result<(), CommandError>` that checks preconditions and returns `CommandError::NoSuchExit { direction: Direction, room: String }`.
3. **`anyhow::anyhow!` macro.** Use `anyhow::anyhow!("message here")` to create an ad-hoc error without defining a type. When is this nice? When is a typed error better?
4. **Walking the error chain.** At the top of your error-print in main, use `e.chain()` (from `anyhow`) to iterate the full cause chain and print each level on its own line.
5. **Replace Day 3's hand-written error.** Rewrite Day 3's `SaveError` using `thiserror`. Count the lines you save.

## What you learned this week

You've covered the core of Rust's type system:

- **Ownership, borrowing, and moves** — Day 1
- **Enums with data, pattern matching, state machines** — Day 2
- **`Option`, `Result`, and `?`** — Day 3
- **Traits, static and dynamic dispatch** — Day 4
- **Generics with trait bounds** — Day 5
- **Iterators, lazy evaluation, custom iterators** — Day 6
- **Rich errors with `thiserror` and `anyhow`** — Day 7

This is a solid base. Anything you build next will feel familiar. You've also built — gradually — a working text adventure with combat, save/load, and a typed parser.

## What's next

Week 2 starts with **closures** — anonymous functions that capture their environment. You'll build an event bus around them, then tackle Rust's two hardest language-level topics: explicit lifetimes (Day 9) and smart pointers (Day 10). By the end of the week you'll have a real-time Snake game running in your terminal.

→ Day 8 — Event bus with closures (coming in next installment)
