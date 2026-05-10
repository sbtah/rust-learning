# Day 18 — A B-tree From Scratch

**Domain:** databases • **Time:** 2 hours • **Difficulty:** hard

## What you'll build

A B-tree implementation in pure safe Rust, generic over key and value types. Insert, get, iterate in order, and range queries. You'll test it against `std::collections::BTreeMap` with property-based tests — randomly generated operations — to prove correctness. This is the data structure at the heart of every relational database's index.

## What you'll learn

- **B-tree** structure, splits, and why databases prefer them over binary search trees
- Generics with **multiple trait bounds** (`K: Ord + Clone`)
- **Owned recursive data** structures (`Vec<Box<Node<K, V>>>`)
- Iterators over tree structures — **explicit stack** traversal
- Property-based testing with **proptest**
- When custom data structures are worth it vs reaching for `std`

## Background

### Why not a binary search tree?

A standard BST (balanced or not) has one key per node and two children. For an index with 1 million keys, that's 1 million nodes. Each node lookup involves a pointer chase, which may miss cache or even hit disk. Twenty cache misses just to look up a key.

**B-trees** pack many keys into each node — typically 50 to 500. A B-tree of 1 million keys has maybe 10,000 nodes total and is only 3-4 levels deep. Fewer pointer chases means fewer cache misses and faster lookups.

On disk, the math is even starker. Each node is a page (4 KiB), reading a page is one seek (~100 µs on HDDs, ~10 µs on SSDs). A B-tree with branching factor 200 and 1 billion keys is only 4 levels deep — 4 seeks for a lookup. Binary search tree? 30 seeks.

B-trees power MySQL's InnoDB, PostgreSQL's indexes, SQLite, and about every other relational store. They're the default.

### B-tree mechanics

A B-tree of order `m`:

- Every node has at most `m - 1` keys.
- Every node except the root has at least `ceil(m/2) - 1` keys.
- Every internal node with `k` keys has exactly `k + 1` children.
- Keys in a node are sorted.
- For internal node with keys `k1 < k2 < ... < kn`, children `c0, c1, ..., cn`:
  - `c0` contains keys `< k1`
  - `c1` contains keys `> k1` and `< k2`
  - ...
  - `cn` contains keys `> kn`
- All leaves are at the same depth (the tree is balanced).

Insertion:

1. Find the leaf where the key belongs (descend the tree).
2. If the leaf has room (< `m - 1` keys), insert and done.
3. If the leaf is full, **split**: take the median key, promote it to the parent, create two sibling leaves with the remaining keys.
4. If the parent becomes full, split it too. Cascade up.
5. If the root splits, create a new root. The tree grows by one level.

We'll implement a simplified B-tree (no deletion for this exercise — deletion in B-trees is notoriously fiddly). Insertion + lookup + iteration is enough to match the performance story.

### Generics with multiple bounds

