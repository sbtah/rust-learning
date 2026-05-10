# Day 8 — Closures and an Event Bus

**Domain:** games • **Time:** 60–90 minutes • **Difficulty:** medium

## What you'll build

A game-wide event bus. Anywhere in your code emits an event ("player took damage", "enemy defeated"); registered handlers — expressed as closures — react. An XP counter keeps a running total. A warning handler yells when HP drops low. A discovery tracker remembers which rooms the player has visited. None of these know about each other; they're just closures sitting on the bus.

## What you'll learn

- **Closures** — anonymous functions that capture their environment
- **The three `Fn` traits** — `Fn`, `FnMut`, `FnOnce`, and when each applies
- **The `move` keyword** — forcing ownership transfer into a closure
- **`Box<dyn Fn(...)>`** — storing closures in data structures
- **`Rc<RefCell<T>>`** — shared mutable state for closures (preview; full treatment Day 10)
- How event-driven architectures look in a strongly-typed language

## Background

### What is a closure?

A closure is an anonymous function that can capture variables from its surrounding scope. Python has them (as `lambda` or nested `def`). Rust's version is stricter because of ownership rules — you have to be explicit about *how* the closure captures each variable.

```rust
let x = 10;
let add_x = |n| n + x;       // closure captures `x`
println!("{}", add_x(5));    // prints 15
```

`add_x` is a closure. Its type is anonymous — the compiler synthesizes a unique type for every closure. That's why you can't write `let f: Closure = ...` — there's no single type "Closure." You interact with closures through traits.

### The three Fn traits

Every closure implements one or more of these traits, depending on how it uses its captures:

| Trait       | Can call repeatedly? | Needs `&mut`? | Consumes captures? |
|-------------|----------------------|---------------|---------------------|
| `Fn`        | Yes                  | No            | No (shared borrow) |
| `FnMut`     | Yes                  | Yes           | No (exclusive borrow) |
| `FnOnce`    | Exactly once         | —             | Yes (may move out) |

The compiler picks the most permissive trait automatically:

```rust
let greeting = String::from("hello");

let f1 = || println!("{}", greeting);        // Fn — just reads
f1(); f1();                                  // call as many times as you like

let mut count = 0;
let mut f2 = || { count += 1; };             // FnMut — mutates `count`
f2(); f2();

let owned = String::from("world");
let f3 = || { drop(owned); };                // FnOnce — consumes `owned`
f3();                                         // calling again would be a compile error
```

### The hierarchy

`Fn` is a subtype of `FnMut`, which is a subtype of `FnOnce`. Any closure that implements `Fn` also implements `FnMut` and `FnOnce`. That means:

- If a function signature wants `FnOnce`, any closure works.
- If it wants `FnMut`, pure-read and mutating closures both work.
- If it wants `Fn`, only pure-read closures work.

Take the weakest bound that works. Most event-handler APIs want `Fn` — they want to call the handler many times, so they can't have the handler consume captures.

### `move`

By default, a closure borrows from the enclosing scope — the weakest capture possible. Sometimes you need the closure to *own* its captures:

- When the closure outlives the scope it was defined in (threads, storing in structs).
- When you're returning the closure from a function.

```rust
fn make_greeter(name: String) -> impl Fn() {
    // Without `move`, `name` would be borrowed — but `name` dies at
    // the end of `make_greeter`. So we need `move`.
    move || println!("hello, {}", name)
}
```

`move` is a one-word keyword in front of `||`. It tells the closure to take ownership of every variable it captures.

### Storing closures: `Box<dyn Fn(...)>`

Because each closure has a unique anonymous type, you can't put them in a typed collection directly. Use trait objects:

```rust
let handlers: Vec<Box<dyn Fn(i32)>> = vec![
    Box::new(|n| println!("got {}", n)),
    Box::new(|n| println!("double: {}", n * 2)),
];

for h in &handlers {
    h(7);
}
```

`Box<dyn Fn(i32)>` is "some closure that takes an `i32` and returns `()`." The compiler erases the specific type; dispatch happens through a vtable. You saw this pattern with `Box<dyn Entity>` on Day 4 — it's the same idea.

### Why `Fn` for handlers?

An event bus calls handlers many times. It calls them through a shared reference (so many components can register without fighting over `&mut` access to the bus). So handlers need `Fn`, not `FnMut`.

