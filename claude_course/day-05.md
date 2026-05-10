# Day 5 — Typed Inventory

**Domain:** games • **Time:** 60–90 minutes • **Difficulty:** medium

## What you'll build

A generic `Inventory<T: Item>` type. The player has three bags — one for weapons, one for potions, one for scrolls. Each bag has a weight capacity and only holds its own item type. Instead of writing three near-identical structs, you'll write one generic struct and instantiate it three times. This is the bread-and-butter use of generics in Rust.

## What you'll learn

- **Generic structs** with trait-bounded type parameters
- **`where` clauses** for complex bounds
- **Monomorphization** in depth — what the compiler actually produces
- **Multiple type parameters** (a stretch exercise)
- Returning `Option<&T>` and `impl Iterator<Item = &T>`
- When to use generics vs dynamic dispatch

## Background

### Generics in Python vs Rust

Python's type hints support `List[T]`, `Dict[K, V]`, etc., but at runtime everything is duck-typed. `def add(a, b)` doesn't care what `a` and `b` are until it tries `+`.

Rust generics are checked at compile time. Bounds tell the compiler *what you'll use* on the generic type:

```rust
fn largest<T: PartialOrd>(list: &[T]) -> &T {
    let mut largest = &list[0];
    for item in list {
        if item > largest {    // requires PartialOrd
            largest = item;
        }
    }
    largest
}
```

Without `T: PartialOrd`, the compiler rejects `item > largest`. With the bound, any type supporting comparison works: `i32`, `f64`, `&str`, `String`, your own struct that implements `PartialOrd`.

### Monomorphization

When you call `largest(&vec_of_ints)` and `largest(&vec_of_strings)`, the compiler generates two separate copies:

```rust
fn largest_i32(list: &[i32]) -> &i32 { ... }
fn largest_String(list: &[String]) -> &String { ... }
```

You never see the generated code, but it's there. Consequences:

- **Zero runtime cost.** Each version is as fast as a handwritten one.
- **Larger binary.** Each instantiation adds code. Usually invisible; sometimes matters.
- **Slower compile times** with heavy generic usage.

Trade: generics give you runtime speed and type safety at the cost of compile time. For 99% of code, that's a great deal.

### Generic structs

A struct can be generic over types:

```rust
struct Pair<T> {
    first: T,
    second: T,
}

impl<T> Pair<T> {
    fn new(first: T, second: T) -> Pair<T> {
        Pair { first, second }
    }
}

impl<T: std::fmt::Display> Pair<T> {
    fn print(&self) {
        println!("({}, {})", self.first, self.second);
    }
}
```

Note the two `impl` blocks. The first applies to *any* `T`. The second applies only when `T: Display`. `Pair<NonDisplayType>` can still be created and used with `new`, but `print` won't exist on it.

### Multiple bounds and `where`

```rust
// Inline (cramped)
fn foo<T: Clone + std::fmt::Debug>(x: T) { ... }

// With where (cleaner when bounds grow)
fn foo<T>(x: T)
where
    T: Clone + std::fmt::Debug,
{
    ...
}
```

Both are equivalent. Use `where` when readability suffers.

## Setting up

```bash
cargo new day-05
cd day-05
```

No external dependencies today.

## Step 1 — The Item trait

Every bag holds items. Define the interface:

```rust
pub trait Item {
    fn name(&self) -> &str;
    fn weight(&self) -> u32;
    fn description(&self) -> &str;
}
```

Simple. Every item has a name, a weight, and a description.

## Step 2 — Three concrete item types

### Weapon

```rust
#[derive(Debug)]
pub struct Weapon {
    name: String,
    weight: u32,
    damage: u32,
}

impl Weapon {
    pub fn sword() -> Weapon {
        Weapon { name: "iron sword".to_string(), weight: 8, damage: 10 }
    }
    pub fn axe() -> Weapon {
        Weapon { name: "heavy axe".to_string(), weight: 15, damage: 14 }
    }
    pub fn dagger() -> Weapon {
        Weapon { name: "dagger".to_string(), weight: 2, damage: 4 }
    }
}

impl Item for Weapon {
    fn name(&self) -> &str { &self.name }
    fn weight(&self) -> u32 { self.weight }
    fn description(&self) -> &str { "a weapon" }
}
```

### Potion

