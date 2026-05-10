# Day 6 — Iterators Properly

**Domain:** games • **Time:** 90 minutes • **Difficulty:** medium

## What you'll build

A `world_query` module: a collection of small pure functions that answer questions about the adventure world. Things like "what's the nearest alive enemy?", "summarize loot by item name", "can the player afford these items?". Every function is a single iterator chain — no `for` loops, no mutable accumulators. You'll also write your own iterator from scratch that walks the room graph in breadth-first order.

## What you'll learn

- **The `Iterator` trait** — the core of Rust's iteration story
- **Lazy evaluation** — adapters don't do work until consumed
- **Adapter methods**: `map`, `filter`, `filter_map`, `take`, `take_while`, `skip`, `enumerate`, `zip`, `chain`
- **Consumer methods**: `collect`, `sum`, `count`, `find`, `min_by`, `max_by`, `fold`, `any`, `all`
- The **three flavors of iteration**: `iter`, `iter_mut`, `into_iter`
- Writing a **custom iterator** by implementing `Iterator` yourself

## Background

### The Iterator trait

In Rust, iteration is built around one trait:

```rust
pub trait Iterator {
    type Item;
    fn next(&mut self) -> Option<Self::Item>;
}
```

An iterator knows how to produce the next item (or signal exhaustion with `None`). Every iterator — whether it's walking a `Vec`, a `HashMap`, a file's lines, a parsed JSON stream, or an infinite stream of random numbers — implements exactly this.

A `for` loop desugars to this:

```rust
for x in collection { body }

// is equivalent to:

let mut iter = IntoIterator::into_iter(collection);
while let Some(x) = iter.next() {
    body
}
```

That's it. Everything else in Rust's iterator machinery builds on this one method.

### Lazy evaluation

Iterator adapters (like `.map`, `.filter`) don't do anything immediately. They build up a *description* of work to do. No work happens until a consumer (`.collect`, `.sum`, `.for_each`) asks for values.

```rust
let v = vec![1, 2, 3, 4, 5];
let doubled = v.iter().map(|x| x * 2);   // nothing happens yet
let sum: i32 = doubled.sum();            // now the work happens
```

Consequence: you can chain dozens of adapters and the compiler fuses them into tight loops. No intermediate allocations.

### Three flavors

For a collection `coll`:

- `coll.iter()` — yields `&T` (shared borrow of each element).
- `coll.iter_mut()` — yields `&mut T` (exclusive borrow — only one at a time).
- `coll.into_iter()` — yields `T` (consumes the collection, moves elements out).

In Python there's just one kind. In Rust you pick based on what you need.

### `collect::<Vec<_>>()` is explicit

Python guesses: `[x*2 for x in nums]` gives a list. `{x*2 for x in nums}` a set. Rust makes you say:

```rust
let v: Vec<i32> = nums.iter().map(|x| x * 2).collect();
// or
let v = nums.iter().map(|x| x * 2).collect::<Vec<i32>>();
```

The turbofish (`::<Type>`) is one common way; the type annotation (`let v: Vec<i32>`) is another. You need one of them because `collect` is generic over the target type.

### The big adapter menu

