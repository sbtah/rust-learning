# Day 9 — Explicit Lifetimes

**Domain:** games • **Time:** 90–120 minutes • **Difficulty:** hard

## What you'll build

A zero-copy parser for a simple INI-like config format. You read a whole file into a `String` once, parse it into a `Config<'a>` where every key and value is a `&str` slice pointing *into* that original string. No `to_string()`, no `String` allocations for content. The whole parsed config borrows from the input. The compiler won't let you drop the input while the config is still alive — and enforcing that is what lifetimes are for.

## What you'll learn

- **What lifetimes really are** — and what they aren't
- **Lifetime annotations** on structs, functions, and impls
- **Lifetime elision rules** — when the compiler infers, when it can't
- **The borrow checker's reasoning**, made explicit
- Zero-copy parsing: the idiomatic Rust way
- When to bail out and just allocate a `String`

## Background

### What is a lifetime?

A lifetime is the span during which a reference is valid. Every reference in Rust has a lifetime — the compiler tracks it, even when you don't write it down.

```rust
let s = String::from("hello");   // s lives from here...
let r = &s;                      // r borrows s
println!("{}", r);               //   until here, where the borrow is last used
```

The lifetime of `r` is the region of code where `r` can be used. If `r` were still alive when `s` was dropped, we'd have a dangling reference — a use-after-free. The compiler prevents this.

Most of the time you don't write lifetimes because the compiler infers them via **elision rules**. We write them when:

- A struct holds a reference.
- A function's output references something whose source isn't obvious.
- We want to be more specific than the defaults.

### Lifetime syntax: `'a`

Lifetimes are named with an apostrophe prefix: `'a`, `'b`, `'input`. The names are *your* choice; `'a` is the convention for the first/only one. Think of them like generic type parameters, but for lifetimes — they describe how long a reference lasts.

```rust
// "Takes a reference that lives at least as long as 'a,
//  returns a reference with that same lifetime 'a."
fn first<'a>(s: &'a str) -> &'a str {
    &s[..1]
}
```

### Elision rules

In function signatures, the compiler applies these rules to fill in missing lifetimes:

1. Each elided *input* reference gets its own lifetime (`&str` → `&'a str`, `&&mut i32` → `&'a &'b mut i32`).
2. If there's **exactly one input lifetime**, every elided output lifetime equals it.
3. If one of the inputs is `&self` or `&mut self`, every elided output lifetime equals *that* lifetime.

Where these cover everything, you write no lifetimes. Where they don't, you do.

### The special lifetime `'static`

`'static` means "lives for the entire program." String literals are `&'static str` — they're baked into the binary, never freed.

```rust
let s: &'static str = "hello";
```

A bound like `T: 'static` means "T doesn't borrow anything with a shorter lifetime." It's what we wrote on Day 8's event handlers.

### What lifetimes are *not*

- They don't change runtime behavior. They're purely a type-level annotation.
- They don't extend a value's life. You can't "make it live longer" by annotating.
- They describe *constraints* — relationships between references — not durations.

### The Python comparison

There is no Python analogue. Python's GC keeps things alive as long as something holds them; lifetimes simply aren't a concept in the language. In Rust, the compiler needs to know "does this reference outlive the thing it points to?" — and the answer comes from lifetime annotations.

## Setting up

```bash
cargo new day-09
cd day-09
```

No dependencies.

## Step 1 — Decide what we're parsing

A config format:

```
# server settings
host = localhost
port = 8080
name = "Alice's server"
debug = true

# client
retries = 3
```

Rules:

- Blank lines ignored.
- Lines starting with `#` ignored.
- Otherwise: `key = value`, with optional whitespace. Value may be quoted to include spaces.

## Step 2 — The "easy" allocating version (and why we reject it)

The most obvious parser:

```rust
pub fn parse(source: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for line in source.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((k, v)) = line.split_once('=') {
            out.push((k.trim().to_string(), v.trim().to_string()));
        }
    }
    out
}
```