```rust
#[derive(Debug)]
pub struct Potion {
    name: String,
    weight: u32,
    heal_amount: u32,
}

impl Potion {
    pub fn small_healing() -> Potion {
        Potion { name: "small potion".to_string(), weight: 1, heal_amount: 10 }
    }
    pub fn large_healing() -> Potion {
        Potion { name: "large potion".to_string(), weight: 2, heal_amount: 30 }
    }
}

impl Item for Potion {
    fn name(&self) -> &str { &self.name }
    fn weight(&self) -> u32 { self.weight }
    fn description(&self) -> &str { "a healing potion" }
}
```

### Scroll

```rust
#[derive(Debug)]
pub struct Scroll {
    name: String,
    weight: u32,
    spell: String,
}

impl Scroll {
    pub fn fireball() -> Scroll {
        Scroll { name: "fireball scroll".to_string(), weight: 1, spell: "fireball".to_string() }
    }
    pub fn ice_bolt() -> Scroll {
        Scroll { name: "ice bolt scroll".to_string(), weight: 1, spell: "ice bolt".to_string() }
    }
}

impl Item for Scroll {
    fn name(&self) -> &str { &self.name }
    fn weight(&self) -> u32 { self.weight }
    fn description(&self) -> &str { "a magical scroll" }
}
```

## Step 3 — The generic Inventory

Now the fun part. One struct, parameterized over the item type:

```rust
pub struct Inventory<T: Item> {
    name: String,
    slots: Vec<T>,
    max_weight: u32,
}
```

The `<T: Item>` says: "`T` is any type implementing `Item`." The struct can only be instantiated with such a type. `Inventory<i32>` won't compile because `i32: Item` isn't true.

### Why not `Vec<Box<dyn Item>>`?

That would give you a bag holding *any* mix of items — goblin-drops-a-sword-and-a-potion style. Legitimate design, and it's what you'd pick for a universal inventory.

The *typed* version (`Inventory<Weapon>` separate from `Inventory<Potion>`) has stricter guarantees:

- You can't accidentally put a potion in your weapon bag.
- `weapons.iter()` returns `impl Iterator<Item = &Weapon>` — full access to Weapon-specific fields (`damage`), not just the trait methods.
- Zero runtime dispatch cost.

Today we're practicing generics, so we pick the typed version.

## Step 4 — Methods on Inventory

Start with the basics:

```rust
impl<T: Item> Inventory<T> {
    pub fn new(name: &str, max_weight: u32) -> Inventory<T> {
        Inventory {
            name: name.to_string(),
            slots: Vec::new(),
            max_weight,
        }
    }

    pub fn total_weight(&self) -> u32 {
        self.slots.iter().map(|i| i.weight()).sum()
    }

    pub fn len(&self) -> usize {
        self.slots.len()
    }

    pub fn is_empty(&self) -> bool {
        self.slots.is_empty()
    }
}
```

`total_weight` uses iterator chaining — we get a proper tour of iterators tomorrow, but the idea is: iterate over items, map each to its weight, sum.

### Why `impl<T: Item> Inventory<T>`?

You have to repeat the generic parameter and its bound on the `impl`. Two alternatives:

```rust
// Full bound repeated
impl<T: Item> Inventory<T> { ... }

// Using where
impl<T> Inventory<T>
where
    T: Item,
{ ... }
```

Equivalent. Pick whichever reads better.

### Adding items — with error handling

```rust
#[derive(Debug)]
pub enum InventoryError {
    TooHeavy { current: u32, max: u32, tried: u32 },
    DuplicateName(String),
}

impl std::fmt::Display for InventoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InventoryError::TooHeavy { current, max, tried } => {
                write!(
                    f,
                    "too heavy: carrying {}kg, adding {}kg would exceed {}kg limit",
                    current, tried, max
                )
            }
            InventoryError::DuplicateName(name) => {
                write!(f, "item named {:?} is already in the bag", name)
            }
        }
    }
}

impl std::error::Error for InventoryError {}
```

Now the method:

```rust
impl<T: Item> Inventory<T> {
    pub fn add(&mut self, item: T) -> Result<(), InventoryError> {
        let new_weight = item.weight();
        let current = self.total_weight();

        if current + new_weight > self.max_weight {
            return Err(InventoryError::TooHeavy {
                current,
                max: self.max_weight,
                tried: new_weight,
            });
        }

        if self.slots.iter().any(|existing| existing.name() == item.name()) {
            return Err(InventoryError::DuplicateName(item.name().to_string()));
        }

        self.slots.push(item);
        Ok(())
    }
}
```

`self.slots.iter().any(|e| ...)` — iterator's `any` returns `true` if at least one element matches the predicate. Python's `any(...)` is the same idea.

### Finding and removing