You should bookmark the [Iterator trait docs](https://doc.rust-lang.org/std/iter/trait.Iterator.html) — the right column is a who's-who of methods. For today, the ones you'll reach for constantly:

- **Transform**: `map`, `filter`, `filter_map`, `flat_map`, `flatten`
- **Take/skip**: `take(n)`, `skip(n)`, `take_while(pred)`, `skip_while(pred)`
- **Annotate**: `enumerate`, `zip(other)`, `chain(other)`
- **Inspect**: `inspect(|x| ...)` (prints during iteration)
- **Terminate**: `collect`, `count`, `sum`, `product`, `min`, `max`, `min_by_key`, `max_by_key`
- **Search**: `find`, `position`, `any`, `all`
- **Aggregate**: `fold(init, |acc, x| ...)`, `reduce`

## Setting up

```bash
cargo new day-06
cd day-06
cargo add rand@0.8
```

We'll copy in the player/world types from previous days. Rather than repeat them here every tutorial, here's the minimal data we need:

```rust
use std::collections::HashMap;

pub struct World {
    pub rooms: HashMap<String, Room>,
}

pub struct Room {
    pub id: String,
    pub name: String,
    pub exits: HashMap<String, String>,     // dir -> dest
    pub items: Vec<Item>,
    pub enemies: Vec<Enemy>,
}

pub struct Player {
    pub pos: (i32, i32),
    pub hp: i32,
}

#[derive(Debug, Clone)]
pub struct Item {
    pub name: String,
    pub cost: u32,
}

#[derive(Debug, Clone)]
pub struct Enemy {
    pub name: String,
    pub hp: i32,
    pub xp_value: u32,
    pub pos: (i32, i32),
}

impl Enemy {
    pub fn is_dead(&self) -> bool { self.hp <= 0 }
}
```

Copy these into `main.rs` to start.

## Step 1 — Warm-up: basic iterator chains

Before we tackle real world-queries, play with some small chains in `main`:

```rust
fn main() {
    let nums = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];

    // Sum of squares of evens
    let sum: i32 = nums.iter()
        .filter(|&&n| n % 2 == 0)
        .map(|&n| n * n)
        .sum();
    println!("sum of squares of evens: {}", sum);

    // Pair up consecutive: (1,2), (3,4), ...
    let pairs: Vec<(i32, i32)> = nums.chunks(2)
        .filter_map(|chunk| match chunk {
            [a, b] => Some((*a, *b)),
            _ => None,
        })
        .collect();
    println!("pairs: {:?}", pairs);

    // Position of first value > 5
    let pos = nums.iter().position(|&n| n > 5);
    println!("first > 5 is at index {:?}", pos);
}
```

Run it:

```
sum of squares of evens: 220
pairs: [(1, 2), (3, 4), (5, 6), (7, 8), (9, 10)]
first > 5 is at index Some(5)
```

### Dereferencing in closures: `|&&n|` and `|&n|`

`nums.iter()` yields `&i32`. If your closure doesn't care about pattern-binding, you write `|x| ...` and `x: &i32`. To work with the int directly, you pattern-match: `|&n| ...` destructures the reference, so `n: i32`.

`.filter` passes `&Self::Item` to its predicate — so `.filter` on an iterator yielding `&i32` passes `&&i32`. Hence the double `&&` destructure.

It takes getting used to. If it's confusing, write out the explicit type: `|n: &&i32| **n % 2 == 0`.

## Step 2 — World queries, step one: nearest enemy

Let's build real functions. Create a module:

```rust
// in main.rs, below the data types

pub mod query {
    use super::*;

    pub fn nearest_alive_enemy<'a>(room: &'a Room, player: &Player) -> Option<&'a Enemy> {
        room.enemies
            .iter()
            .filter(|e| !e.is_dead())
            .min_by_key(|e| {
                let (dx, dy) = (e.pos.0 - player.pos.0, e.pos.1 - player.pos.1);
                dx.abs() + dy.abs()
            })
    }
}
```

### What's happening

- `room.enemies.iter()` gets an iterator yielding `&Enemy`.
- `.filter(|e| !e.is_dead())` removes the dead. `e` is already `&&Enemy` in the closure (filter double-references) — but `!e.is_dead()` auto-derefs through, so it just reads correctly.
- `.min_by_key(|e| ...)` returns `Option<&Enemy>` of the one with the smallest key. If no enemies remain (all dead, or empty to begin with), `None`.
- Manhattan distance: `|dx| + |dy|`.

### The `'a` lifetime

`fn nearest_alive_enemy<'a>(room: &'a Room, ...) -> Option<&'a Enemy>` says: "the returned reference borrows from `room`." You need this because otherwise the compiler can't tell whether the `&Enemy` came from `room` or `player` (it's from `room`). We'll have a day dedicated to lifetimes soon — for now, it's a formality that makes the function signature precise.

## Step 3 — Loot summary with fold