This works. And for most apps it's fine.

But consider: if your source is a 10 MB config file with 50 000 entries, you're now holding the whole thing *twice* — once as the original bytes, once more as thousands of little `String` allocations. Plus the allocator overhead, plus cache misses.

Zero-copy says: keep the source buffer, and represent the parsed data as pointers into it.

### The catch

The parsed data now *depends on* the source. If the source goes out of scope, every `&str` in the parsed output becomes a dangling pointer. We need to teach the compiler about this dependency — and that's exactly what lifetime annotations do.

## Step 3 — Define `Config<'a>`

```rust
pub struct Config<'a> {
    entries: Vec<(&'a str, &'a str)>,
}
```

**Read this as:** "A `Config` parameterized by a lifetime `'a`. Internally it holds pairs of string slices, each living at least as long as `'a`."

The struct definition *declares* the lifetime parameter. Anywhere we write `Config<...>`, we have to fill in the `'a` — though often the compiler infers it.

### What does `Config<'a>` mean operationally?

A `Config<'a>` is only valid during `'a`. If you try to use it after the data it borrows from is gone, the compiler stops you.

## Step 4 — Parse

```rust
impl<'a> Config<'a> {
    pub fn parse(source: &'a str) -> Result<Config<'a>, ParseError> {
        let mut entries = Vec::new();
        for (idx, raw) in source.lines().enumerate() {
            let line_num = idx + 1;
            let line = raw.trim();

            // Skip blank and comment lines
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            let (key, value) = line.split_once('=').ok_or(ParseError::MissingEquals {
                line: line_num,
            })?;

            let key = key.trim();
            if key.is_empty() {
                return Err(ParseError::EmptyKey { line: line_num });
            }

            let value = parse_value(value.trim(), line_num)?;
            entries.push((key, value));
        }

        Ok(Config { entries })
    }

    pub fn get(&self, key: &str) -> Option<&'a str> {
        self.entries
            .iter()
            .find(|(k, _)| *k == key)
            .map(|(_, v)| *v)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&'a str, &'a str)> + '_ {
        self.entries.iter().copied()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }
}
```

And the helpers:

```rust
#[derive(Debug)]
pub enum ParseError {
    MissingEquals { line: usize },
    EmptyKey { line: usize },
    UnterminatedQuote { line: usize },
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::MissingEquals { line } => write!(f, "line {}: missing '='", line),
            ParseError::EmptyKey { line } => write!(f, "line {}: empty key", line),
            ParseError::UnterminatedQuote { line } => write!(f, "line {}: unterminated quote", line),
        }
    }
}

impl std::error::Error for ParseError {}

fn parse_value<'a>(raw: &'a str, line: usize) -> Result<&'a str, ParseError> {
    if raw.starts_with('"') {
        if !raw.ends_with('"') || raw.len() < 2 {
            return Err(ParseError::UnterminatedQuote { line });
        }
        Ok(&raw[1..raw.len() - 1])
    } else {
        Ok(raw)
    }
}
```

This is the real content of today. Let's unpack the lifetimes piece by piece.

### `impl<'a> Config<'a>`

We declare the lifetime parameter on the `impl`, then use it on the type. Symmetric with `impl<T> Vec<T>`.

### `pub fn parse(source: &'a str) -> Result<Config<'a>, ParseError>`

Two `'a`s here. They're the **same lifetime**. Reading the signature:

> *Given a string slice that lives for some lifetime `'a`, I return a `Config` that also lives for `'a`. In other words, the returned `Config` borrows from `source`.*

This is the whole point. It's what makes the compiler enforce the safety invariant.

### `pub fn get(&self, key: &str) -> Option<&'a str>`

Notice: the return type is `Option<&'a str>` — **not** `Option<&str>`. If you wrote `Option<&str>`, elision rule 3 would kick in and it would mean `Option<&'self str>` — "lives as long as `&self`." That would be wrong: we want the returned value to live as long as the original source, not as long as the borrow of `self`.