But handlers often need to *mutate* something (a counter, a log, a visited-set). How? Interior mutability — `Rc<RefCell<T>>`. The closure is `Fn` as far as the bus is concerned (it doesn't hold `&mut` to its captures), but internally, the `RefCell` lets it mutate what it holds. We'll formally meet `RefCell` on Day 10; today we use it as a tool.

## Setting up

```bash
cargo new day-08
cd day-08
```

No external dependencies.

## Step 1 — Define events

Start `main.rs`:

```rust
#[derive(Debug, Clone)]
pub enum Event {
    EnemyDefeated { name: String, xp: u32 },
    ItemPicked { item: String },
    PlayerHpChanged { old: i32, new: i32 },
    RoomEntered { room_id: String },
}
```

`Debug` so we can print them. `Clone` so handlers can take owned copies if they need to — though most handlers will just read the borrowed `&Event`.

## Step 2 — A minimal event bus

The simplest version: a list of handlers, each a closure. Register handlers, emit events to call them all.

```rust
pub struct EventBus {
    handlers: Vec<Box<dyn Fn(&Event)>>,
}

impl EventBus {
    pub fn new() -> EventBus {
        EventBus { handlers: Vec::new() }
    }

    pub fn on<F>(&mut self, handler: F)
    where
        F: Fn(&Event) + 'static,
    {
        self.handlers.push(Box::new(handler));
    }

    pub fn emit(&self, event: &Event) {
        for h in &self.handlers {
            h(event);
        }
    }
}
```

### Breaking this down

- `Vec<Box<dyn Fn(&Event)>>` — a list of heap-allocated trait objects, each wrapping a closure that takes `&Event` and returns `()`.
- `on<F>` is generic over the concrete closure type. The bound `F: Fn(&Event) + 'static` says "any type callable as `&Event -> ()`, and it doesn't borrow anything with a short lifetime."
- The `'static` bound is important. Handlers get stored in the bus; the bus may outlive the scope where the handler was registered. So the handler must not borrow anything that could be dropped. Owning its captures (or borrowing only `'static` data, like string literals) satisfies `'static`.
- `emit(&self, ...)` is `&self`, not `&mut self`. Reading from a `Vec<Box<dyn Fn>>` is a shared operation. We call handlers through shared borrow. That's why we require `Fn`, not `FnMut`.

### Try it

```rust
fn main() {
    let mut bus = EventBus::new();

    bus.on(|event| {
        println!("LOG: {:?}", event);
    });

    bus.emit(&Event::RoomEntered { room_id: "library".to_string() });
    bus.emit(&Event::ItemPicked { item: "scroll".to_string() });
}
```

Output:

```
LOG: RoomEntered { room_id: "library" }
LOG: ItemPicked { item: "scroll" }
```

A working event bus in about fifteen lines. But this handler is stateless. The interesting stuff needs state.

## Step 3 — A stateful handler: the XP counter

We want a handler that accumulates total XP across `EnemyDefeated` events. The counter needs to be both:

- Mutated by the handler (so `total` grows).
- Readable from outside the handler (so we can print it).

The naive attempt fails:

```rust
let mut total = 0u32;
bus.on(|event| {
    if let Event::EnemyDefeated { xp, .. } = event {
        total += xp;   // ERROR: closure captures `total` mutably
    }
});
// ... later ...
println!("total XP: {}", total);    // ERROR: `total` borrowed
```

Two problems:

1. The closure captures `total` by `&mut`, making the closure `FnMut`. But our bus wants `Fn`.
2. Even if we relaxed that, `total` is borrowed by the closure for the closure's entire lifetime. We couldn't read it from outside.

The solution is shared ownership with interior mutability: `Rc<RefCell<u32>>`.

```rust
use std::cell::RefCell;
use std::rc::Rc;

fn main() {
    let mut bus = EventBus::new();

    let total_xp = Rc::new(RefCell::new(0u32));

    // Clone the Rc — this is a cheap reference-count bump
    let total_xp_handler = Rc::clone(&total_xp);
    bus.on(move |event| {
        if let Event::EnemyDefeated { xp, .. } = event {
            *total_xp_handler.borrow_mut() += xp;
        }
    });

    bus.emit(&Event::EnemyDefeated { name: "goblin".into(), xp: 10 });
    bus.emit(&Event::EnemyDefeated { name: "troll".into(), xp: 50 });
    bus.emit(&Event::ItemPicked { item: "lantern".into() });   // ignored by this handler
    bus.emit(&Event::EnemyDefeated { name: "wraith".into(), xp: 30 });

    println!("total XP: {}", total_xp.borrow());
}
```

Run it:

```
total XP: 90
```

### What just happened

- **`Rc<T>`** — *reference counted*. `Rc::new(x)` heap-allocates `x` with a refcount of 1. `Rc::clone(&rc)` bumps the count; when the last clone drops, the data is freed. Single-threaded only (`Arc` is the thread-safe cousin).
- **`RefCell<T>`** — *interior mutability*. `cell.borrow()` returns an immutable guard; `cell.borrow_mut()` an exclusive one. Borrow rules are enforced at *runtime* — if you call `borrow_mut` while another borrow is live, it panics.
- **`Rc::clone(&total_xp)`** — we clone the `Rc` before moving it into the closure, so the outer `total_xp` is still usable. You'll see this pattern endlessly: clone the refcount, move the clone in.
- **`move |event| { ... }`** — we use `move` because the closure outlives the scope where `total_xp_handler` was defined. Without `move`, the closure borrows `total_xp_handler`, which dies at the end of `main` (well, technically it doesn't here, but the compiler is pessimistic about it). `move` makes the closure own the `Rc` clone.
- **`*cell.borrow_mut() += xp`** — `borrow_mut()` returns a `RefMut<u32>`, which derefs to `&mut u32`. `*` gives you the `u32`. Then `+= xp` on that. (If it feels tortured, it's because `RefCell` is a workaround for the type system, not a first-class feature.)
- **`println!("...", total_xp.borrow())`** — outside the closure, we can borrow and read, because `RefCell` allows any number of shared borrows as long as no exclusive borrow is held.

### Is the closure `Fn` or `FnMut`?

`Fn`. That's the magic. From the closure's point of view, it holds an `Rc<RefCell<u32>>` (immutable data — the pointer itself doesn't change). It goes *through* the `RefCell` to mutate. No `&mut self` required on the closure.

That's why the bus can store this handler under an `Fn` bound and call it many times through a shared reference.

## Step 4 — Low-HP warning

Pure `Fn` with no captures beyond the moved-in string:

```rust
bus.on(|event| {
    if let Event::PlayerHpChanged { new, .. } = event {
        if *new <= 10 {
            println!("*** WARNING: HP low ({}) ***", new);
        }
    }
});
```

`*new <= 10` — `new` is `&i32` (destructured from `&Event`), so `*new` is `i32`.

No shared state needed. The simplest kind of handler.

## Step 5 — Discovery tracker

Remember which rooms the player has visited. Print an announcement for each new room.

```rust
use std::collections::HashSet;

let discovered = Rc::new(RefCell::new(HashSet::<String>::new()));
let discovered_handler = Rc::clone(&discovered);
bus.on(move |event| {
    if let Event::RoomEntered { room_id } = event {
        let mut set = discovered_handler.borrow_mut();
        if set.insert(room_id.clone()) {
            // `insert` returns true if the value was newly added
            println!("NEW: discovered '{}'", room_id);
        }
    }
});
```

### One important detail

Look at the order of operations here:

```rust
let mut set = discovered_handler.borrow_mut();
if set.insert(room_id.clone()) { ... }
```

We hold `set` (a `RefMut`) across `insert` and the `println!`. That's fine because nothing in the `println!` tries to borrow `discovered_handler` again.

But if we had:

```rust
let mut set = discovered_handler.borrow_mut();
if set.insert(room_id.clone()) {
    dump_all_rooms(&discovered_handler);   // tries to borrow while borrow_mut is active
}
```

this would **panic at runtime**: "already borrowed mutably". That's `RefCell`'s runtime borrow checking in action. It's a real hazard — hold `RefCell` guards as briefly as possible.

## Step 6 — Put it all together

```rust
use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;

#[derive(Debug, Clone)]
pub enum Event {
    EnemyDefeated { name: String, xp: u32 },
    ItemPicked { item: String },
    PlayerHpChanged { old: i32, new: i32 },
    RoomEntered { room_id: String },
}

pub struct EventBus {
    handlers: Vec<Box<dyn Fn(&Event)>>,
}

impl EventBus {
    pub fn new() -> EventBus {
        EventBus { handlers: Vec::new() }
    }
    pub fn on<F: Fn(&Event) + 'static>(&mut self, handler: F) {
        self.handlers.push(Box::new(handler));
    }
    pub fn emit(&self, event: &Event) {
        for h in &self.handlers {
            h(event);
        }
    }
}

fn main() {
    let mut bus = EventBus::new();

    // 1. Logger
    bus.on(|event| println!("[evt] {:?}", event));

    // 2. XP counter
    let total_xp = Rc::new(RefCell::new(0u32));
    {
        let xp = Rc::clone(&total_xp);
        bus.on(move |event| {
            if let Event::EnemyDefeated { xp: n, .. } = event {
                *xp.borrow_mut() += n;
            }
        });
    }

    // 3. Low-HP warning
    bus.on(|event| {
        if let Event::PlayerHpChanged { new, .. } = event {
            if *new <= 10 {
                println!("*** WARNING: HP low ({}) ***", new);
            }
        }
    });

    // 4. Discovery tracker
    let discovered = Rc::new(RefCell::new(HashSet::<String>::new()));
    {
        let d = Rc::clone(&discovered);
        bus.on(move |event| {
            if let Event::RoomEntered { room_id } = event {
                if d.borrow_mut().insert(room_id.clone()) {
                    println!("NEW: discovered '{}'", room_id);
                }
            }
        });
    }

    // ---- Simulate some gameplay ----
    bus.emit(&Event::RoomEntered { room_id: "library".into() });
    bus.emit(&Event::PlayerHpChanged { old: 50, new: 42 });
    bus.emit(&Event::EnemyDefeated { name: "goblin".into(), xp: 15 });
    bus.emit(&Event::ItemPicked { item: "lantern".into() });
    bus.emit(&Event::RoomEntered { room_id: "library".into() });   // not new
    bus.emit(&Event::RoomEntered { room_id: "dungeon".into() });
    bus.emit(&Event::PlayerHpChanged { old: 42, new: 8 });

    println!();
    println!("--- Stats ---");
    println!("Total XP: {}", total_xp.borrow());
    println!("Rooms discovered: {:?}", discovered.borrow());
}
```

Run it:

```
[evt] RoomEntered { room_id: "library" }
NEW: discovered 'library'
[evt] PlayerHpChanged { old: 50, new: 42 }
[evt] EnemyDefeated { name: "goblin", xp: 15 }
[evt] ItemPicked { item: "lantern" }
[evt] RoomEntered { room_id: "library" }
[evt] RoomEntered { room_id: "dungeon" }
NEW: discovered 'dungeon'
[evt] PlayerHpChanged { old: 42, new: 8 }
*** WARNING: HP low (8) ***

--- Stats ---
Total XP: 15
Rooms discovered: {"library", "dungeon"}
```

Four independent handlers, one event stream, zero coupling. Adding a fifth handler is three lines. The bus never learns about XP counters, HP warnings, or room sets — it only knows how to store and call `Fn(&Event)`.

## Step 7 — Per-kind dispatch (optional but useful)

All four of our handlers start with `if let Event::X { .. } = event`. Repetitive. Let's give the bus a way to register for one variant only.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EventKind {
    EnemyDefeated,
    ItemPicked,
    PlayerHpChanged,
    RoomEntered,
}

impl Event {
    pub fn kind(&self) -> EventKind {
        match self {
            Event::EnemyDefeated { .. } => EventKind::EnemyDefeated,
            Event::ItemPicked { .. }    => EventKind::ItemPicked,
            Event::PlayerHpChanged { .. } => EventKind::PlayerHpChanged,
            Event::RoomEntered { .. }   => EventKind::RoomEntered,
        }
    }
}
```

Now the bus:

```rust
use std::collections::HashMap;

pub struct EventBus {
    all_handlers: Vec<Box<dyn Fn(&Event)>>,
    by_kind: HashMap<EventKind, Vec<Box<dyn Fn(&Event)>>>,
}

impl EventBus {
    pub fn new() -> EventBus {
        EventBus {
            all_handlers: Vec::new(),
            by_kind: HashMap::new(),
        }
    }

    pub fn on<F: Fn(&Event) + 'static>(&mut self, handler: F) {
        self.all_handlers.push(Box::new(handler));
    }

    pub fn on_kind<F: Fn(&Event) + 'static>(&mut self, kind: EventKind, handler: F) {
        self.by_kind.entry(kind).or_default().push(Box::new(handler));
    }

    pub fn emit(&self, event: &Event) {
        for h in &self.all_handlers {
            h(event);
        }
        if let Some(specific) = self.by_kind.get(&event.kind()) {
            for h in specific {
                h(event);
            }
        }
    }
}
```

Now handlers that care about exactly one kind can skip the `if let`:

```rust
let xp = Rc::clone(&total_xp);
bus.on_kind(EventKind::EnemyDefeated, move |event| {
    if let Event::EnemyDefeated { xp: n, .. } = event {
        *xp.borrow_mut() += n;
    }
});
```

The `if let` is still there (because `event: &Event` — we need to extract fields), but now the closure is only *called* for defeats, saving work on every other kind.

### A cleaner version with typed handlers

If you're feeling brave, a small amount of extra indirection gives you genuinely typed handlers. This is a pattern you see in more "serious" event-driven libraries:

```rust
pub fn on_enemy_defeated<F: Fn(&str, u32) + 'static>(&mut self, handler: F) {
    self.on_kind(EventKind::EnemyDefeated, move |event| {
        if let Event::EnemyDefeated { name, xp } = event {
            handler(name, *xp);
        }
    });
}
```

Now callers write:

```rust
bus.on_enemy_defeated(move |_name, xp| {
    *total_xp_handler.borrow_mut() += xp;
});
```

No destructuring in user code. This is what the stretch exercise at the bottom builds toward.

## Common pitfalls

### "expected `Fn`, found `FnMut`"

You wrote a closure that takes `&mut` on a capture:

```rust
let mut count = 0;
bus.on(|event| {
    count += 1;                    // FnMut — won't fit Fn bound
});
```

Compiler error:

```
expected a closure that implements the `Fn` trait, but this closure only implements `FnMut`
```

Fix: put the mutable state behind `Rc<RefCell<T>>`.

### "use of moved value"

A closure `move`'d something, you tried to use the original afterwards:

```rust
let s = String::from("hi");
bus.on(move |_| println!("{}", s));
println!("{}", s);                 // ERROR: s moved into closure
```

Fix: clone before moving — `let s2 = s.clone(); bus.on(move |_| println!("{}", s2));`

### "already borrowed: BorrowMutError"

Runtime panic from `RefCell`. You called `borrow_mut` while another borrow was live:

```rust
let cell = RefCell::new(5);
let r1 = cell.borrow();        // shared borrow held
let r2 = cell.borrow_mut();    // PANIC
```

Hard to hit in toy examples; common in real code when handlers call functions that re-enter the bus or touch the same state. Fix: scope your borrows as tightly as possible. Drop the guard before doing anything that might re-borrow.

### Forgetting `+ 'static`

