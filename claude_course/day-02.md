# Day 2 — Combat System

**Domain:** games • **Time:** 60–90 minutes • **Difficulty:** easy–medium

## What you'll build

A turn-based combat system bolted onto yesterday's text adventure. The player encounters enemies in certain rooms; combat runs as a turn-by-turn loop where both sides attack, heal, cast spells, or flee. Enemies have distinct behaviors. Status effects (Poisoned, Shielded) persist across turns and tick down.

## What you'll learn

- **Enums with data** — sum types, Rust's single best type-system feature
- **`match` exhaustiveness** — the compiler proves you handled every case
- **Pattern matching** on enum variants, with destructuring and guards
- **State machines** modelled as enums
- Using the **`rand` crate** for randomness

## Background

### Enums aren't just C-style enums

In Python or C, enums are named integers. In Rust, an enum is a **sum type**: a value is exactly one of several variants, and each variant can carry its own data.

```rust
enum Shape {
    Circle { radius: f32 },
    Rectangle { width: f32, height: f32 },
    Triangle(f32, f32, f32),   // tuple variant
}

let s = Shape::Circle { radius: 3.0 };
let r = Shape::Rectangle { width: 4.0, height: 2.0 };
```

A `Shape` value is *always* exactly one of these three — never none, never two at once. The compiler tracks which variant it is and makes you handle each case when you access the data.

### Pattern matching with `match`

`match` is exhaustive: you must handle every variant, or the compiler refuses to build.

```rust
fn area(shape: &Shape) -> f32 {
    match shape {
        Shape::Circle { radius } => 3.14159 * radius * radius,
        Shape::Rectangle { width, height } => width * height,
        Shape::Triangle(a, b, c) => {
            let s = (a + b + c) / 2.0;
            (s * (s - a) * (s - b) * (s - c)).sqrt()
        }
    }
}
```

This is the whole point of enums in Rust. If you later add `Shape::Pentagon`, the compiler errors on *every* `match` that doesn't handle it. There is no silent `KeyError`, no default branch swallowing a new case.

You can use `_` as a catch-all wildcard, but **don't** — that defeats exhaustiveness. Write out every variant.

### Patterns with guards

You can add conditions to a match arm:

```rust
match damage {
    n if n > 100 => println!("Massive hit! {}", n),
    n if n > 50  => println!("Solid hit: {}", n),
    n            => println!("Weak hit: {}", n),
}
```

And you can destructure nested patterns:

```rust
match action {
    Action::Attack { weapon: Weapon::Sword { sharpness }, target } if sharpness > 10 => {
        println!("Critical sword strike on {}", target);
    }
    ...
}
```

### The `rand` crate

Rust's stdlib doesn't include random number generation — you pull in the `rand` crate. Add it to your project with:

```bash
cargo add rand@0.8
```

Then use it:

```rust
use rand::Rng;

let mut rng = rand::thread_rng();
let n: u32 = rng.gen_range(1..=20);   // uniform 1..=20 inclusive
```

`thread_rng()` gives you a fast thread-local RNG. `gen_range(lo..hi)` is exclusive on the upper; `lo..=hi` is inclusive.

## Setting up

Start today's project inside your `rust-30/` folder:

```bash
cargo new day-02
cd day-02
cargo add rand@0.8
```

Verify `Cargo.toml` now has:

```toml
[dependencies]
rand = "0.8"
```

## Step 1 — Define enemies and actions

Open `src/main.rs` and start fresh:

```rust
use rand::Rng;

#[derive(Debug, Clone, Copy)]
enum EnemyKind {
    Goblin,
    Troll,
    Wraith,
}

#[derive(Debug)]
enum Spell {
    Fireball,
    Shield,
}

#[derive(Debug)]
enum PlayerAction {
    Attack,
    Heal,
    Run,
    Cast(Spell),
}

fn main() {
    let e = EnemyKind::Troll;
    let a = PlayerAction::Cast(Spell::Fireball);
    println!("{:?} vs {:?}", e, a);
}
```

Run it:

```bash
cargo run
```

Output:

```
Troll vs Cast(Fireball)
```

### What `#[derive(...)]` does

These attributes ask the compiler to auto-generate common trait implementations. Today we use:

- `Debug` — lets us print with `{:?}`.
- `Clone, Copy` — `EnemyKind` is a plain tag (no heap data), so we mark it `Copy`. Copying one is as cheap as copying an integer. We don't derive `Copy` on `PlayerAction` because `Spell` might later hold heap data.

You'll see these derives everywhere. They save tons of boilerplate.

### Tuple vs struct variants