Given a vec of items, count how many of each name.

```rust
pub mod query {
    // ... previous function ...

    pub fn loot_summary(items: &[Item]) -> HashMap<String, u32> {
        items.iter().fold(HashMap::new(), |mut acc, item| {
            *acc.entry(item.name.clone()).or_insert(0) += 1;
            acc
        })
    }
}
```

### `fold(init, |acc, x| ...)`

Most flexible of the consumers. You provide:
- An initial accumulator value.
- A closure `(acc, next_item) -> new_acc`.

Returns the final accumulator. Every other consumer (`sum`, `count`, `collect`) can be written in terms of `fold`.

### `HashMap::entry`

Python's `dict.setdefault` pattern in Rust:

- `map.entry(key)` returns an `Entry` — a view into the spot for that key.
- `.or_insert(0)` returns `&mut V`, inserting `0` first if the key was absent.
- `*ref += 1` dereferences the mutable reference to increment.

You'll see this idiom a lot. It's the cleanest way to "accumulate into a hashmap."

### Try it

```rust
fn main() {
    let items = vec![
        Item { name: "gold".to_string(), cost: 1 },
        Item { name: "gold".to_string(), cost: 1 },
        Item { name: "sword".to_string(), cost: 100 },
        Item { name: "gold".to_string(), cost: 1 },
    ];
    println!("{:?}", query::loot_summary(&items));
}
```

```
{"gold": 3, "sword": 1}
```

Map iteration order is unspecified — yours may vary.

## Step 4 — Sorted descriptions with collect

A function that returns a formatted description of a room's items sorted alphabetically:

```rust
pub mod query {
    // ...

    pub fn describe_items(room: &Room) -> String {
        let mut names: Vec<&str> = room.items.iter().map(|i| i.name.as_str()).collect();
        names.sort();
        if names.is_empty() {
            "No items.".to_string()
        } else {
            format!("Items: {}", names.join(", "))
        }
    }
}
```

### Why two steps (collect then sort)?

Iterators themselves don't sort — sorting needs random access, which iterators don't provide. `.collect()` into a `Vec`, sort, continue.

### `Vec::sort` vs `Vec::sort_by_key`

- `.sort()` — for any `Ord` type (default ordering).
- `.sort_by_key(|x| key)` — sort by a computed key.
- `.sort_by(|a, b| cmp)` — full custom comparator.

## Step 5 — Enemies sorted by HP

Similar pattern:

```rust
pub mod query {
    // ...

    pub fn describe_enemies(room: &Room) -> String {
        let mut alive: Vec<&Enemy> = room.enemies.iter().filter(|e| !e.is_dead()).collect();
        // Sort by HP descending; ties broken by name
        alive.sort_by(|a, b| {
            b.hp.cmp(&a.hp).then_with(|| a.name.cmp(&b.name))
        });
        if alive.is_empty() {
            "No enemies present.".to_string()
        } else {
            let lines: Vec<String> = alive.iter()
                .map(|e| format!("{} (HP {})", e.name, e.hp))
                .collect();
            format!("Enemies: {}", lines.join(", "))
        }
    }
}
```

### `Ord::cmp` and `.then_with`

`a.hp.cmp(&b.hp)` returns `Ordering::{Less, Equal, Greater}`. `.then_with(|| next_ordering)` chains: if the first ordering is `Equal`, use the fallback. This is the idiomatic way to do multi-level sorting in Rust.

Note the closure for `then_with` — it's lazy so you only compute the fallback when needed.

## Step 6 — Aggregates and predicates

### Total XP from dead enemies

```rust
pub mod query {
    // ...

    pub fn total_xp_reward(enemies: &[Enemy]) -> u32 {
        enemies.iter()
            .filter(|e| e.is_dead())
            .map(|e| e.xp_value)
            .sum()
    }
}
```

### Does the inventory have everything required?

```rust
pub mod query {
    // ...

    pub fn inventory_has_all(inv: &[Item], required: &[&str]) -> bool {
        required.iter().all(|req| inv.iter().any(|i| i.name == *req))
    }
}
```

