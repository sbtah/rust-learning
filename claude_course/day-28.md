# Day 28 — BVH Acceleration

**Domain:** 3D graphics • **Time:** 2.5 hours • **Difficulty:** hard

## What you'll build

A bounding volume hierarchy. Right now each ray tests against every sphere in the scene — O(rays × shapes). Today you'll replace that linear scan with a binary tree where each node carries an **axis-aligned bounding box** (AABB) that encloses everything beneath it. A ray tests the box first; if it misses, an entire subtree is skipped. The result is O(log n) per ray on nicely-distributed scenes. On a 500-sphere scene, expect ~40× speedup on top of yesterday's parallelism, often pushing renders from tens of seconds down to under a second.

## What you'll learn

- Axis-aligned bounding boxes and why they're the workhorse of ray tracing
- **The slab method**: ray-AABB intersection via coordinate-wise min/max
- Extending the `Hittable` trait with a `bounding_box()` method
- Recursive tree building: pick a split axis, partition children, recurse
- **SAH preview** vs. simple midpoint splits, and the practical tradeoff
- Benchmarking alternative implementations of the same interface
- Why BVH + rayon compose cleanly (pure reads, no shared state)

## Background

### The O(n) problem

Our current `HittableList::hit` tests every shape against the ray:

```rust
impl Hittable for HittableList {
    fn hit(&self, ray: &Ray, t_min: f32, t_max: f32) -> Option<HitRecord> {
        let mut closest = t_max;
        let mut best = None;
        for object in &self.objects {
            if let Some(rec) = object.hit(ray, t_min, closest) {
                closest = rec.t;
                best = Some(rec);
            }
        }
        best
    }
}
```

For 5 spheres, that's fine. For 500 spheres it's 500 tests per ray. For a 4K image at 500 samples with depth 50, the inner loop runs literally trillions of times. You need to eliminate most of those tests.

### Bounding volumes

The insight: replace "test against every shape" with "test against one cheap enclosing box." If the ray misses the box, none of the shapes inside can possibly be hit.

An **axis-aligned bounding box** is just two `Vec3`s: `min` and `max`. A point is inside iff for each axis `i`, `min[i] <= p[i] <= max[i]`. Easy to compute, easy to test, and **cheap to combine** — the union of two AABBs is just componentwise min and max.

AABBs aren't the tightest possible bounds (oriented bounding boxes or k-DOPs are tighter), but they're dramatically cheaper to intersect-test. The hierarchy compensates for the looser bound.

### The slab method

Think of an AABB as the intersection of three "slabs" — infinite regions between two parallel planes, one slab per axis. For each slab, compute the two parameter values `t_near` and `t_far` where the ray enters and exits. The ray hits the box iff the slab intersections have a common parameter range — i.e., `max(t_near) <= min(t_far)` across all three axes.

For a single axis `x`:

```
t0 = (min.x - origin.x) / direction.x
t1 = (max.x - origin.x) / direction.x
if direction.x < 0.0 { swap(t0, t1) }
```

`t0` is where the ray enters the slab, `t1` is where it exits. Compute per axis, take the running max of `t0`s and running min of `t1`s. If the final interval `[t0_max, t1_min]` is empty or entirely outside `[t_min, t_max]`, the ray misses the box.

One subtle point: division by zero. If `direction.x == 0.0`, the ray is parallel to the slab. IEEE-754 gives you `+inf` or `-inf`, and the comparisons work out correctly — if the ray origin is inside the slab, `[t0, t1] = [-inf, +inf]`; if outside, `[t0, t1] = [inf, inf]` or similar, which correctly indicates no intersection. No special-casing needed.

### Why a tree

One AABB around everything isn't useful — if the ray hits it, you still test every shape. The idea is to **recursively partition** shapes into a binary tree. At each node, you have an AABB enclosing all shapes below it. Traversal:

```
fn hit(node, ray):
    if ray misses node.aabb: return None
    if node is a leaf: test the shapes directly
    else: hit(node.left, ray) or hit(node.right, ray)
```

If the tree is well-balanced and the AABBs are tight, you skip huge portions of the scene. In the best case, each ray visits O(log n) leaves.