- `Cast(Spell)` is a **tuple variant** — unnamed fields.
- `Poisoned { turns_left: u32 }` (we'll add this shortly) is a **struct variant** — named fields.

Use struct variants when there are multiple fields or the meaning isn't obvious from position.

## Step 2 — Enemy stats

Each enemy kind has its own stats. We could hardcode them per-enemy, but a method on `EnemyKind` keeps everything in one place:

```rust
impl EnemyKind {
    fn max_hp(self) -> i32 {
        match self {
            EnemyKind::Goblin => 20,
            EnemyKind::Troll  => 45,
            EnemyKind::Wraith => 30,
        }
    }

    fn name(self) -> &'static str {
        match self {
            EnemyKind::Goblin => "goblin",
            EnemyKind::Troll  => "troll",
            EnemyKind::Wraith => "wraith",
        }
    }
}
```

A few things to notice:

- The methods take `self` (not `&self`) because `EnemyKind: Copy` — passing by value is free.
- Return type `&'static str`: a string that lives for the whole program (string literals are `'static`). We'll dig into lifetimes on Day 9.
- `match` is exhaustive. Add a new variant `Vampire` to `EnemyKind` and these methods won't compile until you add arms for it.

Define an `Enemy` struct holding current state:

```rust
struct Enemy {
    kind: EnemyKind,
    hp: i32,
}

impl Enemy {
    fn new(kind: EnemyKind) -> Enemy {
        Enemy { hp: kind.max_hp(), kind }
    }

    fn is_dead(&self) -> bool {
        self.hp <= 0
    }
}
```

Update `main` to make sure it all compiles:

```rust
fn main() {
    let goblin = Enemy::new(EnemyKind::Goblin);
    println!("{} has {} HP", goblin.kind.name(), goblin.hp);
}
```

Run it:

```
goblin has 20 HP
```

## Step 3 — Player and status effects

Here's where enums really shine. A player can be in one of several *states*, each with its own data. Instead of `is_poisoned: bool` + `poison_turns_left: i32` + `is_shielded: bool` + ... we express it as one enum:

```rust
#[derive(Debug)]
enum PlayerStatus {
    Normal,
    Poisoned { turns_left: u32 },
    Shielded { turns_left: u32 },
}

struct Player {
    hp: i32,
    max_hp: i32,
    status: PlayerStatus,
}

impl Player {
    fn new() -> Player {
        Player { hp: 50, max_hp: 50, status: PlayerStatus::Normal }
    }

    fn is_dead(&self) -> bool {
        self.hp <= 0
    }
}
```

The beauty here: you cannot have a `turns_left` count without being `Poisoned` or `Shielded`. You cannot be `Normal` with a stray poison counter hanging around. The type system enforces what's possible.

### Applying status effects each turn

```rust
impl Player {
    fn tick_status(&mut self) {
        self.status = match std::mem::replace(&mut self.status, PlayerStatus::Normal) {
            PlayerStatus::Normal => PlayerStatus::Normal,
            PlayerStatus::Poisoned { turns_left } => {
                self.hp -= 5;
                println!("Poison drains 5 HP.");
                if turns_left > 1 {
                    PlayerStatus::Poisoned { turns_left: turns_left - 1 }
                } else {
                    println!("The poison fades.");
                    PlayerStatus::Normal
                }
            }
            PlayerStatus::Shielded { turns_left } => {
                if turns_left > 1 {
                    PlayerStatus::Shielded { turns_left: turns_left - 1 }
                } else {
                    println!("Your shield crumbles.");
                    PlayerStatus::Normal
                }
            }
        };
    }
}
```

### `std::mem::replace` — why the acrobatics?

We want to both *read* the current status and *overwrite* it with a new one. If we just did `match self.status`, we'd be moving out of a borrowed `&mut self` — the compiler won't allow it because `self.status` must always hold something valid.

`std::mem::replace(dest, new_value)` swaps: it puts `new_value` into `*dest` and returns whatever was there. We use `PlayerStatus::Normal` as a placeholder, then immediately assign the real new value. Net effect: we got to destructure the old state without leaving `self.status` in limbo.

This pattern comes up a lot. There's a crate (`replace_with`) that hides it, but knowing the raw version matters.

## Step 4 — The combat loop

Time to wire it all up. Replace `main`:

```rust
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

enum CombatOutcome {
    Victory,
    Defeat,
    Fled,
}

fn combat(player: &mut Player, mut enemy: Enemy) -> CombatOutcome {
    let mut rng = rand::thread_rng();

    loop {
        println!(
            "\nYou: {}/{} HP ({:?}) | {}: {} HP",
            player.hp, player.max_hp, player.status, enemy.kind.name(), enemy.hp
        );
        println!("  1) Attack  2) Heal  3) Run  4) Cast Fireball  5) Cast Shield");

        print!("> ");
        io::stdout().flush().ok();

        let input = match read_line() {
            Some(s) => s,
            None => return CombatOutcome::Fled,
        };

        let action = match input.as_str() {
            "1" => PlayerAction::Attack,
            "2" => PlayerAction::Heal,
            "3" => PlayerAction::Run,
            "4" => PlayerAction::Cast(Spell::Fireball),
            "5" => PlayerAction::Cast(Spell::Shield),
            _ => {
                println!("Pick 1-5.");
                continue;
            }
        };

        // --- Player's turn ---
        match &action {
            PlayerAction::Attack => {
                let dmg = rng.gen_range(8..=15);
                enemy.hp -= dmg;
                println!("You strike for {} damage.", dmg);
            }
            PlayerAction::Heal => {
                let amt = rng.gen_range(10..=20);
                player.hp = (player.hp + amt).min(player.max_hp);
                println!("You recover {} HP.", amt);
            }
            PlayerAction::Run => {
                if rng.gen_bool(0.5) {
                    println!("You flee successfully.");
                    return CombatOutcome::Fled;
                } else {
                    println!("You failed to escape!");
                }
            }
            PlayerAction::Cast(Spell::Fireball) => {
                let dmg = rng.gen_range(20..=30);
                enemy.hp -= dmg;
                println!("Fireball! {} damage.", dmg);
            }
            PlayerAction::Cast(Spell::Shield) => {
                player.status = PlayerStatus::Shielded { turns_left: 3 };
                println!("A shimmering shield surrounds you.");
            }
        }

        if enemy.is_dead() {
            println!("The {} falls.", enemy.kind.name());
            return CombatOutcome::Victory;
        }

        // --- Enemy's turn ---
        let raw_damage = enemy_attack(&enemy, &mut rng);
        let damage = match &player.status {
            PlayerStatus::Shielded { .. } => raw_damage / 2,
            _ => raw_damage,
        };
        player.hp -= damage;
        println!("The {} hits you for {} damage.", enemy.kind.name(), damage);

        // Wraith poisons on hit
        if matches!(enemy.kind, EnemyKind::Wraith)
            && !matches!(player.status, PlayerStatus::Poisoned { .. })
            && rng.gen_bool(0.4)
        {
            player.status = PlayerStatus::Poisoned { turns_left: 3 };
            println!("You feel sickness creep into your bones.");
        }

        // --- End-of-turn effects ---
        player.tick_status();

        if player.is_dead() {
            return CombatOutcome::Defeat;
        }
    }
}

fn enemy_attack(enemy: &Enemy, rng: &mut impl Rng) -> i32 {
    match enemy.kind {
        EnemyKind::Goblin => rng.gen_range(3..=8),
        EnemyKind::Troll  => 15,                       // slow but hits hard
        EnemyKind::Wraith => rng.gen_range(6..=10),
    }
}

fn main() {
    let mut player = Player::new();
    println!("A wraith materializes before you!");
    let wraith = Enemy::new(EnemyKind::Wraith);

    match combat(&mut player, wraith) {
        CombatOutcome::Victory => println!("\nVictorious! You have {} HP left.", player.hp),
        CombatOutcome::Defeat  => println!("\nYou have been slain."),
        CombatOutcome::Fled    => println!("\nYou escape with your life."),
    }
}
```

Run it:

```bash
cargo run
```

```
A wraith materializes before you!

You: 50/50 HP (Normal) | wraith: 30 HP
  1) Attack  2) Heal  3) Run  4) Cast Fireball  5) Cast Shield
> 4
Fireball! 23 damage.
The wraith hits you for 7 damage.

You: 43/50 HP (Normal) | wraith: 7 HP
  1) Attack  2) Heal  3) Run  4) Cast Fireball  5) Cast Shield
> 1
You strike for 12 damage.
The wraith falls.

Victorious! You have 43 HP left.
```

### Breaking down some details

**`matches!(expr, pattern)`** — a macro that returns `true` if `expr` matches `pattern`. It's shorthand for:

```rust
match enemy.kind {
    EnemyKind::Wraith => true,
    _ => false,
}
```

The `_` wildcard is acceptable *inside* `matches!` because we're only asking "is it this?". Don't use `_` in full `match` expressions that compute real logic — you lose exhaustiveness.

**`PlayerStatus::Shielded { .. }`** — destructure but ignore all fields. We care that the player is shielded, not by how many turns remain.

**`enemy_attack(enemy: &Enemy, rng: &mut impl Rng)`** — `impl Rng` means "any type implementing the `Rng` trait." We'll make this style of generic programming a theme in Day 5.

**`action` is matched by reference with `&action`** — because we only read from it, we borrow rather than consume. Without `&`, the `match` would move `action` and we couldn't use it again (not that we do, but it's a good habit).