`.all(pred)` returns `true` iff every element matches. `.any(pred)` returns `true` iff at least one does. Short-circuits — stops as soon as it knows the answer.

### First unlocked exit

For this we'd need an `exits` structure richer than our `HashMap<String, String>` — with a `locked: bool`. Let's pretend:

```rust
pub struct Exit {
    pub direction: String,
    pub destination: String,
    pub locked: bool,
}

pub fn first_unlocked_exit(exits: &[Exit]) -> Option<&str> {
    exits.iter()
        .find(|e| !e.locked)
        .map(|e| e.direction.as_str())
}
```

### `find` and `map` on `Option`

`find(pred)` returns `Option<&T>`. `.map(|x| ...)` on `Option` transforms the inner value if present, leaves `None` alone. Python's equivalent would be `None if x is None else f(x)` but more concise.

## Step 7 — A custom iterator

The ultimate iterator exercise: implement `Iterator` by hand. Let's build a BFS walker for the world's room graph.

```rust
use std::collections::{HashSet, VecDeque};

pub struct BfsWalker<'a> {
    world: &'a World,
    queue: VecDeque<&'a str>,
    seen: HashSet<&'a str>,
}

impl<'a> BfsWalker<'a> {
    pub fn new(world: &'a World, start: &'a str) -> BfsWalker<'a> {
        let mut queue = VecDeque::new();
        let mut seen = HashSet::new();
        queue.push_back(start);
        seen.insert(start);
        BfsWalker { world, queue, seen }
    }
}

impl<'a> Iterator for BfsWalker<'a> {
    type Item = &'a Room;

    fn next(&mut self) -> Option<&'a Room> {
        let id = self.queue.pop_front()?;
        let room = self.world.rooms.get(id)?;
        for dest in room.exits.values() {
            if self.seen.insert(dest.as_str()) {   // `insert` returns true if newly added
                self.queue.push_back(dest.as_str());
            }
        }
        Some(room)
    }
}
```

### What's going on

- The walker owns three pieces: a borrowed world, a queue of room IDs still to visit, and a set of already-seen IDs.
- `new` enqueues the start room.
- `next` dequeues the front, enqueues its unseen neighbors, returns the room.
- `seen.insert(x)` returns `true` if the element was newly inserted (not already present).
- When the queue is empty, `pop_front()?` returns `None`, which the `?` uses to return `None` from the whole `next` — the iterator is done.

### Use it

```rust
fn main() {
    let world = build_tiny_world();
    for room in BfsWalker::new(&world, "library").take(10) {
        println!("visited: {}", room.name);
    }
}
```

Because `BfsWalker` is now a real `Iterator`, you can `.take(10).collect()`, `.filter(...)`, `.map(...)` — everything in the iterator toolbox works on it.

This is one of the things that makes Rust's iterator design beautiful: you write one `next` method and get the entire adapter library for free.

## Step 8 — Put the queries to use in main

```rust
fn main() {
    let library = Room {
        id: "library".into(),
        name: "Dusty Library".into(),
        exits: HashMap::new(),
        items: vec![
            Item { name: "scroll".into(), cost: 5 },
            Item { name: "ancient tome".into(), cost: 200 },
        ],
        enemies: vec![
            Enemy { name: "goblin".into(), hp: 15, xp_value: 10, pos: (3, 2) },
            Enemy { name: "rat".into(), hp: 0, xp_value: 1, pos: (1, 1) },    // dead
            Enemy { name: "troll".into(), hp: 40, xp_value: 50, pos: (8, 7) },
        ],
    };

    let player = Player { pos: (0, 0), hp: 50 };

    println!("{}", query::describe_items(&library));
    println!("{}", query::describe_enemies(&library));
    println!("Total XP gained: {}", query::total_xp_reward(&library.enemies));

    if let Some(e) = query::nearest_alive_enemy(&library, &player) {
        println!("Nearest alive enemy: {}", e.name);
    }

    let summary = query::loot_summary(&library.items);
    println!("Loot summary: {:?}", summary);
}
```

