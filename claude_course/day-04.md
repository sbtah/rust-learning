# Day 4 — Entity Trait System

**Domain:** games • **Time:** 90 minutes • **Difficulty:** medium

## What you'll build

Refactor combat so the player and every enemy type share a single `Entity` interface. You'll write code that works on anything "entity-shaped" without caring whether it's a `Goblin`, a `Troll`, or the `Player`. You'll also learn the tradeoff between Rust's two ways of using traits: **static dispatch** (fast, compiled-away) and **dynamic dispatch** (heterogeneous collections, small runtime cost).

## What you'll learn

- **Traits** as interfaces for shared behaviour
- **Default method implementations**
- **Generics with trait bounds** — `fn attack<E: Entity>(...)` (static dispatch)
- **Trait objects** — `Box<dyn Entity>` (dynamic dispatch)
- **Supertraits** — `trait Lootable: Entity`
- When to pick generics vs trait objects

## Background

### Traits are Rust's interfaces

A trait declares a set of methods. Any type can *implement* the trait by providing those methods. Other code can then be generic over "any type implementing this trait."

```rust
trait Greet {
    fn greeting(&self) -> String;
}

struct English;
struct French;

impl Greet for English {
    fn greeting(&self) -> String {
        String::from("Hello!")
    }
}

impl Greet for French {
    fn greeting(&self) -> String {
        String::from("Bonjour!")
    }
}
```

Now you can write code that works on anything implementing `Greet`.

### Python comparison

- Python ABCs / Protocols are traits' nearest cousin, but they're checked at runtime (or statically with type checkers).
- Rust traits are checked at compile time. If a type doesn't implement a required trait, the code won't compile. No `AttributeError` surprises.
- Rust traits can have **default methods** — shared implementations based on the required ones.

### Static dispatch: generics

```rust
fn shout<G: Greet>(thing: &G) {
    println!("{}!", thing.greeting().to_uppercase());
}

shout(&English);   // prints HELLO!!
shout(&French);    // prints BONJOUR!!
```

What the compiler does: for each concrete type you call `shout` with, it generates a separate specialized copy of `shout`. `shout::<English>` and `shout::<French>` become distinct machine code. This process is called **monomorphization**. The cost is binary size; the payoff is zero-cost abstraction — the generated code is as fast as a hand-written version.

### Dynamic dispatch: trait objects

What if you want a *collection* of different types, all implementing the same trait?

```rust
let speakers: Vec<Box<dyn Greet>> = vec![
    Box::new(English),
    Box::new(French),
];
for s in &speakers {
    println!("{}", s.greeting());
}
```

