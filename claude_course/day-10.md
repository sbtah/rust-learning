# Day 10 — Smart Pointers: Box, Rc, RefCell

**Domain:** games + 3D preview • **Time:** 90 minutes • **Difficulty:** medium–hard

## What you'll build

A scene graph — a tree of nodes, each with a name, a local 3D position, and a list of children. Traverse the tree depth-first, computing world positions by summing local positions up the ancestor chain. Along the way you'll meet every core smart pointer and understand which one to reach for and why.

## What you'll learn

- **`Box<T>`** — single owner, heap-allocated; required for recursive types
- **`Rc<T>`** — reference-counted shared ownership (single-threaded)
- **`RefCell<T>`** — interior mutability, borrow rules enforced at runtime
- **`Weak<T>`** — non-owning Rc reference, for breaking cycles
- When to use each — and what goes wrong when you use the wrong one
- A preview of `Arc<T>` + `Mutex<T>` for multi-threaded sharing (Day 14)

## Background

### The problem: one owner isn't always enough

Rust's default is single ownership. Every value has exactly one owner; when the owner goes out of scope, the value is dropped. That model is beautiful for most data, but it breaks down for:

- **Trees and graphs** where a node is referenced from multiple places.
- **Recursive types** where a node contains other nodes of the same type (the size depends on itself).
- **Callbacks and observers** that need shared, mutable state (we saw this on Day 8).
- **Thread sharing** (Day 14).

Rust's answer is *smart pointer types* — each encodes a specific sharing model with explicit costs.

### Box<T>: heap allocation + single ownership

`Box<T>` puts a `T` on the heap, owned by the `Box`. When the `Box` drops, the `T` is freed.

```rust
let b: Box<i32> = Box::new(5);
println!("{}", *b);     // deref to access the i32
```

You can use `Box<T>` anywhere you'd use `T` — it's mostly transparent. The main reasons to box:

1. **Recursive types.** `struct Tree { left: Tree, right: Tree }` has infinite size (each Tree contains Trees contains Trees). `struct Tree { left: Box<Tree>, right: Box<Tree> }` has a fixed size — two pointers.
2. **Trait objects.** `Box<dyn Trait>` for heterogeneous collections — you saw this Day 4.
3. **Large values** you don't want to copy around on the stack.

### Rc<T>: shared, refcounted ownership

`Rc<T>` (reference counted) heap-allocates `T` and tracks how many `Rc` clones refer to it. When the last one drops, the data is freed.

```rust
use std::rc::Rc;

let a = Rc::new(String::from("hello"));
let b = Rc::clone(&a);    // refcount: 2
let c = Rc::clone(&a);    // refcount: 3
// a, b, c all point to the same heap String
drop(b);                   // refcount: 2
// String still alive — still two owners
```

**Key property**: the data inside is *immutable* through an `Rc`. You get `&T`, never `&mut T`. If multiple things share ownership, nobody can unilaterally mutate — that would violate Rust's aliasing rules.

**Single-threaded only.** `Rc` uses non-atomic refcount bumps for speed. Cross a thread boundary with it and the compiler rejects you. For multi-threaded sharing, use `Arc<T>` (atomically counted) — same API, slightly slower, `Send + Sync`.

### RefCell<T>: interior mutability

What if multiple owners really do need to mutate the shared data? You use `RefCell<T>` to escape the "no `&mut` through `Rc`" rule.

```rust
use std::cell::RefCell;

let cell = RefCell::new(5);
*cell.borrow_mut() = 10;            // mutate through shared ref
println!("{}", *cell.borrow());     // read through shared ref
```

Key trick: `RefCell::borrow()` and `borrow_mut()` check the aliasing rules **at runtime** instead of compile time. You can have many shared borrows or one exclusive borrow — same rules, enforced by the runtime. Violating them panics.

```rust
let a = cell.borrow_mut();
let b = cell.borrow();     // PANIC: already mutably borrowed
```

`RefCell<T>` doesn't let you escape the rules, just postpones their check. This has costs:

- Runtime overhead (small but real).
- Bugs become panics instead of compile errors.
- You give up the compiler's guarantee.

Don't reach for `RefCell` until you've tried to structure your code without it.

### Rc<RefCell<T>>: the combo

Very common. `Rc` gives shared ownership; `RefCell` gives interior mutability. Together: multiple owners of mutable shared state.