Trees are memory-flexible in Rust. We'll use `Box<dyn Hittable>` for children so a leaf can hold a `Sphere` and an internal node can hold two more `BvhNode`s — same trait, different layouts, no sum types needed.

### Picking a split axis

When you partition N shapes into two groups, how do you choose which go left and which go right? The quality of that choice dominates BVH performance.

Today's approach — cheap and good enough — is **midpoint split on the longest axis**:

1. Find the AABB of all shapes in this group.
2. Pick the axis with the largest extent (x, y, or z).
3. Sort the shapes by their bounding-box center along that axis.
4. Split at the middle index.

This is O(n log n) per split, O(n log² n) total. Not optimal. The gold standard is **Surface Area Heuristic (SAH)** — score every candidate split by the surface area of each child and the count of shapes, and pick the minimum. SAH makes traversal ~2× faster than midpoint. We'll implement midpoint today; the exercises point at SAH.

### What changes in the Hittable trait

`Hittable` needs one new method: `bounding_box()`. Any shape that wants to go into a BVH must know its own AABB. `Sphere`'s box is `(center - r, center + r)`. A plane has no finite box, so planes can't go into a BVH — they'd live in a separate list of "infinite primitives" that every ray always tests. Today all our shapes are spheres, so we'll skip that complication.

## Setting up

You're extending yesterday's raytracer; no new dependencies.

```bash
cd raytracer
```

## Step 1 — The AABB type

Create `src/aabb.rs`:

```rust
use crate::ray::Ray;
use crate::vec3::Vec3;

#[derive(Debug, Clone, Copy)]
pub struct Aabb {
    pub min: Vec3,
    pub max: Vec3,
}

impl Aabb {
    pub fn new(min: Vec3, max: Vec3) -> Self {
        Self { min, max }
    }

    /// The smallest AABB containing both.
    pub fn surrounding(a: Aabb, b: Aabb) -> Self {
        let min = Vec3::new(
            a.min.x.min(b.min.x),
            a.min.y.min(b.min.y),
            a.min.z.min(b.min.z),
        );
        let max = Vec3::new(
            a.max.x.max(b.max.x),
            a.max.y.max(b.max.y),
            a.max.z.max(b.max.z),
        );
        Self { min, max }
    }

    /// Which axis (0=x, 1=y, 2=z) has the largest extent?
    pub fn longest_axis(&self) -> usize {
        let ext = self.max - self.min;
        if ext.x > ext.y && ext.x > ext.z { 0 }
        else if ext.y > ext.z { 1 }
        else { 2 }
    }

    /// Slab-method ray-AABB test. Returns true if the ray intersects this
    /// AABB within [t_min, t_max].
    pub fn hit(&self, ray: &Ray, mut t_min: f32, mut t_max: f32) -> bool {
        for axis in 0..3 {
            let inv_d = 1.0 / ray.direction[axis];
            let mut t0 = (self.min[axis] - ray.origin[axis]) * inv_d;
            let mut t1 = (self.max[axis] - ray.origin[axis]) * inv_d;
            if inv_d < 0.0 {
                std::mem::swap(&mut t0, &mut t1);
            }
            t_min = t_min.max(t0);
            t_max = t_max.min(t1);
            if t_max <= t_min {
                return false;
            }
        }
        true
    }
}
```

This assumes `Vec3` has an `index` operator: `vec[0]` returns `vec.x`, etc. If you don't have one yet, add to `src/vec3.rs`:

```rust
use std::ops::Index;

impl Index<usize> for Vec3 {
    type Output = f32;
    fn index(&self, i: usize) -> &f32 {
        match i {
            0 => &self.x,
            1 => &self.y,
            2 => &self.z,
            _ => panic!("Vec3 index {i} out of bounds"),
        }
    }
}
```

Add the module to `src/lib.rs`:

```rust
pub mod aabb;
```

### A sanity test