`Box<dyn Greet>` is a **trait object** — a fat pointer: one pointer to the data, one to a *vtable* (table of function pointers for the trait's methods). When you call `s.greeting()`, the runtime looks up the method via the vtable. One indirection per call.

This is how you mix types in one collection. It costs a tiny bit more than static dispatch but enables flexibility you can't get otherwise.

### The rules:

- **Static dispatch (generics)**: one known type per call site. Fast. Use for most things.
- **Dynamic dispatch (`dyn Trait`)**: truly heterogeneous collections or plugin-style systems. Use when you need runtime polymorphism.

### Default methods

Traits can provide default implementations that rely on required methods:

```rust
trait Greet {
    fn greeting(&self) -> String;
    fn shout(&self) -> String {
        self.greeting().to_uppercase() + "!"
    }
}
```

Now every implementor gets `shout` for free, but can override it.

## Setting up

```bash
cargo new day-04
cd day-04
cargo add rand@0.8
```

Copy Day 2's combat code as a starting point. We're going to rebuild the combat section around traits.

## Step 1 — Define the Entity trait

```rust
trait Entity {
    fn name(&self) -> &str;
    fn hp(&self) -> i32;
    fn max_hp(&self) -> i32;
    fn take_damage(&mut self, amount: i32);

    // Default method — implementors get this for free
    fn is_dead(&self) -> bool {
        self.hp() <= 0
    }

    // Another default: a formatted status line
    fn status_line(&self) -> String {
        format!("{}: {}/{} HP", self.name(), self.hp(), self.max_hp())
    }
}
```

`name` returns `&str`, not `String`. This avoids allocation on every call. The implementer decides whether to return a string literal (`&'static str`) or a borrow from a field (`&String` which auto-derefs to `&str`).

## Step 2 — Implement Entity for several types

Let's build four entity types. Start with the player:

```rust
struct Player {
    name: String,
    hp: i32,
    max_hp: i32,
}

impl Player {
    fn new(name: &str) -> Player {
        Player { name: name.to_string(), hp: 50, max_hp: 50 }
    }
}

impl Entity for Player {
    fn name(&self) -> &str { &self.name }
    fn hp(&self) -> i32 { self.hp }
    fn max_hp(&self) -> i32 { self.max_hp }
    fn take_damage(&mut self, amount: i32) {
        self.hp -= amount;
    }
}
```

Notice: `&self.name` has type `&String`, which coerces to `&str` automatically (a "deref coercion") because `String` implements `Deref<Target = str>`. No manual conversion needed.

Now a goblin:

```rust
struct Goblin {
    hp: i32,
}

impl Goblin {
    fn new() -> Goblin {
        Goblin { hp: 20 }
    }
}

impl Entity for Goblin {
    fn name(&self) -> &str { "goblin" }
    fn hp(&self) -> i32 { self.hp }
    fn max_hp(&self) -> i32 { 20 }
    fn take_damage(&mut self, amount: i32) {
        self.hp -= amount;
    }
}
```

A troll:

```rust
struct Troll {
    hp: i32,
}

impl Troll {
    fn new() -> Troll {
        Troll { hp: 45 }
    }
}

impl Entity for Troll {
    fn name(&self) -> &str { "troll" }
    fn hp(&self) -> i32 { self.hp }
    fn max_hp(&self) -> i32 { 45 }
    fn take_damage(&mut self, amount: i32) {
        self.hp -= amount;
    }
}
```

And a wraith — which *overrides* `take_damage` because wraiths are ethereal:

```rust
struct Wraith {
    hp: i32,
}

impl Wraith {
    fn new() -> Wraith {
        Wraith { hp: 30 }
    }
}

impl Entity for Wraith {
    fn name(&self) -> &str { "wraith" }
    fn hp(&self) -> i32 { self.hp }
    fn max_hp(&self) -> i32 { 30 }
    fn take_damage(&mut self, amount: i32) {
        // Wraiths take half damage
        self.hp -= amount / 2;
    }
}
```

Every implementor gets `is_dead()` and `status_line()` for free from the defaults.

## Step 3 — Static dispatch: a generic function

Here's the first payoff. A function that prints any entity's status:

```rust
fn describe<E: Entity>(e: &E) {
    println!("{}", e.status_line());
    if e.is_dead() {
        println!("  (dead)");
    }
}
```

In `main`:

```rust
fn main() {
    let player = Player::new("Alice");
    let goblin = Goblin::new();
    let wraith = Wraith::new();

    describe(&player);
    describe(&goblin);
    describe(&wraith);
}
```

Output:

```
Alice: 50/50 HP
goblin: 20/20 HP
wraith: 30/30 HP
```

### What the compiler did

Three specialized functions were generated: `describe::<Player>`, `describe::<Goblin>`, `describe::<Wraith>`. Each has zero overhead compared to a handwritten non-generic version. This is Rust's "zero-cost abstractions" promise in action.

### `fn describe<E: Entity>` vs `fn describe(e: &impl Entity)`

These two forms are almost equivalent:

```rust
fn describe<E: Entity>(e: &E) { ... }
fn describe(e: &impl Entity) { ... }
```

`impl Trait` in argument position is shorthand for a single generic parameter. Prefer `impl Trait` for simple cases (one generic, no turbofish needed). Prefer `<T: Trait>` when the same type appears in multiple places:

```rust
// Need the explicit form because both args must be the same type
fn pair<E: Entity>(a: &E, b: &E) { ... }

// `impl Trait` in both positions would allow *different* entity types
fn pair(a: &impl Entity, b: &impl Entity) { ... }
```

## Step 4 — Dynamic dispatch: a mixed collection

Generics require you to know the concrete type at compile time. But what if you want a list of mixed entities?

```rust
fn combat_round(combatants: &mut [Box<dyn Entity>]) {
    for e in combatants.iter_mut() {
        println!("{}", e.status_line());
    }
}
```

In `main`:

```rust
fn main() {
    let mut combatants: Vec<Box<dyn Entity>> = vec![
        Box::new(Player::new("Alice")),
        Box::new(Goblin::new()),
        Box::new(Troll::new()),
        Box::new(Wraith::new()),
    ];

    combat_round(&mut combatants);

    // Damage everyone
    for e in combatants.iter_mut() {
        e.take_damage(10);
    }

    combat_round(&mut combatants);
}
```

Output:

```
Alice: 50/50 HP
goblin: 20/20 HP
troll: 45/45 HP
wraith: 30/30 HP
Alice: 40/50 HP
goblin: 10/20 HP
troll: 35/45 HP
wraith: 25/30 HP        <-- 5 damage instead of 10, because wraith
```

Look closely at that last line: the wraith took 5 damage instead of 10. Our override in `Wraith::take_damage` is correctly called through the trait object — dynamic dispatch in action.

### `Box<dyn Entity>` — what is each part?

- `Box<T>` — a heap-allocated `T`, freed when the `Box` is dropped. Rust's equivalent of "I need this on the heap."
- `dyn Entity` — "some type implementing the Entity trait, but the specific type is erased."
- Together: a heap pointer to *some* entity, with a vtable reference for its trait methods.

You cannot have a plain `dyn Entity` variable or field — it has no known size at compile time. You always wrap it in a pointer (`Box`, `&`, `Rc`, `Arc`).

### `Vec<Box<dyn Entity>>` vs `Vec<Player>`

- `Vec<Player>`: all elements are `Player`. Specialized methods. Known size per element. Fast.
- `Vec<Box<dyn Entity>>`: mix of any entity types. One pointer per element, plus the heap allocation per entity. Small overhead.

For combat, we want the mix. Use `Box<dyn ...>`.

## Step 5 — A second trait with a supertrait

Not everything is lootable. The player isn't. Let's model that.

```rust
trait Lootable: Entity {
    fn drop_loot(&self) -> Vec<String>;
}
```

`Lootable: Entity` is a **supertrait bound**: "to implement `Lootable`, you must also implement `Entity`." This lets `Lootable` methods rely on `Entity` methods internally, and it lets us write bounds like `<E: Lootable>` knowing every `E` is also an `Entity`.

Implementations:

```rust
impl Lootable for Goblin {
    fn drop_loot(&self) -> Vec<String> {
        vec!["rusty dagger".to_string(), "2 gold".to_string()]
    }
}

impl Lootable for Troll {
    fn drop_loot(&self) -> Vec<String> {
        vec!["troll hide".to_string(), "mossy club".to_string(), "10 gold".to_string()]
    }
}

impl Lootable for Wraith {
    fn drop_loot(&self) -> Vec<String> {
        vec!["spectral essence".to_string()]
    }
}

// Player is Entity but NOT Lootable — we don't impl Lootable for Player
```

Now a function that only works on lootable things:

```rust
fn collect_loot<L: Lootable>(corpse: &L) -> Vec<String> {
    assert!(corpse.is_dead(), "cannot loot a living {}", corpse.name());
    corpse.drop_loot()
}
```

Try to loot the player:

```rust
let dead_player = Player::new("Alice");
let loot = collect_loot(&dead_player);  // COMPILE ERROR
```

The compiler refuses:

```
error[E0277]: the trait bound `Player: Lootable` is not satisfied
  --> src/main.rs:123:26
   |
   |         collect_loot(&dead_player);
   |         ------------ ^^^^^^^^^^^^ the trait `Lootable` is not implemented for `Player`
```

This is the whole point of traits: unreachable code becomes impossible, caught at compile time.

## Step 6 — Putting it together

A combat round where the winner loots the loser:

```rust
fn fight_and_loot<A, B>(winner: &A, loser: &B) -> Vec<String>
where
    A: Entity,
    B: Lootable,
{
    println!("{} defeats {}!", winner.name(), loser.name());
    loser.drop_loot()
}

fn main() {
    let player = Player::new("Alice");
    let mut goblin = Goblin::new();

    goblin.take_damage(100);   // brutal finisher
    assert!(goblin.is_dead());

    let loot = fight_and_loot(&player, &goblin);
    println!("Looted: {}", loot.join(", "));
}
```

Output:

```
Alice defeats goblin!
Looted: rusty dagger, 2 gold
```

### The `where` clause

Instead of writing `fn fight_and_loot<A: Entity, B: Lootable>(...)`, we moved the bounds into a `where` clause. Semantically identical. Prefer `where` when bounds get long or complex — it reads better.

## Step 7 — Combining static and dynamic

The typical pattern in practice: your collection uses dynamic dispatch (mixed types), but individual operations are generic (fast).

```rust
fn tick(entities: &mut [Box<dyn Entity>]) {
    // iterate via trait objects (heterogeneous)
    for e in entities.iter_mut() {
        if e.is_dead() {
            println!("{} has fallen.", e.name());
        }
    }
}

fn damage_all<E: Entity>(entity: &mut E, n: i32) {
    // generic single operation (specialized per type)
    entity.take_damage(n);
}
```

You pay the vtable cost only when iterating heterogeneous data — which is exactly when you need it.

## Common pitfalls

### "The size for values of type `dyn Entity` cannot be known at compilation time"

You wrote `let e: dyn Entity = Goblin::new()` or `fn foo(e: dyn Entity)`. You always need a pointer: `Box<dyn Entity>`, `&dyn Entity`, or `&mut dyn Entity`.

### "The trait `Entity` cannot be made into an object"

A trait must be **object-safe** to use as `dyn Entity`. The main rules:

- No generic methods. `fn do_it<T>(&self, t: T)` won't work in a trait object — there's no single vtable entry for it.
- No methods returning `Self`.
- No associated constants used in object-safe contexts.

If you hit this, read the compiler's error — it tells you exactly which method broke object safety.

### Forgetting `mut` in `Box<dyn Entity>`

`for e in collection.iter_mut()` gives `&mut Box<dyn Entity>`, which auto-derefs to `&mut dyn Entity` — so you can call `take_damage(&mut self, ...)`. If you use `.iter()` (shared), you can only call `&self` methods.

### "Cannot move out of `*entity`"

Trying to move out of `Box<dyn Entity>` without consuming the `Box`. Solutions:
- Call only `&self` / `&mut self` methods.
- If you need to consume, `*entity` if `T: Sized` (doesn't work for `dyn Trait`), or rethink the design.

### Overusing `dyn`

If your function processes one type at a time, use generics. `fn describe(e: &impl Entity)` is faster and compiles nicely. Reach for `dyn` only when you actually need the heterogeneity.

## What you learned

- **Traits** declare shared behaviour; types **implement** them.
- **Default methods** share code across all implementors.
- **Static dispatch** via generics: monomorphized, zero-cost, one concrete type per call.
- **Dynamic dispatch** via `Box<dyn Trait>`: heterogeneous collections, vtable lookup.
- **Supertraits** (`trait B: A`) enforce prerequisite implementations.
- **`where` clauses** for readable multi-bound generics.
- When to choose each: generics first, `dyn` when you need the runtime flexibility.
- Object safety limits what traits can be made into trait objects.

## Exercises

1. **Combat using trait objects.** Refactor Day 2's combat so the enemy argument is `&mut dyn Entity`. This means the combat function works with any enemy type without code duplication.
2. **`Describable` with blanket impl.** Add `trait Describable: std::fmt::Display + std::fmt::Debug {}` with `impl<T: Display + Debug> Describable for T {}`. Any type with both `Display` and `Debug` automatically becomes `Describable`. This is the pattern used throughout the stdlib (e.g., `Error: Debug + Display`).
3. **`BossEntity` hierarchy.** Add `trait Boss: Entity { fn phase_transition(&mut self); }`. A dragon boss enters phase 2 at 50% HP, phase 3 at 20%. Implement the transition logic in `phase_transition` and call it from combat.
4. **`Arc<dyn Entity + Send + Sync>`.** Swap `Box<dyn Entity>` for `Arc<dyn Entity + Send + Sync>` in your combat module — this is what you'd need for multi-threaded games. Notice the `+ Send + Sync` — dynamic traits can accumulate bounds.

## What's next

Day 5 introduces **generics beyond trait bounds** — you'll build a generic `Inventory<T: Item>` where the three bags in the player's pack (weapons, potions, scrolls) are all instances of the same generic type but specialized to different item types.

→ [Day 5 — Typed inventory](day-05.md)