```rust
let shared = Rc::new(RefCell::new(vec![1, 2, 3]));

let handler_a = Rc::clone(&shared);
let handler_b = Rc::clone(&shared);

handler_a.borrow_mut().push(4);
handler_b.borrow_mut().push(5);

println!("{:?}", shared.borrow());    // [1, 2, 3, 4, 5]
```

We used this on Day 8 for the event bus.

### Weak<T>: the tiebreaker

A problem with `Rc`: if two nodes point at each other (parent ↔ child), their refcounts never reach zero — memory leak.

```rust
// Parent holds Rc to child; child holds Rc to parent.
// Both stuck at refcount ≥ 1 forever.
```

`Weak<T>` is an `Rc` that *doesn't contribute to the refcount*. You can upgrade a `Weak` to an `Rc` via `.upgrade() -> Option<Rc<T>>` — `Some` if the data is still alive, `None` if it's been freed.

Use `Weak` for the "back-edge" in a parent-child relationship. Parent owns children via `Rc`; child has `Weak` reference to parent. No cycle.

### The decision chart

| Need                                        | Reach for                     |
|---------------------------------------------|-------------------------------|
| Put something on the heap                   | `Box<T>`                      |
| Recursive types                             | `Box<T>`                      |
| Trait object                                | `Box<dyn Trait>`              |
| Shared read-only ownership, single-threaded | `Rc<T>`                       |
| Shared read-only ownership, multi-threaded  | `Arc<T>`                      |
| Mutate through a shared ref                 | `RefCell<T>` (single thread) or `Mutex<T>` / `RwLock<T>` (multi) |
| Shared *and* mutable ownership              | `Rc<RefCell<T>>` or `Arc<Mutex<T>>` |
| Back-edges without cycles                   | `Weak<T>`                     |

## Setting up

```bash
cargo new day-10
cd day-10
```

No dependencies.

## Step 1 — Why `Box` for recursive types

Try to define a binary tree:

```rust
struct Tree {
    value: i32,
    left: Tree,       // won't compile
    right: Tree,
}
```

Compile:

```
error[E0072]: recursive type `Tree` has infinite size
 --> src/main.rs:1:1
  |
1 | struct Tree {
  | ^^^^^^^^^^^
2 |     value: i32,
3 |     left: Tree,
  |           ---- recursive without indirection
```

Every `Tree` contains a `Tree` which contains a `Tree`… Size is undefined. Fix:

```rust
struct Tree {
    value: i32,
    left: Option<Box<Tree>>,
    right: Option<Box<Tree>>,
}
```

`Box` is a fixed-size pointer (8 bytes on 64-bit); `Option<Box<...>>` is also fixed size. The recursion happens through the heap.

This is why `Box` is so often used for tree-like structures. Enough warm-up — let's build the real thing.

## Step 2 — The scene graph Node

Here's our target structure:

```
sun (0, 0, 0)
├── earth (1, 0, 0)
│   └── moon (0.1, 0, 0)
└── mars (1.5, 0, 0)
    └── phobos (0.05, 0, 0)
```

Each node has a name, a position *relative to its parent*, and children. Compute the world position of a node by summing positions up the parent chain.

Define the type:

```rust
use std::cell::RefCell;
use std::rc::Rc;

pub struct Node {
    pub name: String,
    pub local_pos: (f32, f32, f32),
    pub children: RefCell<Vec<Rc<Node>>>,
}
```

### Why `Rc<Node>` and not `Box<Node>`?

If each parent owned its children exclusively, `Box` would work. But we want flexibility: traversal code that returns an `Rc<Node>` (so the caller can hold it), auxiliary structures that reference nodes, and so on. Shared ownership makes all this easy; `Box` would force us to work only with borrows.

### Why `RefCell<Vec<...>>` and not just `Vec<...>`?

Because once a `Node` is inside an `Rc`, we only have `&Node` access — we can't mutate the children list normally. `RefCell` lets us. `add_child(parent, child)` takes `&Rc<Node>` (shared!) and adds a child — which requires mutating `children`.

### Constructor

```rust
impl Node {
    pub fn new(name: &str, local_pos: (f32, f32, f32)) -> Rc<Node> {
        Rc::new(Node {
            name: name.to_string(),
            local_pos,
            children: RefCell::new(Vec::new()),
        })
    }
}
```

Note we return `Rc<Node>`, not `Node`. Nodes are always shared; there's no point giving callers bare ownership they'd only immediately wrap in `Rc`.

## Step 3 — Adding children