So we must write `'a` explicitly.

This is important enough to repeat: **if a method returns a reference to data your struct borrows from, annotate the returned lifetime with the struct's lifetime, not the `&self` lifetime.**

### `pub fn iter(&self) -> impl Iterator<Item = (&'a str, &'a str)> + '_`

A lot going on:

- `impl Iterator<Item = (&'a str, &'a str)>` — "some iterator yielding tuple pairs with lifetime `'a`."
- `+ '_` — the iterator itself borrows from `&self` (for the internal `Iter` it holds). `'_` means "the compiler figures out which lifetime — probably `&self`'s."

You often need the `+ '_` when returning `impl Trait` from a method — without it, the compiler assumes `+ 'static`, which is wrong when the iterator holds a borrow.

### `.copied()` on the iterator

`self.entries.iter()` yields `&(&'a str, &'a str)`. That's a reference to a tuple of references. We want just the tuple of references. `.copied()` derefs, giving `(&'a str, &'a str)` directly. Works because `&str` is `Copy`.

## Step 5 — Try it

```rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let source = r#"
# server config
host = localhost
port = 8080
name = "Alice's server"

# client
retries = 3
"#;

    let cfg = Config::parse(source)?;
    println!("Parsed {} entries", cfg.len());

    println!("host    = {:?}", cfg.get("host"));
    println!("port    = {:?}", cfg.get("port"));
    println!("name    = {:?}", cfg.get("name"));
    println!("retries = {:?}", cfg.get("retries"));
    println!("missing = {:?}", cfg.get("missing"));

    println!("All entries:");
    for (k, v) in cfg.iter() {
        println!("  {} = {}", k, v);
    }

    Ok(())
}
```

Run:

```
Parsed 5 entries
host    = Some("localhost")
port    = Some("8080")
name    = Some("Alice's server")
retries = Some("3")
missing = None
All entries:
  host = localhost
  port = 8080
  name = Alice's server
  retries = 3
```

## Step 6 — Watch the compiler enforce the invariant

Here's what makes it all worth it. Add this at the end of `main`:

```rust
let cfg = {
    let source_inner = String::from("host = localhost");
    let c = Config::parse(&source_inner)?;
    println!("got: {:?}", c.get("host"));
    c                                    // try to return c
};

println!("{:?}", cfg.get("host"));       // would read freed memory
```

Compile:

```
error[E0597]: `source_inner` does not live long enough
   |
   |         let source_inner = String::from("host = localhost");
   |             ------------ binding `source_inner` declared here
   |         let c = Config::parse(&source_inner)?;
   |                               ------------- borrow of `source_inner` occurs here
   ...
   |         c
   |         - returning this value requires that `source_inner` is borrowed for `'1`
   |     };
   |     - `source_inner` dropped here while still borrowed
```

This is the error that makes lifetime annotations worth writing. The compiler proved, before you even ran the program, that returning `c` here would use freed memory. It caught the bug at compile time, gave you an exact line, and told you what borrow constraint was violated.

Delete the problematic code and continue.

## Step 7 — Another demonstration: drop order

```rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let source = String::from("port = 8080");
    let cfg = Config::parse(&source)?;

    println!("{:?}", cfg.get("port"));    // ok

    drop(source);                          // try to free the source

    println!("{:?}", cfg.get("port"));    // would be UB
    Ok(())
}
```

```
error[E0505]: cannot move out of `source` because it is borrowed
   |
   |     let cfg = Config::parse(&source)?;
   |                              ------- borrow of `source` occurs here
   ...
   |     drop(source);
   |     ^^^^^^^^^^^^ move out of `source` occurs here
   ...
   |     println!("{:?}", cfg.get("port"));
   |                      ------ borrow later used here
