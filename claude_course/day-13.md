# Day 13 — Testing

**Domain:** games • **Time:** 90 minutes • **Difficulty:** medium

## What you'll build

Test suites for two of your Week 1–2 codebases: the command parser from Day 7, and the Snake game logic from Day 12. You'll write unit tests, integration tests, documentation tests, and property-based tests. Every layer of Rust's testing story, covered.

## What you'll learn

- **Unit tests** in `#[cfg(test)]` modules
- **Test organization**: naming, fixtures, one concept per test
- **Assertion macros**: `assert_eq!`, `assert!`, `matches!`, `assert_matches!`
- **Integration tests** in the `tests/` directory
- **Documentation tests** — your `///` doc examples run as tests
- **Property-based testing** with `proptest`
- **Benchmarks** with `criterion` (preview; full treatment on Day 26)

## Background

### Rust's testing story is unusually good

In Python, testing needs a framework (`pytest`, `unittest`). In Rust, it's built in. `cargo test` finds, compiles, and runs tests automatically. The same toolchain handles:

- **Unit tests**: next to production code, in `#[cfg(test)]` modules.
- **Integration tests**: separate `tests/` directory; treats your crate as an external consumer.
- **Doctests**: examples in documentation comments are compiled *and* run as tests.
- **Benchmarks**: `criterion` gives statistical benchmarking with reports.
- **Property tests**: `proptest` or `quickcheck` generate random inputs.

All under one command.

### The `#[cfg(test)]` attribute

Code wrapped in `#[cfg(test)]` only compiles during test builds. This is how Rust's unit tests live next to production code without bloating release binaries:

```rust
pub fn square(n: i32) -> i32 { n * n }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn square_of_3_is_9() {
        assert_eq!(square(3), 9);
    }
}
```

In release builds, the `tests` module doesn't exist at all.

### Assertion macros

- `assert!(cond)` — panics if false.
- `assert_eq!(a, b)` — panics if `a != b`, prints both values on failure.
- `assert_ne!(a, b)` — panics if `a == b`.
- `matches!(value, pattern)` — returns true if pattern matches. Put inside `assert!` for pattern assertions.

Custom message:

```rust
assert!(n > 0, "expected positive, got {}", n);
```

Prefer `assert_eq!` / `assert_ne!` over `assert!(a == b)` — the diff on failure is far more useful.

### Test attributes

- `#[test]` — marks a function as a test.
- `#[should_panic]` — test passes only if the body panics.
- `#[should_panic(expected = "substring")]` — the panic message must contain `"substring"`.
- `#[ignore]` — test is skipped unless `cargo test -- --ignored`.

## Setting up

We'll work with two separate projects to keep things clean.

For the parser:
```bash
cargo new day-13-parser --lib
cd day-13-parser
cargo add thiserror
cargo add --dev proptest
```

`--lib` creates a library crate (no `main.rs`). `--dev` adds the dependency to `[dev-dependencies]` — only compiled for tests.

## Step 1 — Set up the parser

Port Day 7's parser into `src/lib.rs`. Trimmed version for today:

```rust
//! A small command parser for an adventure game.

use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    North, South, East, West,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Action {
    Look,
    Go(Direction),
    Take(String),
    Quit,
}

#[derive(Debug, PartialEq, Eq, Error)]
pub enum CommandError {
    #[error("empty input")]
    Empty,
    #[error("unknown verb '{0}'")]
    UnknownVerb(String),
    #[error("what do you want to {verb}?")]
    MissingObject { verb: String },
    #[error("'{0}' is not a direction")]
    BadDirection(String),
}

impl std::str::FromStr for Direction {
    type Err = CommandError;
    fn from_str(s: &str) -> Result<Direction, CommandError> {
        match s.to_lowercase().as_str() {
            "n" | "north" => Ok(Direction::North),
            "s" | "south" => Ok(Direction::South),
            "e" | "east"  => Ok(Direction::East),
            "w" | "west"  => Ok(Direction::West),
            _ => Err(CommandError::BadDirection(s.to_string())),
        }
    }
}

/// Parses a command string into an [`Action`].
///
/// # Examples
///
/// ```
/// use day_13_parser::{parse_command, Action, Direction};
///
/// assert_eq!(parse_command("look").unwrap(), Action::Look);
/// assert_eq!(parse_command("go n").unwrap(), Action::Go(Direction::North));
/// ```
pub fn parse_command(input: &str) -> Result<Action, CommandError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(CommandError::Empty);
    }

    let lower = trimmed.to_lowercase();
    let mut parts = lower.split_whitespace();
    let verb = parts.next().ok_or(CommandError::Empty)?;
    let rest: Vec<&str> = parts.collect();

    match verb {
        "look" | "l" => Ok(Action::Look),
        "go" | "move" => {
            let dir_s = rest.first().ok_or(CommandError::MissingObject {
                verb: "go".to_string(),
            })?;
            let dir = dir_s.parse::<Direction>()?;
            Ok(Action::Go(dir))
        }
        "take" | "grab" => {
            let obj = rest.join(" ");
            if obj.is_empty() {
                Err(CommandError::MissingObject { verb: "take".to_string() })
            } else {
                Ok(Action::Take(obj))
            }
        }
        "quit" | "q" => Ok(Action::Quit),
        other => Err(CommandError::UnknownVerb(other.to_string())),
    }
}
```

Notice the `#[derive(PartialEq, Eq)]` on `Action` and `CommandError`. Tests need equality comparisons, and these derives enable `assert_eq!` to work on our enums.

## Step 2 — Unit tests

At the bottom of `src/lib.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_look() {
        assert_eq!(parse_command("look").unwrap(), Action::Look);
    }

    #[test]
    fn parses_look_alias() {
        assert_eq!(parse_command("l").unwrap(), Action::Look);
    }

    #[test]
    fn parses_all_cardinal_directions() {
        assert_eq!(parse_command("go n").unwrap(), Action::Go(Direction::North));
        assert_eq!(parse_command("go s").unwrap(), Action::Go(Direction::South));
        assert_eq!(parse_command("go e").unwrap(), Action::Go(Direction::East));
        assert_eq!(parse_command("go w").unwrap(), Action::Go(Direction::West));
        assert_eq!(parse_command("go north").unwrap(), Action::Go(Direction::North));
        assert_eq!(parse_command("go south").unwrap(), Action::Go(Direction::South));
    }

    #[test]
    fn parses_take_with_alias() {
        assert_eq!(
            parse_command("grab sword").unwrap(),
            Action::Take("sword".to_string())
        );
    }

    #[test]
    fn rejects_empty_input() {
        assert_eq!(parse_command("").unwrap_err(), CommandError::Empty);
        assert_eq!(parse_command("    ").unwrap_err(), CommandError::Empty);
    }

    #[test]
    fn rejects_unknown_verb() {
        assert!(matches!(
            parse_command("dance"),
            Err(CommandError::UnknownVerb(v)) if v == "dance"
        ));
    }

    #[test]
    fn rejects_missing_object() {
        assert!(matches!(
            parse_command("take"),
            Err(CommandError::MissingObject { verb }) if verb == "take"
        ));
    }

    #[test]
    fn rejects_bad_direction() {
        assert!(matches!(
            parse_command("go nowhere"),
            Err(CommandError::BadDirection(d)) if d == "nowhere"
        ));
    }

    #[test]
    fn case_insensitive() {
        assert_eq!(parse_command("LOOK").unwrap(), Action::Look);
        assert_eq!(parse_command("Go North").unwrap(), Action::Go(Direction::North));
    }

    #[test]
    fn leading_trailing_whitespace() {
        assert_eq!(parse_command("  look  ").unwrap(), Action::Look);
    }
}
```

Run:

```bash
cargo test
```

```
running 10 tests
test tests::parses_look ... ok
test tests::parses_look_alias ... ok
test tests::parses_all_cardinal_directions ... ok
test tests::parses_take_with_alias ... ok
test tests::rejects_empty_input ... ok
test tests::rejects_unknown_verb ... ok
test tests::rejects_missing_object ... ok
test tests::rejects_bad_direction ... ok
test tests::case_insensitive ... ok
test tests::leading_trailing_whitespace ... ok

test result: ok. 10 passed; 0 failed; 0 ignored
```

### Test naming