## Step 5 — Bonus: multiple encounters

As written, combat is one fight and done. Let's chain a few:

Replace `main`:

```rust
fn main() {
    let mut player = Player::new();

    let encounters = vec![
        EnemyKind::Goblin,
        EnemyKind::Goblin,
        EnemyKind::Troll,
        EnemyKind::Wraith,
    ];

    for (i, kind) in encounters.into_iter().enumerate() {
        println!("\n=== Encounter {} ===", i + 1);
        println!("A {} leaps out!", kind.name());

        let enemy = Enemy::new(kind);
        match combat(&mut player, enemy) {
            CombatOutcome::Victory => println!("Defeated the {}.", kind.name()),
            CombatOutcome::Defeat => {
                println!("You have been slain.");
                return;
            }
            CombatOutcome::Fled => {
                println!("You flee and lick your wounds.");
            }
        }
    }

    println!("\nYou survived all encounters with {}/{} HP.", player.hp, player.max_hp);
}
```

### `into_iter().enumerate()`

- `.into_iter()` consumes `encounters`, yielding owned values.
- `.enumerate()` yields `(index, value)` pairs.

This is the Rust equivalent of Python's `enumerate()`. We'll meet iterators fully on Day 6.

### Why `for (i, kind) in ...` and not `for (i, kind) in &...`