```

Again the compiler stops you. `cfg` borrows from `source`; `drop(source)` would move it (ending its life); we're about to use `cfg` afterwards — rejected.

Delete the `drop` and the last `println!`, or reorder so the last use of `cfg` comes before freeing `source`.

## Step 8 — Lifetime elision, seen clearly

Go back to `parse_value`:

```rust
fn parse_value<'a>(raw: &'a str, line: usize) -> Result<&'a str, ParseError> {
    // ...
}
```

Could we elide the `'a`? Yes. Apply the rules:

- **Rule 1**: Each input reference gets its own lifetime. So `raw: &'a str`.
- **Rule 2**: Exactly one input lifetime → every elided output lifetime equals it.

So this works:

```rust
fn parse_value(raw: &str, line: usize) -> Result<&str, ParseError> {
    // ...
}
```

Identical meaning. The `'a` version is more explicit; the elided one is more idiomatic. Prefer the elided form when rule 2 or 3 gives the same result.

### When elision isn't enough

Consider a function that takes *two* string slices and returns one:

```rust
fn longer(a: &str, b: &str) -> &str {
    if a.len() > b.len() { a } else { b }
}
```

Compile error:

```
error[E0106]: missing lifetime specifier
   |
   | fn longer(a: &str, b: &str) -> &str {
   |              ----     ----     ^ expected named lifetime parameter
```

Rule 2 fails — there are two input lifetimes. The compiler doesn't know whether the returned `&str` should live as long as `a`'s or `b`'s. You have to say:

```rust
fn longer<'a>(a: &'a str, b: &'a str) -> &'a str {
    if a.len() > b.len() { a } else { b }
}
```

"Both inputs and the output all have lifetime `'a`." That means: the caller decides on some `'a` that fits both inputs (Rust will pick the shorter of the two), and the output is only valid as long as `'a`.

## Step 9 — When to give up and allocate

Sometimes the borrowing dance gets too painful. If you find yourself writing:

- `Config<'a, 'b, 'c>` with three lifetime parameters just to satisfy the compiler,
- Or fighting to thread one lifetime through dozens of functions,
- Or constantly re-parsing because you can't store `Config` somewhere,

stop and allocate. `to_string()` is not a sin. The price is real (allocation + copy), but not usually prohibitive.

Zero-copy is valuable for:

- Parsers where the input is large and the access pattern is read-mostly.
- Performance-critical paths.
- Interop with bytes-on-disk (memory-mapped files — Day 19!).

Don't force it where it's a burden.

## Step 10 — A lifecycle-driven bonus: tokenizer with borrowed state

A lazy iterator parser, where each line yields a borrowed pair:

```rust
pub fn parse_iter<'a>(source: &'a str) -> impl Iterator<Item = Result<(&'a str, &'a str), ParseError>> + 'a {
    source.lines().enumerate().filter_map(|(idx, raw)| {
        let line_num = idx + 1;
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            return None;
        }
        let (k, v) = match line.split_once('=') {
            Some((k, v)) => (k.trim(), v.trim()),
            None => return Some(Err(ParseError::MissingEquals { line: line_num })),
        };
        if k.is_empty() {
            return Some(Err(ParseError::EmptyKey { line: line_num }));
        }
        let v = match parse_value(v, line_num) {
            Ok(v) => v,
            Err(e) => return Some(Err(e)),
        };
        Some(Ok((k, v)))
    })
}
```

Use it:

```rust
for result in parse_iter(source) {
    match result {
        Ok((k, v)) => println!("  {} = {}", k, v),
        Err(e) => eprintln!("  ! {}", e),
    }
}
```

This doesn't allocate a `Vec` at all. Streams through the source once. For huge files, this is the pattern you want.

### The `+ 'a` at the end

`impl Iterator<...> + 'a` — "the returned iterator itself lives as long as `'a`." Needed because the iterator internally holds a borrow into `source`, which lives for `'a`. Without the `+ 'a`, Rust would assume `+ 'static`, see the inner borrows, and reject the signature.