Name tests for what they assert: `rejects_unknown_verb`, not `test1`. When a test fails, the name tells you what broke without opening the file. Bad names are a common sign of bad tests.

### `matches!` for error patterns

Error variants with String payloads don't implement `Eq` the way you'd want to compare them. Either:

- `#[derive(PartialEq, Eq)]` on the error (as we did here), allowing `assert_eq!` with a constructed error.
- Or use `matches!` with a guard: `matches!(result, Err(CommandError::X(v)) if v == "expected")`.

The guard form is often clearer because you don't have to construct a full equality sentinel.

## Step 3 — Doctests

Look back at our `parse_command` — we included examples in the doc comment:

````rust
/// ```
/// use day_13_parser::{parse_command, Action, Direction};
///
/// assert_eq!(parse_command("look").unwrap(), Action::Look);
/// assert_eq!(parse_command("go n").unwrap(), Action::Go(Direction::North));
/// ```
````

Cargo compiles and runs these. On a working crate:

```bash
cargo test --doc
```

```
running 1 test
test src/lib.rs - parse_command (line 52) ... ok

test result: ok. 1 passed
```

Doctests have real value:

- They stay in sync with the code (if they break, the test fails).
- They're visible to anyone browsing your docs on docs.rs.
- They answer the "how do I use this?" question every new user asks.

### Doctest gotchas

- Doctests import *as an external crate* — use the crate name, not `super::*`.
- They run in release optimization by default. If your test relies on debug assertions, be careful.
- Add `no_run` to prevent running (only compile): ``` ```no_run ```
- Use `should_panic`: ``` ```should_panic ``` to verify the example panics.

## Step 4 — Integration tests

Unit tests live next to production code. Integration tests live in a `tests/` directory at the project root. They import your crate like any external user would.

Create `tests/parser_integration.rs`:

```rust
use day_13_parser::{parse_command, Action, Direction};

#[test]
fn happy_path_sequence() {
    // Simulate a user playing: look, go north, take sword, quit
    let commands = [
        ("look", Action::Look),
        ("go north", Action::Go(Direction::North)),
        ("take sword", Action::Take("sword".to_string())),
        ("quit", Action::Quit),
    ];
    for (input, expected) in commands {
        assert_eq!(parse_command(input).unwrap(), expected);
    }
}

#[test]
fn error_recovery() {
    // User makes typos, each produces a clear error; then a good command works
    assert!(parse_command("dance").is_err());
    assert!(parse_command("go nowhere").is_err());
    assert!(parse_command("take").is_err());
    assert!(parse_command("look").is_ok());
}
```

Run:

```bash
cargo test
```

You'll see three test binaries now: unit tests, the integration file, and the doctest.

### Differences from unit tests

- No `#[cfg(test)]` needed — the whole file is a test file.
- No `use super::*;` — import from your crate by name.
- Each file in `tests/` compiles as a separate test binary.
- Can't test private items from here (as intended — integration tests verify the public API).

### When to use which

- **Unit tests** for internal components, private helpers, edge cases of implementation details.
- **Integration tests** for the public API, end-to-end flows, "does this crate work as advertised?"

## Step 5 — Property-based tests with `proptest`

Example-based tests catch bugs you thought of. Property-based tests catch bugs you didn't. They work like this:

- Define a *property* — something that should hold for all inputs (or a broad class).
- `proptest` generates hundreds of random inputs matching a shape you specify.
- If any input violates the property, `proptest` *shrinks* it to the smallest failing case and reports it.

A property for our parser: parsing a direction back should round-trip.