Output:

```
Items: ancient tome, scroll
Enemies: troll (HP 40), goblin (HP 15)
Total XP gained: 1
Nearest alive enemy: goblin
Loot summary: {"scroll": 1, "ancient tome": 1}
```

Notice that we said "Total XP gained: 1" — only the dead rat contributed. The filter works as intended.

## Common pitfalls

### "Cannot move out of captured variable"

Your closure tries to consume something borrowed:

```rust
let s = String::from("hi");
let closures: Vec<Box<dyn Fn() -> String>> = vec![
    Box::new(|| s),   // ERROR: cannot move `s` into closure
];
```

Solutions:
- `s.clone()` inside the closure.
- `move` before the closure to take ownership: `Box::new(move || s)`.
- Return `&str` instead of `String`.

### Consuming the iterator twice

Iterators are one-shot. Once consumed, they're done.

```rust
let iter = v.iter().map(|x| x * 2);
let total: i32 = iter.sum();           // consumes
let double: Vec<i32> = iter.collect(); // ERROR: iter is already moved
```

Solutions:
- Materialize to a `Vec` first if you need it twice.
- Call the adapter chain twice from the source.
- `iter().map(...)` multiple times — cheap, since iterators are lazy.

### `.collect::<Vec<_>>()` vs `.collect::<Vec<T>>()`

Both work. `Vec<_>` is "a Vec, I don't care of what." Rust uses context to fill in the `_`. Clearer for short chains; use explicit types when compilation gets confused.

### Iterator uses `&&T` types

As we saw, `.iter()` yields `&T`, and `.filter`/`.find` closures get `&Self::Item`, which is `&&T`. Patterns like `|&&x|` or `|x| **x > 0` or `|x: &&i32| ...` all work. Pick one style and stay consistent.

### Always writing `let v: Vec<_> = ... .collect()`

Sometimes you don't need a vec at all. If you're just iterating downstream, keep the iterator lazy:

```rust
// Unnecessary collect + loop
let doubled: Vec<i32> = nums.iter().map(|x| x * 2).collect();
for d in doubled { println!("{}", d); }

// Better: lazy
for d in nums.iter().map(|x| x * 2) {
    println!("{}", d);
}
```

## What you learned

- The `Iterator` trait: one method (`next`), a universe of adapters.
- **Lazy evaluation**: adapters build pipelines; consumers run them.
- **Transform** (`map`, `filter`, `filter_map`), **take/skip**, **aggregate** (`fold`, `sum`, `count`), **search** (`find`, `any`, `all`), **annotate** (`enumerate`, `zip`).
- **`iter()`, `iter_mut()`, `into_iter()`** — borrow, mutate, consume.
- **`HashMap::entry().or_insert()`** for accumulating into maps.
- **Multi-level sorting** with `cmp` + `.then_with`.
- Writing your own iterator by `impl Iterator for MyType`.

## Exercises

1. **No-loop discipline.** Go back to Day 1 / Day 3 code and find any `for` or `while` that can be replaced with iterator chains. Do it.
2. **Stream processing.** Implement `fn moving_average(nums: &[f64], window: usize) -> Vec<f64>` using only iterators (hint: `windows`, `map`, `collect`).
3. **`DfsWalker`.** Same as `BfsWalker` but depth-first. Use `Vec<&str>` as a stack instead of `VecDeque`.
4. **`PairingIter`.** Implement a custom iterator that, given two iterators, yields pairs `(a, b)` where `a < b`, in sorted merged order — basically a merge-sort step. Good exercise in holding state.
5. **`cargo clippy`.** Run it on your world-query module. Clippy will suggest further iterator simplifications — try them and learn.

## What's next

Day 7 wraps up Week 1 with **proper error handling**: the `thiserror` and `anyhow` crates, error composition via `From` impls, and adding context to failures. You'll build a full command parser with rich, helpful error messages — and finally have zero `.unwrap()` in your code.

→ [Day 7 — Rich errors](day-07.md)