```rust
impl<T: Item> Inventory<T> {
    pub fn find(&self, name: &str) -> Option<&T> {
        self.slots.iter().find(|item| item.name() == name)
    }

    pub fn remove(&mut self, name: &str) -> Option<T> {
        let pos = self.slots.iter().position(|item| item.name() == name)?;
        Some(self.slots.remove(pos))
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> + '_ {
        self.slots.iter()
    }
}
```

A few new things:

- `find(predicate)` returns `Option<&T>` — the first matching element borrowed.
- `position(predicate)` returns `Option<usize>` — the index of the first match. The `?` here unwraps `Some(i)` or returns `None` from the whole `remove` function.
- `pub fn iter(&self) -> impl Iterator<Item = &T> + '_` — returns "some iterator over references to T, living as long as `&self`." `'_` is "infer the lifetime" (Day 9).

## Step 5 — A generic free function

Because we've got a trait bound, we can also write a standalone function that takes any inventory:

```rust
pub fn show<T: Item>(inv: &Inventory<T>) {
    println!("{} ({}/{}):", inv.name, inv.total_weight(), inv.max_weight);
    for item in inv.iter() {
        println!("  - {} ({}, {} kg)", item.name(), item.description(), item.weight());
    }
    if inv.is_empty() {
        println!("  (empty)");
    }
}
```

Wait — we accessed `inv.name` but `name` was a private field (no `pub`). Fix that by exposing a getter, or just make the field public for this module. Let's keep `name` private and add:

```rust
impl<T: Item> Inventory<T> {
    pub fn display_name(&self) -> &str {
        &self.name
    }
}
```

Update `show`:

```rust
pub fn show<T: Item>(inv: &Inventory<T>) {
    println!("{} ({}/{}):", inv.display_name(), inv.total_weight(), inv.max_weight);
    ...
}
```

Wait again — `inv.max_weight` is also a private field. Use `max_weight` the same way. Let me just put a getter once and move on:

```rust
impl<T: Item> Inventory<T> {
    pub fn display_name(&self) -> &str { &self.name }
    pub fn max_weight(&self) -> u32 { self.max_weight }
}
```

And the function:

```rust
pub fn show<T: Item>(inv: &Inventory<T>) {
    println!(
        "{} ({}/{} kg):",
        inv.display_name(),
        inv.total_weight(),
        inv.max_weight()
    );
    for item in inv.iter() {
        println!("  - {} ({}, {} kg)", item.name(), item.description(), item.weight());
    }
    if inv.is_empty() {
        println!("  (empty)");
    }
}
```

### Field access from outside the module

Rust has module privacy. Fields are private by default; you can only read/write them from code in the same module. To expose to outside callers, you either make the field `pub` or provide an accessor method (preferred — you keep invariants).

## Step 6 — Use it

```rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut weapons = Inventory::<Weapon>::new("Weapons", 50);
    let mut potions = Inventory::<Potion>::new("Potions", 10);
    let mut scrolls = Inventory::<Scroll>::new("Scrolls", 5);

    weapons.add(Weapon::sword())?;
    weapons.add(Weapon::dagger())?;
    weapons.add(Weapon::axe())?;

    potions.add(Potion::small_healing())?;
    potions.add(Potion::large_healing())?;

    scrolls.add(Scroll::fireball())?;

    show(&weapons);
    show(&potions);
    show(&scrolls);

    // Try to overload weapons
    println!();
    match weapons.add(Weapon::axe()) {
        Ok(()) => println!("added another axe"),
        Err(e) => println!("expected failure: {}", e),
    }

    Ok(())
}
```

Run it:

```
Weapons (25/50 kg):
  - iron sword (a weapon, 8 kg)
  - dagger (a weapon, 2 kg)
  - heavy axe (a weapon, 15 kg)
Potions (3/10 kg):
  - small potion (a healing potion, 1 kg)
  - large potion (a healing potion, 2 kg)
Scrolls (1/5 kg):
  - fireball scroll (a magical scroll, 1 kg)

expected failure: item named "heavy axe" is already in the bag
```

One function `show`, three different types of inventory, all type-safe.

### Main's return type

`fn main() -> Result<(), Box<dyn std::error::Error>>` lets `main` use `?` to propagate errors. `Box<dyn Error>` is "any error type" — acceptable for `main`; we'll use proper error types in libraries (Day 7).

### Turbofish: `Inventory::<Weapon>::new(...)`

The `::<Weapon>` is the **turbofish** syntax. You write it when the compiler can't infer the type:

```rust
let weapons = Inventory::<Weapon>::new("Weapons", 50);    // turbofish
let mut weapons: Inventory<Weapon> = Inventory::new("Weapons", 50);    // type ascription
```

Both work. Turbofish is compact; ascription is often more readable.

### What the compiler generated

Three separate specialized `Inventory` types are emitted in the binary:

```text
Inventory::<Weapon>           with Vec<Weapon>, add(Weapon), ...
Inventory::<Potion>           with Vec<Potion>, add(Potion), ...
Inventory::<Scroll>           with Vec<Scroll>, add(Scroll), ...
```

Same source, three specialized copies. `iter()` on weapons yields `&Weapon`; on potions yields `&Potion`. Types don't mix.

## Step 7 — Specialised impls

You can write methods that only exist for specific concrete instantiations. Say weapon inventories have a `most_damaging` method:

```rust
impl Inventory<Weapon> {
    pub fn most_damaging(&self) -> Option<&Weapon> {
        self.slots.iter().max_by_key(|w| w.damage)
    }
}
```

Note: no `<T>`. This impl is specialized to `Inventory<Weapon>`.

But wait — `.damage` is a private field on `Weapon`. Since `Weapon` and this `impl` live in the same module, access works. If you split them, you'd need an accessor.

Use it:

```rust
if let Some(best) = weapons.most_damaging() {
    println!("Best weapon: {}", best.name());
}
```

`Inventory<Potion>` has no `most_damaging` method, and calling it would be a compile error. Specialization by concrete type is a powerful tool.

## Common pitfalls

### "The size for values of type `T` cannot be known at compilation time"

You wrote `T` instead of `&T` somewhere the compiler needs a known size. If `T: Sized` is implied (it is, by default), you should be fine. If you introduced `T: ?Sized` (opting out of the size requirement), you can't store `T` by value in a struct — use `&T` or `Box<T>`.

### Bound forgetting

You tried to use `self.slots.iter().map(|i| i.weight()).sum()` — sum expects the element type to implement `Sum` (or convert via `Sum::<Item>`). The error says:

```
error[E0277]: the trait bound `u32: Sum<...>` is not satisfied
```

The fix: `let ws: u32 = ...; .sum()` gives the compiler enough info. Or `.sum::<u32>()`. Generic type inference doesn't always find the right concrete type.

### "Method not found" on `Inventory<T>`

You wrote a method like `most_damaging` on `impl Inventory<Weapon>` but called it on `Inventory<Potion>`. The error is crystal clear:

```
error[E0599]: no method named `most_damaging` found for struct `Inventory<Potion>`
```

Which is the point — specialized methods exist only for their specialized types.

### Exposing more than you want

You made `slots: pub Vec<T>` for convenience. Now callers can do `inventory.slots.push(item)` and bypass your weight check. Hide internal state; expose accessors. `show`'s implementation should need `display_name`, `total_weight`, `max_weight`, `iter`, and `is_empty` — nothing more.

## What you learned

- **Generic structs**: `Inventory<T: Item>` — one definition, many concrete types.
- **Trait bounds** on type parameters constrain what the generic type must support.
- **Monomorphization**: separate specialized code per concrete type. Zero runtime cost.
- **`impl<T: Trait> Struct<T>` vs `impl Struct<ConcreteType>`** — general vs specialized methods.
- **`where` clauses** for readable multi-bound generics.
- **Turbofish** `::<T>` to disambiguate types when inference can't.
- **Returning `impl Iterator<Item = ...>`** from methods.

## Exercises

1. **Swap to iterator sum properly.** Replace `.map(...).sum()` style with something that works even if `T` is a custom numeric type. (Hint: the stdlib `Sum` trait.)
2. **Multiple generics.** Add a `LootTable<T: Item, R: rand::Rng>`. `roll(&mut self) -> &T` picks a random item weighted by `weight()`. Show how the same code works whether `R` is `ThreadRng` or `StdRng`.
3. **Bag-of-anything.** Write `type MixedBag = Vec<Box<dyn Item>>`. Write a function `show_mixed(&MixedBag)` that prints any combination. Compare: the typed `Inventory<T>` vs the dynamic `MixedBag`. When would you pick each?
4. **Sort and filter.** Add `heaviest(&self) -> Option<&T>` to any `Inventory<T>`. Implement with `iter().max_by_key(...)`.

## What's next

Day 6 dives deeply into **iterators** — one of Rust's best-loved features. You've already used `.iter().map(...).sum()`; tomorrow you'll build real iterator chains, understand lazy evaluation, and even write your own custom iterator.

→ [Day 6 — Iterators properly](day-06.md)