Our tree needs keys that can be compared. `Ord` is the trait for total ordering. We also need to clone keys and values when splitting (a key being promoted to a parent appears in the parent, and we'd otherwise have to move ownership awkwardly).

```rust
pub struct BTree<K: Ord + Clone, V: Clone> {
    ...
}
```

Any `K` that implements both `Ord` and `Clone` works. Strings, integers, tuples of integers — all fine.

### Owned recursive data

Rust's default ownership model (stack-like) can't represent `struct Node { children: Vec<Node> }` — it'd be infinite-size. We need indirection:

```rust
struct Node<K, V> {
    keys: Vec<K>,
    values: Vec<V>,
    children: Vec<Box<Node<K, V>>>,  // empty for leaves
}
```

`Box<Node<K, V>>` is a heap-allocated pointer. Size is a known constant (one pointer). Now `Node` has a known size.

`Vec<Box<Node>>` has some overhead — two allocations per child (one for the Vec element, one for the Box). A production B-tree would use one `Vec<Node>` with indices instead of pointers, saving both allocations and a dereference. We'll use the clean version and note the optimization.

## Setting up

Continue in `rkvs`:

```sh
cargo add proptest --dev
```

`--dev` adds it to `[dev-dependencies]` — only compiled for tests, not shipped with the binary.

## Step 1 — The Node struct

Create `src/btree.rs`:

```rust
const ORDER: usize = 6;  // max keys per node; small so we actually split during testing
const MAX_KEYS: usize = ORDER - 1;

#[derive(Debug)]
struct Node<K: Ord + Clone, V: Clone> {
    keys: Vec<K>,
    values: Vec<V>,
    children: Vec<Box<Node<K, V>>>,  // empty for leaf, else has keys.len()+1 entries
}

impl<K: Ord + Clone, V: Clone> Node<K, V> {
    fn new_leaf() -> Self {
        Node {
            keys: Vec::new(),
            values: Vec::new(),
            children: Vec::new(),
        }
    }

    fn is_leaf(&self) -> bool {
        self.children.is_empty()
    }

    fn is_full(&self) -> bool {
        self.keys.len() >= MAX_KEYS
    }
}
```

`ORDER = 6` is small — it means leaves split once they have 5 keys and get a 6th. A production tree would pick 100-500 so each node fills a cache line or disk page. Keeping it small makes splits happen frequently during testing, exercising the interesting code.

We store keys and values as parallel `Vec`s rather than `Vec<(K, V)>`. It's a minor performance optimization (keys pack densely) and matches how production B-trees lay out memory.

## Step 2 — Get (the easy part)

```rust
impl<K: Ord + Clone, V: Clone> Node<K, V> {
    fn get(&self, key: &K) -> Option<&V> {
        // Binary search within the node
        match self.keys.binary_search(key) {
            Ok(i) => Some(&self.values[i]),
            Err(i) => {
                if self.is_leaf() {
                    None
                } else {
                    self.children[i].get(key)
                }
            }
        }
    }
}
```

`binary_search` on a sorted slice returns `Ok(index)` if found, `Err(insertion_index)` if not. For us:

- `Ok(i)` — the key is at position `i` in this node. Return the value.
- `Err(i)` — the key would go at position `i`. If we're a leaf, the key isn't in the tree. Otherwise, descend into `children[i]` — that's the subtree containing keys in the right range.

Recursive descent. The tree height is `O(log_m N)` so this is fast.

## Step 3 — Insert with splits

Insertion is the hard part. Our strategy:

1. `insert` on a node. It recurses down, possibly splits, and returns any split result to its parent.
2. The split result is `(promoted_key, promoted_value, right_sibling_node)`.
3. If the root receives a split result, we build a new root.

```rust
struct Split<K, V> {
    key: K,
    value: V,
    right: Box<Node<K, V>>,
}

impl<K: Ord + Clone, V: Clone> Node<K, V> {
    /// Returns Some(Split) if this node split during insertion.
    fn insert(&mut self, key: K, value: V) -> Option<Split<K, V>> {
        if self.is_leaf() {
            self.insert_into_leaf(key, value);
        } else {
            match self.keys.binary_search(&key) {
                Ok(i) => {
                    self.values[i] = value;
                    return None;
                }
                Err(i) => {
                    if let Some(split) = self.children[i].insert(key, value) {
                        // Child split — absorb the promoted key
                        self.keys.insert(i, split.key);
                        self.values.insert(i, split.value);
                        self.children.insert(i + 1, split.right);
                    }
                }
            }
        }

        if self.is_full() {
            Some(self.split())
        } else {
            None
        }
    }

    fn insert_into_leaf(&mut self, key: K, value: V) {
        match self.keys.binary_search(&key) {
            Ok(i) => self.values[i] = value,
            Err(i) => {
                self.keys.insert(i, key);
                self.values.insert(i, value);
            }
        }
    }
}
```

The recursive structure is subtle — let's walk it:

- **Leaf case**: find insertion point, insert or overwrite.
- **Internal case**: if the key is already in this node, overwrite. Otherwise, descend into the appropriate child.
  - If the child splits (returns `Some(split)`), we get a new key + value to insert at position `i`, plus a new right sibling to attach at position `i + 1`.
- After inserting (leaf or child-promote), check if we're full. If so, split ourselves and return the promoted data.

### The split function

```rust
impl<K: Ord + Clone, V: Clone> Node<K, V> {
    fn split(&mut self) -> Split<K, V> {
        let mid = self.keys.len() / 2;

        // Take the median
        let median_key = self.keys.remove(mid);
        let median_value = self.values.remove(mid);

        // Everything after the median goes to a new right node
        let right_keys = self.keys.split_off(mid);
        let right_values = self.values.split_off(mid);

        let right_children = if self.is_leaf() {
            Vec::new()
        } else {
            self.children.split_off(mid + 1)
        };

        let right = Node {
            keys: right_keys,
            values: right_values,
            children: right_children,
        };

        Split {
            key: median_key,
            value: median_value,
            right: Box::new(right),
        }
    }
}
```

Step by step: take out the median key-value pair, split the remaining keys and values at the median point into self (left) and right. For internal nodes, also split children. Package the median as the promoted key-value, with the new right sibling.

`Vec::split_off(i)` returns a new vec containing elements from `i` onwards, leaving self with `0..i`. Efficient — no individual moves, just a pointer adjustment.

### The BTree wrapper

```rust
pub struct BTree<K: Ord + Clone, V: Clone> {
    root: Box<Node<K, V>>,
    len: usize,
}

impl<K: Ord + Clone, V: Clone> BTree<K, V> {
    pub fn new() -> Self {
        BTree {
            root: Box::new(Node::new_leaf()),
            len: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn get(&self, key: &K) -> Option<&V> {
        self.root.get(key)
    }

    pub fn insert(&mut self, key: K, value: V) {
        let was_present = self.get(&key).is_some();

        if let Some(split) = self.root.insert(key, value) {
            // Root split — build a new root
            let old_root = std::mem::replace(&mut self.root, Box::new(Node::new_leaf()));
            self.root.keys.push(split.key);
            self.root.values.push(split.value);
            self.root.children.push(old_root);
            self.root.children.push(split.right);
        }

        if !was_present {
            self.len += 1;
        }
    }
}

impl<K: Ord + Clone, V: Clone> Default for BTree<K, V> {
    fn default() -> Self {
        Self::new()
    }
}
```

The magic is in the root-split case. When `root.insert` returns `Some(split)`, we:

1. Replace the root with a new empty leaf (temporarily).
2. Push the promoted key/value into the new root.
3. Set the new root's children to `[old_root, split.right]`.

`std::mem::replace` is how we swap ownership without `Clone` — we met it on Day 2 for player status transitions.

`was_present` is checked *before* `insert` runs. If we checked after, we'd be counting keys that were just overwritten.

## Step 4 — A sanity test

Register and basic test:

```rust
// src/main.rs
mod btree;
```

```rust
// in src/btree.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_and_get() {
        let mut tree: BTree<i32, String> = BTree::new();
        assert!(tree.is_empty());

        tree.insert(10, "ten".into());
        tree.insert(20, "twenty".into());
        tree.insert(5, "five".into());

        assert_eq!(tree.len(), 3);
        assert_eq!(tree.get(&10), Some(&"ten".to_string()));
        assert_eq!(tree.get(&5), Some(&"five".to_string()));
        assert_eq!(tree.get(&20), Some(&"twenty".to_string()));
        assert_eq!(tree.get(&99), None);
    }

    #[test]
    fn overwrite() {
        let mut tree: BTree<i32, i32> = BTree::new();
        tree.insert(1, 100);
        tree.insert(1, 200);
        assert_eq!(tree.len(), 1);
        assert_eq!(tree.get(&1), Some(&200));
    }

    #[test]
    fn many_insertions_trigger_splits() {
        let mut tree: BTree<i32, i32> = BTree::new();
        for i in 0..100 {
            tree.insert(i, i * 10);
        }
        assert_eq!(tree.len(), 100);
        for i in 0..100 {
            assert_eq!(tree.get(&i), Some(&(i * 10)));
        }
    }
}
```

Run:

```sh
cargo test btree::tests
```

Three tests should pass. The `many_insertions_trigger_splits` case is key — with `ORDER = 6`, after 5 insertions you get the first leaf split; by 100 insertions the tree is several levels deep, exercising internal node splits too.

## Step 5 — Ordered iteration

B-trees beat hash tables at range queries because they store keys in order. We need an iterator.

```rust
impl<K: Ord + Clone, V: Clone> Node<K, V> {
    fn collect_into(&self, out: &mut Vec<(K, V)>) {
        if self.is_leaf() {
            for (k, v) in self.keys.iter().zip(self.values.iter()) {
                out.push((k.clone(), v.clone()));
            }
        } else {
            // Interleave: c0, k0, c1, k1, ..., cn
            for i in 0..self.keys.len() {
                self.children[i].collect_into(out);
                out.push((self.keys[i].clone(), self.values[i].clone()));
            }
            self.children.last().unwrap().collect_into(out);
        }
    }
}

impl<K: Ord + Clone, V: Clone> BTree<K, V> {
    pub fn entries(&self) -> Vec<(K, V)> {
        let mut out = Vec::with_capacity(self.len);
        self.root.collect_into(&mut out);
        out
    }
}
```

The interleave pattern is the heart of in-order traversal: for an internal node with `n` keys and `n+1` children, visit `child[0]`, then `key[0]`, then `child[1]`, then `key[1]`, ..., finally `child[n]`. Every key appears exactly once, in sorted order.

For production code we'd want a lazy `Iterator` implementation, not a full `Vec` allocation. The exercise covers that.

Add a test:

```rust
    #[test]
    fn entries_in_order() {
        let mut tree: BTree<i32, i32> = BTree::new();
        let input = [5, 2, 9, 1, 7, 3, 8, 4, 6];
        for &i in &input {
            tree.insert(i, i * 10);
        }

        let entries = tree.entries();
        let keys: Vec<i32> = entries.iter().map(|(k, _)| *k).collect();
        assert_eq!(keys, vec![1, 2, 3, 4, 5, 6, 7, 8, 9]);
    }
```

Insert in scrambled order, iterate in sorted order. This is what B-trees do and hash tables can't.

## Step 6 — Range queries

```rust
impl<K: Ord + Clone, V: Clone> Node<K, V> {
    fn collect_range_into(&self, lo: &K, hi: &K, out: &mut Vec<(K, V)>) {
        if self.is_leaf() {
            for (k, v) in self.keys.iter().zip(self.values.iter()) {
                if k >= lo && k < hi {
                    out.push((k.clone(), v.clone()));
                }
            }
        } else {
            // Find which children overlap the range
            let start = self.keys.partition_point(|k| k < lo);
            let end = self.keys.partition_point(|k| k < hi);

            for i in start..=end {
                self.children[i].collect_range_into(lo, hi, out);
                if i < self.keys.len() && &self.keys[i] >= lo && &self.keys[i] < hi {
                    out.push((self.keys[i].clone(), self.values[i].clone()));
                }
            }
        }
    }
}

impl<K: Ord + Clone, V: Clone> BTree<K, V> {
    pub fn range(&self, lo: &K, hi: &K) -> Vec<(K, V)> {
        let mut out = Vec::new();
        self.root.collect_range_into(lo, hi, &mut out);
        out
    }
}
```

`partition_point` is the underused cousin of `binary_search`: it returns the index where the predicate goes from `true` to `false`. Here, we find which children we need to descend into — any child whose subtree overlaps `[lo, hi)`.

The range is half-open: includes `lo`, excludes `hi`. Matches Rust's convention (`..` in slice ranges).

Test:

```rust
    #[test]
    fn range_query() {
        let mut tree: BTree<i32, i32> = BTree::new();
        for i in 0..20 {
            tree.insert(i, i);
        }

        let got = tree.range(&5, &10);
        let keys: Vec<i32> = got.iter().map(|(k, _)| *k).collect();
        assert_eq!(keys, vec![5, 6, 7, 8, 9]);
    }
```

## Step 7 — Property-based testing against `BTreeMap`

Now the real test. `std::collections::BTreeMap` is a production B-tree — if our tree agrees with it on random inputs, ours is probably correct.

Add to `src/btree.rs`:

```rust
#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;
    use std::collections::BTreeMap;

    proptest! {
        #[test]
        fn matches_std_btreemap(ops in prop::collection::vec(any::<(i32, i32)>(), 0..500)) {
            let mut ours: BTree<i32, i32> = BTree::new();
            let mut theirs: BTreeMap<i32, i32> = BTreeMap::new();

            for (k, v) in ops {
                ours.insert(k, v);
                theirs.insert(k, v);
            }

            prop_assert_eq!(ours.len(), theirs.len());

            // Random keys: get should agree
            for k in -100..100 {
                prop_assert_eq!(ours.get(&k), theirs.get(&k));
            }

            // Ordered iteration should agree
            let our_entries: Vec<_> = ours.entries();
            let their_entries: Vec<_> = theirs.iter().map(|(k, v)| (*k, *v)).collect();
            prop_assert_eq!(our_entries, their_entries);
        }

        #[test]
        fn range_matches_std(
            ops in prop::collection::vec(any::<(i32, i32)>(), 0..200),
            lo in any::<i32>(),
            hi in any::<i32>(),
        ) {
            let (lo, hi) = if lo > hi { (hi, lo) } else { (lo, hi) };
            let mut ours: BTree<i32, i32> = BTree::new();
            let mut theirs: BTreeMap<i32, i32> = BTreeMap::new();

            for (k, v) in ops {
                ours.insert(k, v);
                theirs.insert(k, v);
            }

            let our_range = ours.range(&lo, &hi);
            let their_range: Vec<_> = theirs.range(lo..hi).map(|(k, v)| (*k, *v)).collect();

            prop_assert_eq!(our_range, their_range);
        }
    }
}
```

`proptest!` generates random inputs. For each test case, both trees perform the same operations; we assert their states agree.

`prop::collection::vec(any::<(i32, i32)>(), 0..500)` — generate a vec of up to 500 `(i32, i32)` tuples, each drawn from the entire i32 range. Many tuples will have duplicate keys, testing overwrite. Some keys will be `i32::MIN`, `i32::MAX`, or other edge cases that a human wouldn't think to test.

Run:

```sh
cargo test btree::proptests
```

Expected: two tests pass, each running 256 random cases by default. If something's wrong, proptest will automatically **shrink** to a minimal failing case — for example, "inserting these 3 specific values in this order produces wrong output."

If you broke the split logic (try removing the `mid + 1` in `children.split_off(mid + 1)` and running), proptest won't just fail — it'll find the simplest case that fails and print it. This is orders of magnitude more valuable than writing tests by hand.

## Step 8 — Measure against `BTreeMap`

```rust
#[cfg(test)]
#[test]
#[ignore]
fn bench_vs_std() {
    use std::time::Instant;
    const N: usize = 100_000;

    let start = Instant::now();
    let mut ours: BTree<i32, i32> = BTree::new();
    for i in 0..N as i32 {
        ours.insert(i, i * 2);
    }
    let our_insert = start.elapsed();

    let start = Instant::now();
    let mut theirs: std::collections::BTreeMap<i32, i32> = std::collections::BTreeMap::new();
    for i in 0..N as i32 {
        theirs.insert(i, i * 2);
    }
    let their_insert = start.elapsed();

    println!("insert {}: ours {:?}, std {:?}", N, our_insert, their_insert);
}
```

Run:

```sh
cargo test --release bench_vs_std -- --ignored --nocapture
```

You'll probably see `std::BTreeMap` is 3-10x faster. That's expected — `std::BTreeMap` uses tuned layouts, unsafe code, and the `B = 6` constant from real-world measurements. Our purpose today isn't to beat std; it's to understand the algorithm.

## Common pitfalls

### Off-by-one in `split`

```rust
let right_children = self.children.split_off(mid + 1);  // CORRECT
let right_children = self.children.split_off(mid);      // WRONG
```

For an internal node, `children.len() == keys.len() + 1`. After removing the median key at position `mid`, the children split point is still `mid + 1` — because child `mid` is the left-of-median child (stays with self) and child `mid + 1` onwards go right.

Test failure from this kind of bug looks like: "inserted 20 keys, `get(15)` returns `None`." Proptest shrinks it to a minimal repro.

### Recursion depth

For `ORDER = 6` and 100k keys, tree depth is ~7. Recursion is fine. With order=2 (a plain binary tree), depth would be 17 — still fine, but worth being aware of the stack size. Rust defaults to 8 MB of stack which is plenty for trees of reasonable branching.

### Forgetting to clone keys

```rust
out.push((self.keys[i], self.values[i]));  // would move out of borrowed vec: error
out.push((self.keys[i].clone(), self.values[i].clone()));  // correct
```

You can't move out of a shared reference. Since we return owned `(K, V)`, we must clone. A reference-returning iterator (`Iterator<Item=(&K, &V)>`) would avoid this — it's what `std::BTreeMap::iter` does. Exercise territory.

### `Ord` without consistent `PartialEq`

If you implement `Ord` manually and the comparison disagrees with `PartialEq`, `binary_search` will produce garbage results. Always derive both together, or be extremely careful. For most types, `#[derive(PartialEq, Eq, PartialOrd, Ord)]` is the right move.

### Deletion

We didn't implement it. Deletion in a B-tree can require **merging** nodes (when a key is removed from a node with the minimum count, the node must borrow from or merge with a sibling). It's doable but doubles the code. Production databases often handle deletions by marking tombstones and compacting in the background — which is what Bitcask did yesterday.

## What you learned

- **B-tree** structure: wide, shallow, sorted. Cache- and disk-friendly.
- **Split on overflow**: promote median, grow up. Root splits grow the tree by a level.
- **`Vec<Box<Node>>`** for recursive data structures. Known size via boxing.
- **Parallel `Vec<K>` + `Vec<V>`** — minor optimization, standard pattern.
- **Ordered iteration** via interleaved children-and-keys traversal.
- **Range queries** via `partition_point` to find relevant subtrees.
- **Property-based testing** via proptest finds bugs humans wouldn't write tests for.
- `std::BTreeMap` exists and is faster — but now you know what's inside it.

## Exercises

1. **Iterator without allocations.** Replace `entries(&self) -> Vec<(K, V)>` with `fn iter(&self) -> impl Iterator<Item = (&K, &V)>`. You'll need to maintain an explicit stack of `(node, index)` pairs. Compare performance against the `Vec` version for a range scan over 1M entries.
2. **Deletion.** Implement `fn remove(&mut self, key: &K) -> Option<V>`. Handle both leaf and internal removal (replacing with in-order predecessor or successor). Handle underflow (merge or borrow from sibling). Add proptest cases covering mixed insert/remove workloads.
3. **Bulk loading.** Implement `BTree::from_sorted(items: Vec<(K, V)>) -> BTree<K, V>` that builds the tree bottom-up from a pre-sorted sequence. This is much faster than inserting one by one for large data sets. Benchmark.
4. **Order as a const generic.** Change the hardcoded `ORDER` to `const N: usize` as a struct-level generic: `BTree<K, V, const N: usize = 16>`. Benchmark with `N = 4, 16, 64, 256` for 100k insertions. What's the sweet spot on your CPU?
5. **Serializable tree.** Add `#[derive(Serialize, Deserialize)]` to the tree so you can bincode it to disk. How large is a 10k-entry tree on disk vs. just bincode-serializing `Vec<(K, V)>`? What about after compression (`flate2` crate)?

## What's next

Day 19 introduces **memory-mapped files** — the dark magic behind fast read paths. Instead of `seek` + `read`, you map the entire file into process memory and read bytes directly. We'll introduce Rust's `unsafe` keyword, with exactly one carefully-commented unsafe block for the mmap syscall, and benchmark mmap reads vs. the seek-based path from Day 17.

→ [Day 19 — Memory-Mapped Files and `unsafe`](day-19.md)