## Common pitfalls

### "Explicit lifetime required" on a struct

```rust
struct Holder {
    s: &str,
}
```

Error: "expected named lifetime parameter." Structs holding references must declare the lifetime:

```rust
struct Holder<'a> {
    s: &'a str,
}
```

### `impl` block forgets the lifetime

```rust
struct Holder<'a> { s: &'a str }

impl Holder {                      // ERROR: missing 'a
    fn get(&self) -> &str { self.s }
}
```

Fix:

```rust
impl<'a> Holder<'a> {
    fn get(&self) -> &'a str { self.s }
}
```

### Using `&self`'s lifetime when you meant the struct's

```rust
impl<'a> Holder<'a> {
    fn get(&self) -> &str { self.s }    // returns &'self str due to elision
}
```

This compiles but over-constrains callers. They can't hold the returned `&str` longer than they hold `&self`. Fix:

```rust
fn get(&self) -> &'a str { self.s }
```

### "Cannot return value referencing local variable"

```rust
fn make() -> Config<'?> {
    let source = String::from("...");
    Config::parse(&source).unwrap()
}
```

There's no lifetime that works here — `source` dies at the end of `make`, and the returned `Config` must outlive the return. Fix: the caller owns the source, and `make` takes `&str` or is refactored to return `String` plus `Config` via an owning wrapper (the `ouroboros` crate exists precisely to package this pattern).

### "Borrowed value does not live long enough" in a loop

```rust
let mut configs = Vec::new();
for path in paths {
    let source = std::fs::read_to_string(path)?;     // new allocation each iteration
    let cfg = Config::parse(&source)?;
    configs.push(cfg);                                // ERROR: cfg outlives source
}
```

`source` dies at the end of each loop iteration; `cfg` borrows from it. Pushing `cfg` into `configs` (which outlives the iteration) is rejected. Fix: store both — keep the source alive alongside the config. You're reaching the point where zero-copy is more trouble than benefit; consider allocating.

## What you learned

- **Lifetimes** describe how long references are valid; the compiler tracks them always.
- **Elision rules** let you skip annotations in common cases.
- Functions returning references from inputs: annotate if rule 2 doesn't apply.
- Structs holding references: always annotate.
- `Config<'a>` — the struct borrows from something lasting at least `'a`.
- The compiler enforces that a `Config<'a>` can't outlive its source.
- `+ '_` and `+ 'a` on `impl Trait` returns when the returned type borrows.
- Zero-copy is valuable but optional — allocating is fine when borrows get unwieldy.

## Exercises

1. **Owned version.** Write an `OwnedConfig` struct that holds the source `String` plus the parsed entries (as `Vec<(String, String)>`). Users hand you a `String`; you parse and keep both. No lifetime parameter needed. Compare ergonomics.
2. **Streaming large files.** Use `parse_iter` to scan a 10 MB generated config file. Measure memory usage (e.g., with `ps` or `/proc/self/statm`) — it should stay tiny even for huge inputs.
3. **`'static` configs.** If the source is a string literal (`&'static str`), your parsed `Config<'static>` can be stored globally. Use `std::sync::OnceLock<Config<'static>>` to build a process-wide singleton from a compile-time string.
4. **Section support.** Extend the format with `[section]` headers. Keep zero-copy — section names are also `&'a str`.
5. **Borrowed keys in lookup.** `get(&self, key: &str)` returns `Option<&'a str>`. Could you also accept a `&[u8]` and handle UTF-8 validation without allocation? (Hint: `std::str::from_utf8`.)

## What's next

Day 10 shifts from references to owned data structures with shared ownership: **smart pointers**. `Box`, `Rc`, `RefCell`, `Weak`. You'll build a scene graph — a tree of 3D nodes with parents and children — and see exactly why Rust has multiple "pointer-like" types rather than one do-it-all.

→ [Day 10 — Smart pointers](day-10.md)