The bus stores the handler for an unbounded time. Without `+ 'static` in the bound, the closure might borrow something that dies before the handler is called — undefined behavior. The compiler enforces it, but the error message can be cryptic:

```
error[E0373]: closure may outlive the current function
```

Fix: add `+ 'static` to the bound. Usually that means using `move` and giving the closure ownership of what it captures.

### Using `&mut self` on `emit`

If `emit` were `&mut self`, the bus couldn't be shared — only one emitter at a time. The `Fn` bound on handlers plus `&self` on `emit` means the bus is cheap to share and call from many places. That's a key design choice.

## What you learned

- **Closures** capture their environment; their type is anonymous.
- The three **Fn traits**: `Fn` (shared read), `FnMut` (exclusive mutation), `FnOnce` (consumes).
- **`move`** forces a closure to own its captures.
- **`Box<dyn Fn(&Event)>`** stores closures of different concrete types in one collection.
- **`Rc<RefCell<T>>`** gives you shared mutable state that closures can use while remaining `Fn`.
- **`'static` bounds** on stored closures.
- Per-kind dispatch via `HashMap<EventKind, Vec<Handler>>`.

## Exercises

1. **Unregister handlers.** Currently handlers live forever. Add `fn on(&mut self, handler: F) -> HandlerId` returning an opaque ID, and `fn off(&mut self, id: HandlerId)` that removes the matching handler.
2. **Typed events.** Implement the "typed handler" sketch from Step 7 for all four event kinds. Compare ergonomics — is the destructuring worth avoiding?
3. **Priorities.** Add a priority argument to `on`. Handlers fire in priority order (high first). Test by registering two handlers at different priorities for the same event.
4. **Event queueing.** Add `fn queue(&mut self, event: Event)` that stores events, and `fn flush(&mut self)` that fires all queued events. Useful for ordering — ensures all effects of "room entered" fire before the next event begins.
5. **Wire into the adventure.** Take your Week 1 text adventure and wire the event bus in. Combat emits `EnemyDefeated`, `take` emits `ItemPicked`, damage emits `PlayerHpChanged`, movement emits `RoomEntered`. Build a simple achievement system on top (closure-based, of course).

## What's next

Day 9 is Rust's most-feared topic: **explicit lifetimes**. Don't worry — most days you never touch them because the compiler infers them. But writing a zero-copy parser *forces* you to confront them, and once you have, they stop being mysterious. We'll build a config parser where every key and value is a `&str` borrow into the original source — no allocations at all.

→ [Day 9 — Explicit lifetimes](day-09.md)