Add at the bottom of `src/aabb.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ray_hits_box() {
        let b = Aabb::new(Vec3::new(-1.0, -1.0, -1.0), Vec3::new(1.0, 1.0, 1.0));
        let ray = Ray::new(Vec3::new(0.0, 0.0, -5.0), Vec3::new(0.0, 0.0, 1.0));
        assert!(b.hit(&ray, 0.001, 100.0));
    }

    #[test]
    fn ray_misses_box() {
        let b = Aabb::new(Vec3::new(-1.0, -1.0, -1.0), Vec3::new(1.0, 1.0, 1.0));
        let ray = Ray::new(Vec3::new(5.0, 0.0, -5.0), Vec3::new(0.0, 0.0, 1.0));
        assert!(!b.hit(&ray, 0.001, 100.0));
    }

    #[test]
    fn surrounding_contains_both() {
        let a = Aabb::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 1.0, 1.0));
        let b = Aabb::new(Vec3::new(-1.0, 2.0, 0.5), Vec3::new(0.5, 3.0, 2.0));
        let s = Aabb::surrounding(a, b);
        assert_eq!(s.min, Vec3::new(-1.0, 0.0, 0.0));
        assert_eq!(s.max, Vec3::new(1.0, 3.0, 2.0));
    }
}
```

Run:

```bash
cargo test aabb
```

Expected output: `3 passed`. The slab math is famously off-by-sign-flip-able — having tests in place before building the BVH is cheap insurance.

## Step 2 — Extend the Hittable trait

Open `src/hit.rs`. Add a required method:

```rust
pub trait Hittable: Send + Sync {
    fn hit(&self, ray: &Ray, t_min: f32, t_max: f32) -> Option<HitRecord>;
    fn bounding_box(&self) -> Aabb;
}
```

Import `Aabb` at the top:

```rust
use crate::aabb::Aabb;
```

The compiler will now yell at every impl of `Hittable`. Good — that's the point. Each shape needs to declare its bounds.

### Sphere

In `src/scene.rs` (or wherever `Sphere` lives):

```rust
use crate::aabb::Aabb;

impl Hittable for Sphere {
    // ... existing hit method ...

    fn bounding_box(&self) -> Aabb {
        let r = Vec3::new(self.radius.abs(), self.radius.abs(), self.radius.abs());
        Aabb::new(self.center - r, self.center + r)
    }
}
```

Use `.abs()` because hollow-glass spheres have negative radius (you set this up on Day 25 for the Dielectric trick).

### HittableList

```rust
impl Hittable for HittableList {
    // ... existing hit method ...

    fn bounding_box(&self) -> Aabb {
        assert!(!self.objects.is_empty(), "empty HittableList has no bounding box");
        let mut bbox = self.objects[0].bounding_box();
        for obj in &self.objects[1..] {
            bbox = Aabb::surrounding(bbox, obj.bounding_box());
        }
        bbox
    }
}
```

Now `cargo build` should succeed. Run your existing tests too — nothing else should regress.

## Step 3 — The BVH node

Create `src/bvh.rs`:

```rust
use crate::aabb::Aabb;
use crate::hit::{HitRecord, Hittable};
use crate::ray::Ray;

pub struct BvhNode {
    bbox: Aabb,
    left: Box<dyn Hittable>,
    right: Box<dyn Hittable>,
}

impl Hittable for BvhNode {
    fn hit(&self, ray: &Ray, t_min: f32, t_max: f32) -> Option<HitRecord> {
        if !self.bbox.hit(ray, t_min, t_max) {
            return None;
        }
        let left_hit = self.left.hit(ray, t_min, t_max);
        // Second ray intersects against the narrowed interval: if the left
        // subtree found a hit at t=12, no point checking the right subtree
        // beyond t=12 — any hit there would be farther.
        let upper = match &left_hit {
            Some(rec) => rec.t,
            None => t_max,
        };
        let right_hit = self.right.hit(ray, t_min, upper);
        right_hit.or(left_hit)
    }

    fn bounding_box(&self) -> Aabb {
        self.bbox
    }
}
```

The `upper` narrowing in `hit` is a small but critical optimization: once the left subtree reports a hit at some `t`, we can reject any right-subtree candidate beyond `t`. Without this, BVH hit time degrades to O(n) in adversarial cases.

Add the module:

```rust
// src/lib.rs
pub mod bvh;
```

## Step 4 — Building the tree

A BVH node needs a constructor that takes a list of shapes and recursively splits them. The tricky part: we need to own the shapes (`Box<dyn Hittable>`), sort them by axis, and hand halves to recursive calls. Add to `src/bvh.rs`:

```rust
impl BvhNode {
    pub fn build(mut objects: Vec<Box<dyn Hittable>>) -> Box<dyn Hittable> {
        match objects.len() {
            0 => panic!("BvhNode::build called with empty list"),
            1 => objects.pop().unwrap(),
            2 => {
                // Leaf pair — no point creating a trivial interior node with
                // a single child on one side, but two shapes fit neatly.
                let right = objects.pop().unwrap();
                let left = objects.pop().unwrap();
                let bbox = Aabb::surrounding(left.bounding_box(), right.bounding_box());
                Box::new(BvhNode { bbox, left, right })
            }
            _ => {
                // Figure out the axis to split on: longest extent of the
                // parent AABB.
                let parent_bbox = objects
                    .iter()
                    .map(|o| o.bounding_box())
                    .reduce(Aabb::surrounding)
                    .unwrap();
                let axis = parent_bbox.longest_axis();

                // Sort by AABB centroid along that axis.
                objects.sort_by(|a, b| {
                    let ca = a.bounding_box().min[axis] + a.bounding_box().max[axis];
                    let cb = b.bounding_box().min[axis] + b.bounding_box().max[axis];
                    ca.partial_cmp(&cb).unwrap()
                });

                // Split in half and recurse.
                let mid = objects.len() / 2;
                let right_half = objects.split_off(mid);
                let left = Self::build(objects);
                let right = Self::build(right_half);
                let bbox = Aabb::surrounding(left.bounding_box(), right.bounding_box());
                Box::new(BvhNode { bbox, left, right })
            }
        }
    }
}
```

Things worth flagging:

- `objects.split_off(mid)` transfers the second half into a new `Vec`, leaving the first half in the original. This avoids cloning.
- `Aabb::surrounding` is called twice per node — once to pick the axis, once to set the final bbox. The first call could be memoized to halve the setup cost. For today, not worth the complexity.
- `.partial_cmp(&cb).unwrap()` panics on NaN. In a well-formed scene, centroids are finite floats. A more robust version (see exercises) would handle NaN gracefully.
- The `2` case is an optimization, not required — you could treat it as the default case. Special-casing avoids one layer of recursion.

## Step 5 — Plug it into scene building

Open `src/scene_file.rs` and add a variant of `SceneDesc::build` that yields a BVH-wrapped world:

```rust
use crate::bvh::BvhNode;

impl SceneDesc {
    pub fn build_bvh(&self, aspect: f32) -> (Camera, Box<dyn Hittable>) {
        let camera = self.camera.build(aspect);
        let shapes: Vec<Box<dyn Hittable>> = self.shapes.iter().map(|e| e.build()).collect();
        let root = BvhNode::build(shapes);
        (camera, root)
    }
}
```