`&encounters.iter().enumerate()` would yield `(index, &EnemyKind)`. Since `EnemyKind: Copy`, we could still use it — but `into_iter()` is cleaner when you're done with the vec.

Also: notice I used `kind` after combat (to print "Defeated the goblin"). I can do this because `kind` is `Copy` — moving it into `Enemy::new(kind)` makes a copy; the original `kind` variable is still valid.

## Common pitfalls

### "Non-exhaustive patterns"

The compiler error you *want* to see often. When you add a new variant to an enum, every `match` on that enum fails until you handle the new case. Don't reach for `_` to silence it — add the proper arm. Exhaustiveness is the feature.

### "Use of moved value"

You matched on an enum by value (not `&enum`) and then tried to use the variable again:

```rust
let a = PlayerAction::Attack;
match a {
    PlayerAction::Attack => println!("attack!"),
    _ => {}
}
println!("{:?}", a);   // ERROR: a was moved into the match
```

Solution: `match &a` or `match a.clone()`.

### Forgetting to tick status

Your poison doesn't do damage. Check: is `player.tick_status()` called *every* turn, even the ones you flee or escape from? Make a test: `while !player.is_dead() { player.tick_status(); }`. If it runs forever, tick doesn't decrement.

### `gen_range` panics

`rng.gen_range(5..5)` panics because the range is empty. `gen_range(5..=5)` is fine (yields 5). Be careful with dynamic ranges.

## What you learned

- **Enums with data**: variants can carry their own fields.
- **Exhaustive `match`**: the compiler proves every case is handled.
- **Pattern destructuring**: extract fields from variants directly.
- **Guards** in match arms (`if condition`).
- **`matches!` macro** for boolean checks.
- **`std::mem::replace`** for "destructure and replace" in `&mut` contexts.
- **State machines** modelled as enums.
- **The `rand` crate**: `thread_rng`, `gen_range`, `gen_bool`.

## Exercises

1. **Weapon enum.** Add `enum Weapon { Fists, Sword { sharpness: u32 }, Staff { spell: Spell } }` to `Player`. Attack damage depends on the weapon. A `Staff { spell: Spell::Fireball }` combined with `PlayerAction::Cast(Spell::Fireball)` gets a synergy bonus — express that with nested pattern matching in a single `match` arm.
2. **Enemy AI.** Currently the troll always hits for 15. Make the troll *sometimes* roar (warning the player) and hit hard next turn. Model it with an `EnemyState` enum on the `Enemy` struct.
3. **Poison stacking.** If the player is already poisoned and gets poisoned again, refresh to the higher duration. Implement with pattern matching, not an `if-else` chain.
4. **Wire it into Day 1.** Certain rooms have an enemy. Entering triggers combat. If the player flees, they bounce back to the previous room.

## What's next

Day 3 introduces `Option<T>`, `Result<T, E>`, and the `?` operator — Rust's answer to Python's `None` and exceptions. You'll use them to add `save` and `load` commands to the adventure, then write file I/O that properly propagates errors without a single `.unwrap()`.

→ [Day 3 — Save and load](day-03.md)