```rust
pub fn add_child(parent: &Rc<Node>, child: Rc<Node>) {
    parent.children.borrow_mut().push(child);
}
```

Inputs:

- `&Rc<Node>` for the parent — we're *borrowing* the parent's Rc. We don't need to clone it; we just need to call a method through it.
- `Rc<Node>` (owned) for the child — the parent is taking a reference, so it takes ownership of a clone.

The caller site typically looks like:

```rust
add_child(&sun, Rc::clone(&earth));
```

We clone the child's Rc explicitly. The original `earth` variable is still usable.

### Why `Rc::clone(&x)` and not `x.clone()`?

They do the same thing. But convention is to write `Rc::clone(&x)` because:

1. It makes clear this is a cheap refcount bump, not a deep copy of the underlying data.
2. If `Node` someday derives `Clone` (a deep copy), `x.clone()` would silently switch to that behavior. `Rc::clone(&x)` unambiguously means the refcount bump.

Clippy will suggest you change `x.clone()` to `Rc::clone(&x)` when `x: Rc<_>`. Listen.

## Step 4 — Building a world

```rust
fn main() {
    let sun = Node::new("sun", (0.0, 0.0, 0.0));
    let earth = Node::new("earth", (1.0, 0.0, 0.0));
    let moon = Node::new("moon", (0.1, 0.0, 0.0));
    let mars = Node::new("mars", (1.5, 0.0, 0.0));
    let phobos = Node::new("phobos", (0.05, 0.0, 0.0));

    add_child(&earth, Rc::clone(&moon));
    add_child(&sun, Rc::clone(&earth));
    add_child(&mars, Rc::clone(&phobos));
    add_child(&sun, Rc::clone(&mars));

    println!("Scene rooted at: {}", sun.name);
    println!("Sun has {} children.", sun.children.borrow().len());
}
```

Run it:

```
Scene rooted at: sun
Sun has 2 children.
```

### What are the refcounts?