We return `Box<dyn Hittable>` because the root can be a single `Sphere` (if there's one shape), a `BvhNode` (most common), or a leaf pair. The caller doesn't care — it just calls `.hit(...)`.

`render_to_canvas` already takes `&dyn Hittable`, so it works unchanged. You can pass either `&*root` (if `root: Box<dyn Hittable>`) or a `&HittableList` — the function signature accepts both.

Wire it into `main.rs`. Replace the `desc.build(aspect)` call in the `Render` handler:

```rust
        Command::Render { scene, output, width, height, samples, depth } => {
            let desc = load_scene(&scene)?;
            let aspect = width as f32 / height as f32;
            let (camera, world) = desc.build_bvh(aspect);

            let mut canvas = Canvas::new(width, height);
            let t0 = std::time::Instant::now();
            render_to_canvas(&mut canvas, &camera, &*world, samples, depth);
            eprintln!("rendered in {:.2}s", t0.elapsed().as_secs_f32());

            canvas.save_png(&output)?;
            eprintln!("wrote {}", output.display());
        }
```

Note `&*world` — dereference the `Box` to get `&dyn Hittable`. Rust's auto-deref won't do that for you in this coercion.

Run:

```bash
cargo run --release -- render scenes/three_spheres.ron
```

Expected output: same three-sphere image as yesterday, rendered in roughly the same time. With 5 spheres, BVH setup is overhead; the payoff comes at 100+.

## Step 6 — The benchmark scene

Create `examples/gen_many_spheres.rs`:

```rust
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::io::Write;

fn main() {
    let mut rng = StdRng::seed_from_u64(42);
    let mut out = std::io::BufWriter::new(std::fs::File::create("scenes/many.ron").unwrap());

    writeln!(out, "(").unwrap();
    writeln!(out, "    camera: (").unwrap();
    writeln!(out, "        lookfrom: (13.0, 2.0, 3.0),").unwrap();
    writeln!(out, "        lookat: (0.0, 0.0, 0.0),").unwrap();
    writeln!(out, "        vfov: 20.0,").unwrap();
    writeln!(out, "    ),").unwrap();
    writeln!(out, "    shapes: [").unwrap();
    // Large ground sphere.
    writeln!(out, "        (shape: Sphere(center: (0.0, -1000.0, 0.0), radius: 1000.0), material: Lambertian(albedo: (0.5, 0.5, 0.5))),").unwrap();

    // 484 small spheres on a grid.
    for a in -11..11 {
        for b in -11..11 {
            let cx = a as f32 + 0.9 * rng.gen::<f32>();
            let cz = b as f32 + 0.9 * rng.gen::<f32>();
            let pick = rng.gen::<f32>();
            let mat = if pick < 0.8 {
                let r = rng.gen::<f32>() * rng.gen::<f32>();
                let g = rng.gen::<f32>() * rng.gen::<f32>();
                let b = rng.gen::<f32>() * rng.gen::<f32>();
                format!("Lambertian(albedo: ({r}, {g}, {b}))")
            } else if pick < 0.95 {
                let r = 0.5 * (1.0 + rng.gen::<f32>());
                let g = 0.5 * (1.0 + rng.gen::<f32>());
                let b = 0.5 * (1.0 + rng.gen::<f32>());
                let fuzz = 0.5 * rng.gen::<f32>();
                format!("Metal(albedo: ({r}, {g}, {b}), fuzz: {fuzz})")
            } else {
                "Dielectric(ior: 1.5)".to_string()
            };
            writeln!(out, "        (shape: Sphere(center: ({cx}, 0.2, {cz}), radius: 0.2), material: {mat}),").unwrap();
        }
    }

    // Three big hero spheres.
    writeln!(out, "        (shape: Sphere(center: (0.0, 1.0, 0.0), radius: 1.0), material: Dielectric(ior: 1.5)),").unwrap();
    writeln!(out, "        (shape: Sphere(center: (-4.0, 1.0, 0.0), radius: 1.0), material: Lambertian(albedo: (0.4, 0.2, 0.1))),").unwrap();
    writeln!(out, "        (shape: Sphere(center: (4.0, 1.0, 0.0), radius: 1.0), material: Metal(albedo: (0.7, 0.6, 0.5), fuzz: 0.0))").unwrap();
    writeln!(out, "    ],").unwrap();
    writeln!(out, ")").unwrap();
}
```

Run it:

```bash
cargo run --release --example gen_many_spheres
```

You now have `scenes/many.ron` with ~488 spheres. Render it:

```bash
cargo run --release -- render scenes/many.ron -o many.png --width 400 --height 225 --samples 50
```

## Step 7 — Measure the speedup

To prove BVH actually works, compare it against the linear-scan path. Add both code paths to `SceneDesc`:

```rust
impl SceneDesc {
    pub fn build(&self, aspect: f32) -> (Camera, HittableList) {
        let camera = self.camera.build(aspect);
        let mut world = HittableList::new();
        for entry in &self.shapes {
            world.add(entry.build());
        }
        (camera, world)
    }

    pub fn build_bvh(&self, aspect: f32) -> (Camera, Box<dyn Hittable>) {
        let camera = self.camera.build(aspect);
        let shapes: Vec<Box<dyn Hittable>> = self.shapes.iter().map(|e| e.build()).collect();
        let root = BvhNode::build(shapes);
        (camera, root)
    }
}
```

Create `examples/bench_bvh.rs`:

```rust
use raytracer::canvas::Canvas;
use raytracer::renderer::render_to_canvas;
use raytracer::scene_file::load_scene;
use std::path::Path;
use std::time::Instant;

fn main() {
    let width = 400u32;
    let height = 225u32;
    let samples = 20u32;
    let depth = 50u32;
    let desc = load_scene(Path::new("scenes/many.ron")).expect("scene");
    let aspect = width as f32 / height as f32;

    // Linear scan.
    let (camera, world) = desc.build(aspect);
    let mut canvas = Canvas::new(width, height);
    let t0 = Instant::now();
    render_to_canvas(&mut canvas, &camera, &world, samples, depth);
    let linear = t0.elapsed();
    println!("linear : {linear:.2?}");

    // BVH.
    let (camera, world) = desc.build_bvh(aspect);
    let mut canvas = Canvas::new(width, height);
    let t0 = Instant::now();
    render_to_canvas(&mut canvas, &camera, &*world, samples, depth);
    let bvh = t0.elapsed();
    println!("bvh    : {bvh:.2?}");

    println!("speedup: {:.1}x", linear.as_secs_f32() / bvh.as_secs_f32());
}
```

Run it:

```bash
cargo run --release --example bench_bvh
```

Expected output (representative, not exact):

```
linear : 38.21s
bvh    : 0.96s
speedup: 39.8x
```

The exact number depends on your CPU and how well-distributed the spheres are. Anything in the 20×–60× range is normal for this scene size. If you see <5×, something is wrong with the BVH — probably the `upper` narrowing in `hit` or the axis selection.

### Sanity: does the image match?

Render both ways:

```bash
cargo run --release -- render scenes/many.ron -o many_bvh.png --width 400 --samples 20
```

Then temporarily swap `build_bvh` back to `build` in `main.rs` and render to `many_linear.png`. Compare them byte-for-byte:

```bash
diff <(xxd many_bvh.png) <(xxd many_linear.png)
```

They should be identical. The BVH doesn't change *what* rays hit — only *how fast* you figure out which ones do. Because yesterday's renderer seeds per-pixel (bit-identical parallelism), identical-image is the expected outcome. If your images differ, you have a bug in the BVH traversal (usually a sign error in the slab test, or forgetting the `upper` narrowing).

Swap back to `build_bvh` when you're done.

## Step 8 — Surface Area Heuristic preview

Midpoint splits work well on uniformly distributed spheres (like the grid above). On a scene where most shapes cluster on one side, midpoint splits create lopsided trees and degrade toward O(n).

SAH replaces the midpoint with a cost function. For each candidate split along each axis, score:

```
cost(split) = surface_area(left)  * count(left)
            + surface_area(right) * count(right)
```

The minimum-cost split goes next. This penalizes splits that put all the shapes on one side (one child has the full scene area × full count) and favors splits that shrink both sides.

A practical implementation:

1. Sort shapes by centroid along each axis.
2. Precompute prefix AABBs (left-to-right) and suffix AABBs (right-to-left).
3. Scan through split points; compute cost in O(1) per candidate using the precomputed boxes.
4. Pick the minimum across all three axes.

This takes the BVH from "decent" to "great" — typically another 2× on top of midpoint. For production ray tracers (pbrt, embree, etc.), SAH or a variant is standard. For a hobby project, midpoint is fine; we stop here today.

## Common pitfalls

### BVH and linear scan render different images

Usually a sign error in the slab method or a missing axis swap when `inv_d < 0`. Add a test that renders a simple scene both ways and asserts byte-equal output. The common trigger is `if inv_d < 0.0 { swap(&mut t0, &mut t1) }` — some tutorials write it as `direction.x < 0`, which is equivalent but one refactor away from breaking.

### BVH is slower than linear scan

Check:

1. Are you narrowing `t_max` when traversing the right subtree? Missing this regression alone turns BVH into O(n) on some scenes.
2. Is your tree degenerate? Print the tree depth (recursive helper) — for N shapes, depth should be ~log2(N), not N. If it's N, the sort comparator is returning `Ordering::Equal` for everything.
3. Are you rebuilding the BVH inside the pixel loop? Build once before rendering.

### Partial-ord panic on NaN

`partial_cmp(...).unwrap()` panics if either centroid is NaN. This shouldn't happen with valid scenes, but a safer helper:

```rust
ca.partial_cmp(&cb).unwrap_or(std::cmp::Ordering::Equal)
```

This sorts NaN values to the end. Better yet, validate scene data at load time and reject NaN-containing shapes before building.

### Rayon + BVH performance is disappointing

Check `rayon::current_num_threads()` — if it's 1, something is pinning the pool. The BVH traversal itself is pure reads into immutable data behind `&dyn Hittable`, which is `Send + Sync` because of the trait bound. There's no locking, no false sharing. If you see linear scaling on the old linear-scan renderer but flat scaling on the BVH renderer, something else (allocator contention, thermal throttling) is at play, not the BVH itself.

### Borrow checker fight in `BvhNode::build`

Rust's ownership rules mean once you `split_off`, you can't use the old half as `&mut Vec`. Our `split_off` comes after the `sort_by`, so this is fine — but if you rearrange the order and try to sort the right half after splitting, the borrow checker will complain about overlapping mutable borrows. Sort *before* splitting.

### Stack overflow on huge scenes

Our `BvhNode::build` recurses. For 1M shapes, tree depth is ~20 — no problem. For pathological inputs (all shapes at the same centroid), depth could approach N. If you hit stack overflow, convert to an iterative loop with an explicit work queue, or increase the thread stack size.

## What you learned

- **AABBs** are two `Vec3`s (min/max) — cheap to construct, cheap to combine, cheap to test.
- **The slab method** tests ray-AABB by walking three axes; the intervals must overlap.
- **BVH nodes** are binary trees over `Box<dyn Hittable>`, where each node carries an AABB. A single trait makes leaves and internal nodes interchangeable.
- **Narrowing `t_max`** after the first child hit is critical — forget it and you lose most of the speedup.
- **Midpoint split on longest axis** is good enough for most hobby scenes. SAH does 2× better at 10× the code.
- **Build once, render many** — BVH construction is O(n log n), not something you redo per ray.
- **BVH composes with rayon cleanly** because traversal only reads `&dyn Hittable`. No locks, no shared state.
- **Linear vs BVH should produce byte-identical images** — when they don't, you have a real bug, not a rounding issue.

## Exercises

1. **SAH splits.** Replace the midpoint split with SAH. Measure the traversal-time improvement on `scenes/many.ron` (same image, just faster). Report the speedup.
2. **Print tree shape.** Add a `fn debug_dump(&self, depth: usize)` to `BvhNode` that prints `[depth] node aabb=... n_shapes=...`. Run it on `scenes/many.ron`. Verify the depth is ~log2(N).
3. **Axis-specific sort caching.** The current build sorts fresh at every node. Instead, sort once along each axis up front and store three arrays of indices. At each split, reuse the precomputed order. Should shave 20-30% off build time at large N.
4. **BVH for planes?** Planes have no finite AABB. How would you integrate them with a BVH? One answer: keep a separate `Vec<Plane>` that every ray always tests, in addition to the BVH test. Implement this split and measure whether it's actually worse than putting planes in a giant AABB.
5. **Persistent BVH.** Use `serde` (from Day 27) to serialize a built `BvhNode` to disk, then load it back. Avoids rebuild time for static scenes. What's tricky: `Box<dyn Hittable>` isn't directly serializable — you'd need a `serde`-friendly tree representation alongside the runtime one.
6. **Verify equivalence in a test.** Add an integration test that generates a random scene (seed-fixed), renders it with the linear path and the BVH path, and asserts the images are byte-identical. Catches regressions in BVH math.

## What's next

You've built a fast, parallel, data-driven ray tracer. It's real software: tens of files, multiple crates, clean error paths, tests, benchmarks. That's the last technical skill for this course.

**Day 29 is the capstone kickoff.** You'll pick one of three tracks — Games, Database, or 3D Graphics — and spec out the project that will define the last two days of the course. You won't write production code yet: the deliverable is a `README.md` with acceptance criteria, a module scaffold with `todo!()` stubs, and a hello-world end-to-end path proving the skeleton compiles and runs. **Day 30** then turns that skeleton into a shipped project.

→ [Day 29 — Capstone Design](day-29.md)