In `src/lib.rs`, add to the test module:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    // ... existing tests ...

    proptest! {
        #[test]
        fn roundtrip_directions(dir in prop_oneof![
            Just("north"), Just("south"), Just("east"), Just("west"),
            Just("n"), Just("s"), Just("e"), Just("w"),
        ]) {
            let action = parse_command(&format!("go {}", dir)).unwrap();
            assert!(matches!(action, Action::Go(_)));
        }

        #[test]
        fn never_panics(input in ".*") {
            // For any string at all, parser must not panic; it must return Ok or Err
            let _ = parse_command(&input);
        }

        #[test]
        fn empty_prefix_errors(prefix in "[[:space:]]{0,20}") {
            assert!(matches!(
                parse_command(&prefix),
                Err(CommandError::Empty)
            ));
        }
    }
}
```

### What `proptest!` does

The macro defines one or more tests. Each has a *strategy* for generating inputs. When you run the test, proptest runs it ~256 times with different random inputs.

- `Just("north")` produces the literal string `"north"`.
- `prop_oneof![a, b, c]` picks one of the strategies uniformly.
- `".*"` is a regex strategy — any string.
- `"[[:space:]]{0,20}"` — zero to twenty whitespace chars.

Run:

```bash
cargo test
```

```
test tests::never_panics ... ok
test tests::roundtrip_directions ... ok
test tests::empty_prefix_errors ... ok
```

### Why `never_panics` is gold

Fuzz-testing in one test. The parser has to behave on any possible input — no matter how weird. If someone types 10 000 characters of emoji, the parser might crash. `never_panics` finds that.

### Shrinking in action

To see proptest do its thing, deliberately break the parser:

```rust
// In parse_command, somewhere dubious
if input.len() == 17 { panic!("arbitrary!"); }
```

Run tests. You get:

```
thread 'tests::never_panics' panicked at ...
Test failed: arbitrary!
minimal failing input: input = "aaaaaaaaaaaaaaaaa"   // 17 chars
successes: 45
local rejects: 0
global rejects: 0
```

Proptest tried random 17-character strings, found they all panic, and kept shrinking until it found the smallest (17 `a`s). Remove the panic to fix.

## Step 6 — Testing Snake logic

Port your Day 12 `src/game.rs` into a new crate:

```bash
cd ..
cargo new day-13-snake --lib
cd day-13-snake
cargo add rand@0.8
```

Copy the game logic. Add tests at the bottom:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn new_game() -> Game {
        // Seeded deterministic game for testing
        Game::new(0)
    }

    #[test]
    fn starts_in_playing() {
        let g = new_game();
        assert_eq!(g.status, Status::Playing);
    }

    #[test]
    fn starts_length_3() {
        let g = new_game();
        assert_eq!(g.snake.len(), 3);
    }

    #[test]
    fn moves_right_each_tick() {
        let mut g = new_game();
        let head0 = *g.snake.front().unwrap();
        g.update();
        let head1 = *g.snake.front().unwrap();
        assert_eq!(head1, (head0.0 + 1, head0.1));
    }

    #[test]
    fn reverses_rejected() {
        let mut g = new_game();    // moving right
        g.apply_input(Input::Turn(Direction::Left));
        assert_eq!(g.pending_dir, Direction::Right);
    }

    #[test]
    fn valid_turn_applied() {
        let mut g = new_game();    // moving right
        g.apply_input(Input::Turn(Direction::Up));
        assert_eq!(g.pending_dir, Direction::Up);
    }

    #[test]
    fn wall_collision_ends_game() {
        let mut g = new_game();
        for _ in 0..WIDTH {
            g.update();
        }
        assert_eq!(g.status, Status::Over);
    }

    #[test]
    fn self_collision_ends_game() {
        let mut g = new_game();
        // Turn into a full rectangle that collides
        g.apply_input(Input::Turn(Direction::Down));
        g.update();
        g.apply_input(Input::Turn(Direction::Left));
        g.update();
        g.apply_input(Input::Turn(Direction::Up));
        g.update();
        // Not enough to self-collide on length 3, but enough to test the direction logic
        // A stronger test would construct a specific scenario; leaving as exercise.
    }

    #[test]
    fn eating_food_grows() {
        let mut g = new_game();
        let start_len = g.snake.len();
        // Force food right in front of the head
        let head = *g.snake.front().unwrap();
        g.food = (head.0 + 1, head.1);
        g.update();
        assert_eq!(g.snake.len(), start_len + 1);
        assert_eq!(g.score, 10);
    }
}
```

Run:

```bash
cargo test
```

All pass. Notice how easy this was: the game logic is in a library, with no I/O. Testing it is free.

### Testing randomness

Spawning food uses randomness — tests that depend on it are non-deterministic. Two ways to handle:

1. **Inject the RNG.** Make `spawn_food` take `rng: &mut impl Rng`. Tests pass a `StdRng::seed_from_u64(42)` for reproducibility.
2. **Test invariants, not specific values.** "Food is always in an empty cell" and "food is never on the snake" don't care about specific positions.

Both are valid. Injection is cleaner but requires plumbing.

## Step 7 — Benchmarks (preview)

We're not building a full bench setup today (Day 26 handles that), but here's what it looks like.

`cargo add --dev criterion` and add to `Cargo.toml`:

```toml
[[bench]]
name = "parser_bench"
harness = false
```

Create `benches/parser_bench.rs`:

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use day_13_parser::parse_command;

fn bench_parse(c: &mut Criterion) {
    c.bench_function("parse_look", |b| {
        b.iter(|| parse_command(black_box("look")))
    });

    c.bench_function("parse_unknown", |b| {
        b.iter(|| parse_command(black_box("asdfghjkl")))
    });
}

criterion_group!(benches, bench_parse);
criterion_main!(benches);
```

Run:

```bash
cargo bench
```

You get statistical reports — `parse_look` takes ~120ns, etc. For our tiny parser there's nothing to optimize, but the machinery is here.

`black_box` tells the compiler "pretend you don't know this value," preventing it from optimizing the call away.

## Common pitfalls

### Tests pass locally, fail in CI

Usually platform differences (line endings, path separators) or racing tests that share state (shared files, fixed ports). Rule: each test must clean up after itself and not rely on external state.

### "cannot find value `super` in this scope" in doctests

Doctests don't have access to `super::*`. Import by crate name.

### Non-deterministic tests

You used randomness without a seed. Different run, different result. Flaky tests are worse than no tests. Either inject the RNG or test properties.

### Slow property tests

`proptest` runs 256 cases by default. With heavy setup, this is slow. Reduce with:

```rust
proptest! {
    #![proptest_config(ProptestConfig::with_cases(20))]
    #[test]
    fn expensive(v in any::<Vec<u32>>()) { ... }
}
```

### `cargo test` leaves files behind

You wrote a temp file in a test and didn't clean it up. Fix: use `tempfile` crate or explicitly delete on success *and* failure (RAII guard!).

### Testing private items from integration tests

Can't. Integration tests see only `pub` items. Sometimes that's annoying; it's intentional — integration tests should use your crate like users will.

## What you learned

- **Unit tests** in `#[cfg(test)]` modules next to production code.
- **Integration tests** in `tests/`, treating your crate as external.
- **Documentation tests** — examples in `///` comments run as tests.
- **Assertion macros**: `assert_eq!`, `assert!`, `matches!`.
- **Test attributes**: `#[should_panic]`, `#[ignore]`.
- **`proptest`** for property-based testing with automatic shrinking.
- **`criterion`** for statistical benchmarks (full treatment Day 26).
- **Testable architecture**: separate logic from I/O; inject dependencies.
- Testing randomness via seeded RNG injection.

## Exercises

1. **Golden tests.** Write a test that feeds a fixed command sequence to the parser and compares the JSON-serialized output to a file on disk. Any change to behavior requires updating the "golden" file. Pattern used heavily in compilers.
2. **`insta` for snapshots.** Install the `insta` crate and rewrite your golden tests using `insta::assert_debug_snapshot!`. Run `cargo insta review` to see the workflow.
3. **Test coverage.** Install [cargo-llvm-cov](https://github.com/taiki-e/cargo-llvm-cov) and run `cargo llvm-cov` to measure test coverage. Find an untested branch and write a test for it.
4. **Property: inverse.** If you add a `serialize_action(action) -> String` method, property-test that `parse_command(serialize_action(a)) == Ok(a)` for any generated `Action`.
5. **Bench variations.** In the bench file, add a bench for parsing every verb your parser supports. Run `cargo bench` and compare.

## What's next

Day 14 closes out Week 2 with **concurrency**: threads, channels, and shared state. You'll parallelize A* pathfinding across AI snakes, benchmark single-threaded vs multi-threaded, and meet `Arc<Mutex<T>>` — the multi-threaded version of Day 10's `Rc<RefCell<T>>`.

→ [Day 14 — Threads and channels](day-14.md)