- `sun`: refcount 1 (only the `sun` variable).
- `earth`: refcount 2 (the `earth` variable + sun's children vec holds one).
- `moon`: refcount 2 (`moon` variable + earth's children vec).
- Same for mars and phobos.

When `main` ends, the local variables drop in reverse declaration order. `phobos` drops → refcount goes from 2 to 1 (still held by mars). `mars` drops → 2 to 1 (held by sun). Eventually `sun` drops → refcount goes to 0 → sun dropped → mars dropped (refcount 0 now that sun's children is gone) → phobos dropped. Clean teardown, no cycles, no leaks.

## Step 5 — DFS traversal

To walk the graph, we take an `&Rc<Node>` and recurse. The twist is that we want each visitor to see not just the current node, but also the path of ancestors (so it can compute world position).

```rust
pub fn visit<F>(node: &Rc<Node>, visitor: &F)
where
    F: Fn(&Rc<Node>, &[Rc<Node>]),
{
    fn walk<F: Fn(&Rc<Node>, &[Rc<Node>])>(
        node: &Rc<Node>,
        path: &mut Vec<Rc<Node>>,
        visitor: &F,
    ) {
        visitor(node, path);
        path.push(Rc::clone(node));
        for child in node.children.borrow().iter() {
            walk(child, path, visitor);
        }
        path.pop();
    }

    let mut path: Vec<Rc<Node>> = Vec::new();
    walk(node, &mut path, visitor);
}
```

### Breaking it down

- The outer `visit` sets up a mutable `path` vec and kicks off the recursion.
- `walk` is the real recursive function. It calls the visitor, pushes the current node onto `path`, recurses into children, pops `path` on the way out.
- The visitor gets `(&Rc<Node>, &[Rc<Node>])` — current node and slice of ancestors (not including self).

### Why the nested function?

`walk` needs `&mut Vec<Rc<Node>>` internally; the public API shouldn't expose that. Nested fn hides the implementation detail.

### Test it

```rust
fn main() {
    // ... set up scene as before ...

    visit(&sun, &|node, path| {
        let indent = "  ".repeat(path.len());
        println!("{}{} (local {:?})", indent, node.name, node.local_pos);
    });
}
```

Output:

```
sun (local (0.0, 0.0, 0.0))
  earth (local (1.0, 0.0, 0.0))
    moon (local (0.1, 0.0, 0.0))
  mars (local (1.5, 0.0, 0.0))
    phobos (local (0.05, 0.0, 0.0))
```

## Step 6 — World position

Now the visitor can compute world position by summing the path:

```rust
pub fn world_position(node: &Rc<Node>, ancestors: &[Rc<Node>]) -> (f32, f32, f32) {
    let mut x = node.local_pos.0;
    let mut y = node.local_pos.1;
    let mut z = node.local_pos.2;
    for a in ancestors {
        x += a.local_pos.0;
        y += a.local_pos.1;
        z += a.local_pos.2;
    }
    (x, y, z)
}
```

Or more idiomatically with an iterator:

```rust
pub fn world_position(node: &Rc<Node>, ancestors: &[Rc<Node>]) -> (f32, f32, f32) {
    ancestors.iter().chain(std::iter::once(node)).fold(
        (0.0, 0.0, 0.0),
        |(ax, ay, az), n| {
            (ax + n.local_pos.0, ay + n.local_pos.1, az + n.local_pos.2)
        },
    )
}
```

Use it in the visitor:

```rust
visit(&sun, &|node, path| {
    let wp = world_position(node, path);
    let indent = "  ".repeat(path.len());
    println!(
        "{}{}  local {:.2?}  world {:.2?}",
        indent, node.name, node.local_pos, wp
    );
});
```

Output:

```
sun  local (0.00, 0.00, 0.00)  world (0.00, 0.00, 0.00)
  earth  local (1.00, 0.00, 0.00)  world (1.00, 0.00, 0.00)
    moon  local (0.10, 0.00, 0.00)  world (1.10, 0.00, 0.00)
  mars  local (1.50, 0.00, 0.00)  world (1.50, 0.00, 0.00)
    phobos  local (0.05, 0.00, 0.00)  world (1.55, 0.00, 0.00)
```

Moon is at world x = 1.1 — parent (earth at 1.0) + its local 0.1. Phobos similarly.

## Step 7 — Finding a node

```rust
pub fn find(root: &Rc<Node>, name: &str) -> Option<Rc<Node>> {
    if root.name == name {
        return Some(Rc::clone(root));
    }
    for child in root.children.borrow().iter() {
        if let Some(found) = find(child, name) {
            return Some(found);
        }
    }
    None
}
```

Use:

```rust
if let Some(n) = find(&sun, "moon") {
    println!("Found {} at local {:?}", n.name, n.local_pos);
}
```

Notice `find` returns an owned `Rc<Node>` (not a reference). That's the convenience of `Rc` — you hand out a reference-counted handle, callers hold it as long as they like.

### The hidden RefCell hazard

Look at the recursive call:

```rust
for child in root.children.borrow().iter() {
    if let Some(found) = find(child, name) {
        return Some(found);
    }
}
```

`root.children.borrow()` gives a `Ref<Vec<Rc<Node>>>`. `.iter()` creates an iterator borrowing from it. As long as we're inside the `for` loop, the borrow is active.

If `find` internally tried to `borrow_mut()` on *this same* `root.children`, it would panic. Our recursive call descends into a *child's* children, not the parent's, so we're fine. But this is the kind of subtle trap that makes `RefCell` code scary in real projects.

## Step 8 — Cycles and why we need `Weak`

Imagine adding parent pointers. Every node needs to know its parent. Naively:

```rust
pub struct NodeWithParent {
    pub name: String,
    pub parent: Option<Rc<NodeWithParent>>,   // uh oh
    pub children: RefCell<Vec<Rc<NodeWithParent>>>,
}
```

Parent holds children via `Rc`. Children hold parent via `Rc`. This is a **reference cycle** — refcounts never reach zero, memory leaks. Rust won't stop you from creating this; it's memory-safe (no UB) but leaky.

The fix: `Weak<T>` for the back-edge.

```rust
use std::rc::Weak;

pub struct NodeWithParent {
    pub name: String,
    pub parent: RefCell<Option<Weak<NodeWithParent>>>,
    pub children: RefCell<Vec<Rc<NodeWithParent>>>,
}

impl NodeWithParent {
    pub fn new(name: &str) -> Rc<NodeWithParent> {
        Rc::new(NodeWithParent {
            name: name.to_string(),
            parent: RefCell::new(None),
            children: RefCell::new(Vec::new()),
        })
    }
}

pub fn add_child_with_parent(parent: &Rc<NodeWithParent>, child: Rc<NodeWithParent>) {
    *child.parent.borrow_mut() = Some(Rc::downgrade(parent));
    parent.children.borrow_mut().push(child);
}
```

`Rc::downgrade(&rc)` produces a `Weak<T>` — a reference that doesn't count. Cycle broken.

### Using the parent pointer

```rust
pub fn parent_of(node: &Rc<NodeWithParent>) -> Option<Rc<NodeWithParent>> {
    node.parent.borrow().as_ref()?.upgrade()
}
```

`upgrade()` returns `Option<Rc<T>>`:
- `Some(rc)` if the data is still alive.
- `None` if the last owning `Rc` has been dropped.

The `Option<Weak<_>>` is unwrapped with `?`, then `upgrade()` is called on the `Weak`. Returns `None` if either is missing.

## Common pitfalls

### Runtime `RefCell` panic

```rust
let a = cell.borrow_mut();
let b = cell.borrow();       // PANIC
```

The canonical `RefCell` pain. In real code it often happens because a callback or recursive function re-enters the same `RefCell`. Rules:

- Hold `Ref` / `RefMut` guards for the shortest possible scope.
- If you're calling into code that might touch the same cell, drop your guard first.
- Consider restructuring to avoid `RefCell` entirely.

### Reference cycles

If your data has "both directions" (parent ↔ child, twin lists pointing at each other), **one direction must be `Weak`**. Otherwise: memory leak.

### Cloning where you meant to borrow

```rust
fn describe(node: Rc<Node>) {   // takes ownership of an Rc (refcount bump)
    println!("{}", node.name);
}
describe(Rc::clone(&sun));       // unnecessary — just borrow!
```

Better:

```rust
fn describe(node: &Node) {       // just borrow
    println!("{}", node.name);
}
describe(&sun);
```

`Rc<T>` derefs to `T`, so `&sun` gives `&Node` (via `&Rc<Node>` → `&Node`). No refcount bump needed for a read-only view.

### `Rc` across threads

```rust
use std::thread;

let shared = Rc::new(42);
thread::spawn(move || {
    println!("{}", shared);       // ERROR: Rc is not Send
});
```

Fix: use `Arc` instead of `Rc`.

### `RefCell` across threads

Same deal: `RefCell` is not `Sync`. Use `Mutex<T>` or `RwLock<T>` on threaded code (Day 14).

## What you learned

- **`Box<T>`** for heap allocation and recursive types.
- **`Rc<T>`** for shared, single-threaded, immutable ownership.
- **`RefCell<T>`** for interior mutability, with runtime borrow checking.
- **`Rc<RefCell<T>>`** — the typical combination for shared mutable state.
- **`Weak<T>`** to break reference cycles without creating leaks.
- **`Arc<T>`** is the thread-safe cousin of `Rc` — same API, atomic refcounts.
- Scene graphs: tree of Rc-owned nodes, RefCell-wrapped children list, optional Weak parent.
- `Rc::clone(&x)` is idiomatic over `x.clone()`.

## Exercises

1. **Promote to `Arc`.** Change `Rc<Node>` to `Arc<Node>` and `RefCell` to `Mutex`. Wrap everything so a separate thread can traverse the graph without crashing (Day 14 covers threading properly, but you can experiment).
2. **Parent pointers.** Finish the `NodeWithParent` sketch. Write `world_position_via_parents(node)` that walks *up* the tree summing positions, upgrading `Weak` to `Rc` at each step.
3. **Detach and move nodes.** Implement `detach(child: &Rc<Node>)` that removes a child from its parent's list, and `reparent(new_parent: &Rc<Node>, child: &Rc<Node>)` that detaches then adds. Watch borrow lifetimes carefully.
4. **Graph, not tree.** What happens if you `add_child` the same node to two different parents? (Short answer: it works — refcount is 2 now. But traversal visits it twice.) Add a `visited: HashSet<*const Node>` to `visit` to deduplicate — using raw pointer equality as identity.
5. **No `RefCell`.** Rewrite the scene graph to avoid `RefCell` entirely by building the tree immutably — bottom up, children first, then a parent that owns them via `Vec<Rc<Node>>` (not `RefCell<Vec<...>>`). You give up after-the-fact mutation, but you get compile-time safety back.

## What's next

Day 11 pivots to a totally different domain: **terminal rendering**. You'll learn the `crossterm` crate, raw mode, cursor positioning, and input polling. This is all prep for Day 12, when you build Snake — a real-time game with a fixed-timestep loop and keyboard controls.

→ [Day 11 — Terminal rendering](day-11.md)
