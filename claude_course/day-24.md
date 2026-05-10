# Day 24 — Hittable Trait, Camera, and Antialiasing

**Domain:** 3D graphics • **Time:** 2 hours • **Difficulty:** medium

## What you'll build

A proper `Hittable` trait so your ray tracer works with a *collection of any shapes*, not just spheres. A flexible `Camera` with configurable origin (`lookfrom`), target (`lookat`), up vector, and vertical field of view. And **multi-sample antialiasing (MSAA)**: fire N rays per pixel with tiny random offsets, average the colors, and the sphere silhouettes become silky smooth instead of staircased. By the end of today your renders look significantly more like "real" ray tracer output.

## What you'll learn

- Traits with `&self` methods for polymorphism (**dynamic dispatch** via `Box<dyn Trait>`)
- `HitRecord` as a return type rich enough to carry normal, t, point
- Building an orthonormal camera basis from `lookfrom`/`lookat`/`up`
- Random number generation with the `rand` crate
- Why averaging multiple rays per pixel = antialiasing
- Time-efficient sampling patterns (jittered vs. random)
- Designing trait objects so future shapes (cylinders, triangles, meshes) plug in without modifying existing code

## Background

### Dynamic dispatch in Rust

On Day 4 you met traits, and on Day 5 you met generics with `<T: Trait>`. Generics are **static dispatch**: the compiler generates a specialized copy of the calling function for each concrete type (zero runtime cost, larger binary).

Today we want something different. We want a `Vec<Thing>` where each element might be a `Sphere` or a `Plane` or a `Triangle` — concrete types decided at runtime. That's **dynamic dispatch**:

```rust
let world: Vec<Box<dyn Hittable>> = vec![
    Box::new(Sphere { ... }),
    Box::new(Plane { ... }),
];
```

`Box<dyn Hittable>` is a "trait object": a fat pointer holding (a) a pointer to the concrete data, (b) a pointer to a virtual dispatch table (vtable) of the trait's methods. Calling `thing.hit(ray)` looks up `hit` in the vtable and calls it — one indirect jump. Modern CPUs branch-predict this well; in practice the overhead is a few nanoseconds per call.

Rules for trait objects (`dyn Trait`):

1. Methods must be **object-safe**: no `Self` in return types or parameters (except as `&self`/`&mut self`), no generics.
2. You own the object behind some kind of pointer: `Box<dyn T>`, `&dyn T`, `Arc<dyn T>`, `Rc<dyn T>`.
3. A trait object is *dynamically sized* — you can't move one onto the stack directly (that's what the `Box` is for).

### Why not use an enum?

You could write `enum Shape { Sphere(Sphere), Plane(Plane) }` and match on it. That gives you static dispatch (faster) and closed polymorphism (every variant known at compile time). It's a reasonable choice for small fixed sets.

We'll use `Box<dyn Hittable>` because it's the idiomatic approach for extensible systems and it pairs nicely with scene files later (Day 27). You can switch to enums for a 5-15% speedup if you profile and decide it matters.

### The pinhole camera, properly

Day 23's camera was fixed: at origin, looking down -Z. We want to configure:

- `lookfrom`: where the camera is
- `lookat`: a point it's pointing at
- `vup`: a "world up" hint (usually `(0, 1, 0)`) to resolve rotation around the view direction
- `vfov`: vertical field of view in degrees
- `aspect`: width / height

From these we build an orthonormal basis `(u, v, w)`:

```
w = (lookfrom - lookat).normalize()   // "backward" from camera
u = vup.cross(w).normalize()           // camera "right"
v = w.cross(u)                         // camera "up"
```

`w` is backward (following OpenGL convention: camera looks down -Z). `u` points right on the image. `v` points up on the image. All three are unit vectors and mutually perpendicular.

Then viewport height = `2 * tan(vfov/2)`, width = viewport_height * aspect. Position the viewport one unit in front of the camera (focal length = 1 unit along -w):

```
lower_left = lookfrom - (horizontal/2) - (vertical/2) - w
horizontal = viewport_width * u
vertical = viewport_height * v
```

Generating a ray through normalized screen coords `(s, t)`:

```
direction = lower_left + s*horizontal + t*vertical - lookfrom
```

Note `direction` is the viewport-hit point minus the origin, *not normalized*. We usually don't normalize ray direction until we need distance.

### Antialiasing, intuitively

A pixel is a square region of the image. The "correct" color for that pixel is the average of the light arriving through every point in that square. A single ray samples only one point, so if the square straddles a sphere edge, you get a discrete hit-or-miss — the classic staircase.

Shooting many rays at small offsets inside the pixel, then averaging, approximates the integral. More samples = smoother. The convergence is O(1/sqrt(N)): to halve the noise you need 4x the samples.

For a demo we'll use 10-100 samples. Production offline renderers might use 1000-10000. The difference between "bad" and "acceptable" is often at N=16; the difference between "acceptable" and "clean" is at N=100.

Two sampling strategies:

- **Random (Monte Carlo)**: pure uniform random in `[0, 1) × [0, 1)` within the pixel. Unbiased but noisy at low sample counts.
- **Jittered**: divide the pixel into a √N × √N grid, pick one random point in each cell. Lower variance, less banding. A sweet spot for medium sample counts.

We'll use random today for simplicity. Jittered is an exercise.

## Setting up

Still in the `raytracer` project. Add the `rand` crate:

```bash
cargo add rand
```

Your `Cargo.toml` dependencies should now include:

```toml
[dependencies]
image = "0.25"
clap = { version = "4", features = ["derive"] }
rand = "0.8"
```

## Step 1 — Define the `Hittable` trait

Open `src/hit.rs` and rewrite it. We'll keep the `Sphere` type but add the trait:

```rust
use crate::ray::Ray;
use crate::vec3::Vec3;

/// Information about a ray-surface intersection.
pub struct HitRecord {
    pub t: f32,
    pub point: Vec3,
    pub normal: Vec3,
    /// True if the ray hit the outside of the surface, false if it hit the inside.
    pub front_face: bool,
}

impl HitRecord {
    /// Create a HitRecord with a properly oriented normal.
    /// `outward_normal` should always point away from the surface.
    pub fn new(t: f32, point: Vec3, outward_normal: Vec3, ray: &Ray) -> Self {
        let front_face = ray.direction.dot(outward_normal) < 0.0;
        let normal = if front_face {
            outward_normal
        } else {
            -outward_normal
        };
        Self { t, point, normal, front_face }
    }
}

/// Any shape that can be tested for ray intersection.
pub trait Hittable: Send + Sync {
    fn hit(&self, ray: &Ray, t_min: f32, t_max: f32) -> Option<HitRecord>;
}
```

### What's `front_face` for?

A ray can hit a sphere from outside or inside. A glass sphere has both cases — the ray enters, bounces around internally, hits the back wall from inside. The material code on Day 25 needs to know which side of the surface the ray hit to compute correct refraction.

Convention: the "outward normal" points out of the solid. `front_face = true` means "ray hit the outside." We flip the stored normal to always face into the ray, which simplifies lighting math.

### Why `Send + Sync` in the trait bound?

`Send` = safe to transfer to another thread. `Sync` = safe to share a reference across threads. We need both because on Day 26 we'll parallelize with rayon: every thread accesses the shared `Vec<Box<dyn Hittable>>`, and the `Hittable` values inside need to be thread-safe. Adding the bound now means all future `Hittable` impls must also be thread-safe — a one-time discipline that saves you a frustrating refactor later.

### Implement `Hittable` for `Sphere`

Below the trait:

```rust
pub struct Sphere {
    pub center: Vec3,
    pub radius: f32,
}

impl Sphere {
    pub fn new(center: Vec3, radius: f32) -> Self {
        Self { center, radius }
    }
}

impl Hittable for Sphere {
    fn hit(&self, ray: &Ray, t_min: f32, t_max: f32) -> Option<HitRecord> {
        let m = ray.origin - self.center;
        let a = ray.direction.dot(ray.direction);
        let half_b = ray.direction.dot(m);
        let c = m.dot(m) - self.radius * self.radius;

        let disc = half_b * half_b - a * c;
        if disc < 0.0 {
            return None;
        }

        let sqrt_d = disc.sqrt();

        // Find nearest root in [t_min, t_max]
        let mut t = (-half_b - sqrt_d) / a;
        if t < t_min || t > t_max {
            t = (-half_b + sqrt_d) / a;
            if t < t_min || t > t_max {
                return None;
            }
        }

        let point = ray.at(t);
        let outward_normal = (point - self.center) / self.radius;
        Some(HitRecord::new(t, point, outward_normal, ray))
    }
}
```

Division by `self.radius` rather than `.normalize()` is a micro-optimization: we already know the length is `radius` (for any point on the sphere surface), so scalar division is faster than computing a square root.

## Step 2 — Hittable list

A scene is a collection of hittables. The natural thing: implement `Hittable` for `Vec<Box<dyn Hittable>>` itself, so we can just call `world.hit(ray, ...)`.

Still in `src/hit.rs`:

```rust
pub struct HittableList {
    pub items: Vec<Box<dyn Hittable>>,
}

impl HittableList {
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    pub fn add(&mut self, item: Box<dyn Hittable>) {
        self.items.push(item);
    }

    pub fn clear(&mut self) {
        self.items.clear();
    }
}

impl Default for HittableList {
    fn default() -> Self {
        Self::new()
    }
}

impl Hittable for HittableList {
    fn hit(&self, ray: &Ray, t_min: f32, t_max: f32) -> Option<HitRecord> {
        let mut closest_so_far = t_max;
        let mut best_hit: Option<HitRecord> = None;

        for item in &self.items {
            if let Some(rec) = item.hit(ray, t_min, closest_so_far) {
                closest_so_far = rec.t;
                best_hit = Some(rec);
            }
        }

        best_hit
    }
}
```

The trick: we pass `closest_so_far` as `t_max` to each subsequent hit test. That's a free optimization — any hit farther than the current best is rejected without building a `HitRecord`. This becomes important when you have many objects.

### Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sphere_hits_record_correct_normal() {
        let sphere = Sphere::new(Vec3::new(0.0, 0.0, -5.0), 1.0);
        let ray = Ray::new(Vec3::ZERO, Vec3::new(0.0, 0.0, -1.0));
        let hit = sphere.hit(&ray, 0.0, f32::INFINITY).unwrap();

        assert!((hit.t - 4.0).abs() < 1e-4);
        // Normal at (0, 0, -4) should point at camera: (0, 0, 1)
        assert!((hit.normal - Vec3::new(0.0, 0.0, 1.0)).length() < 1e-4);
        assert!(hit.front_face);
    }

    #[test]
    fn hittable_list_picks_closest() {
        let mut world = HittableList::new();
        world.add(Box::new(Sphere::new(Vec3::new(0.0, 0.0, -5.0), 1.0)));
        world.add(Box::new(Sphere::new(Vec3::new(0.0, 0.0, -10.0), 1.0)));

        let ray = Ray::new(Vec3::ZERO, Vec3::new(0.0, 0.0, -1.0));
        let hit = world.hit(&ray, 0.0, f32::INFINITY).unwrap();
        // Closest sphere is at z = -5, surface at z = -4, t = 4
        assert!((hit.t - 4.0).abs() < 1e-4);
    }

    #[test]
    fn hittable_list_miss() {
        let mut world = HittableList::new();
        world.add(Box::new(Sphere::new(Vec3::new(0.0, 0.0, -5.0), 1.0)));

        let ray = Ray::new(Vec3::ZERO, Vec3::new(1.0, 0.0, 0.0));
        assert!(world.hit(&ray, 0.0, f32::INFINITY).is_none());
    }
}
```

Run:

```bash
cargo test hit
```

All three should pass.

## Step 3 — Build the real camera

Open `src/camera.rs` and replace with:

```rust
use crate::ray::Ray;
use crate::vec3::Vec3;

pub struct Camera {
    origin: Vec3,
    lower_left: Vec3,
    horizontal: Vec3,
    vertical: Vec3,
}

impl Camera {
    /// vfov is in degrees, aspect is width/height.
    pub fn new(lookfrom: Vec3, lookat: Vec3, vup: Vec3, vfov_deg: f32, aspect: f32) -> Self {
        let theta = vfov_deg.to_radians();
        let h = (theta / 2.0).tan();
        let viewport_height = 2.0 * h;
        let viewport_width = aspect * viewport_height;

        // Camera basis: w points backward, u points right, v points up.
        let w = (lookfrom - lookat).normalize();
        let u = vup.cross(w).normalize();
        let v = w.cross(u);

        let origin = lookfrom;
        let horizontal = u * viewport_width;
        let vertical = v * viewport_height;
        let lower_left = origin - horizontal / 2.0 - vertical / 2.0 - w;

        Self { origin, lower_left, horizontal, vertical }
    }

    pub fn ray(&self, s: f32, t: f32) -> Ray {
        Ray::new(
            self.origin,
            self.lower_left + self.horizontal * s + self.vertical * t - self.origin,
        )
    }
}
```

Note we renamed the old `SimpleCamera` away. If anything else references it, update those too.

### A sanity test

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn camera_points_at_target() {
        let cam = Camera::new(
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 0.0, -1.0),
            Vec3::new(0.0, 1.0, 0.0),
            90.0,
            16.0 / 9.0,
        );

        // A ray through the middle of the viewport (s=0.5, t=0.5)
        // should point roughly at the lookat.
        let r = cam.ray(0.5, 0.5);
        let dir = r.direction.normalize();
        // Expected direction: (0, 0, -1)
        assert!((dir - Vec3::new(0.0, 0.0, -1.0)).length() < 1e-4);
    }
}
```

Run `cargo test camera`. It should pass.

## Step 4 — Multi-sample rendering

Rewrite `render_first_sphere` in `src/main.rs`, but really let's rename the subcommand to `render` and make it the main path going forward:

In the `Command` enum:

```rust
    /// Render the standard test scene with antialiasing.
    Render {
        #[arg(short, long, default_value = "render.png")]
        output: String,
        #[arg(long, default_value_t = 400)]
        width: u32,
        #[arg(long, default_value_t = 225)]
        height: u32,
        /// Number of rays per pixel.
        #[arg(long, default_value_t = 10)]
        samples: u32,
    },
```

In `main()`:

```rust
        Command::Render { output, width, height, samples } => {
            render(&output, width, height, samples)
        }
```

Now the rendering function itself:

```rust
use raytracer::{
    camera::Camera,
    canvas::Canvas,
    hit::{Hittable, HittableList, Sphere},
    ray::Ray,
    vec3::Vec3,
};
use rand::Rng;

fn ray_color(ray: &Ray, world: &dyn Hittable) -> Vec3 {
    if let Some(hit) = world.hit(ray, 0.001, f32::INFINITY) {
        // Debug: color by normal
        return (hit.normal + Vec3::splat(1.0)) * 0.5;
    }

    // Sky
    let unit = ray.direction.normalize();
    let t = 0.5 * (unit.y + 1.0);
    let top = Vec3::new(0.5, 0.7, 1.0);
    let bottom = Vec3::new(1.0, 1.0, 1.0);
    bottom * (1.0 - t) + top * t
}

fn render(output: &str, width: u32, height: u32, samples: u32) {
    let aspect = width as f32 / height as f32;

    let camera = Camera::new(
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(0.0, 0.0, -1.0),
        Vec3::new(0.0, 1.0, 0.0),
        90.0,
        aspect,
    );

    let mut world = HittableList::new();
    world.add(Box::new(Sphere::new(Vec3::new(0.0, 0.0, -1.0), 0.5)));
    world.add(Box::new(Sphere::new(Vec3::new(0.0, -100.5, -1.0), 100.0)));

    let mut canvas = Canvas::new(width, height);
    let mut rng = rand::thread_rng();

    let inv_samples = 1.0 / samples as f32;

    for y in 0..height {
        for x in 0..width {
            let mut accum = Vec3::ZERO;

            for _ in 0..samples {
                // Random offset within the pixel
                let du: f32 = rng.gen();
                let dv: f32 = rng.gen();

                let s = (x as f32 + du) / (width - 1) as f32;
                // Flip Y: image row 0 = top, viewport t=1 = top
                let t = 1.0 - (y as f32 + dv) / (height - 1) as f32;

                let ray = camera.ray(s, t);
                accum = accum + ray_color(&ray, &world);
            }

            let color = accum * inv_samples;
            canvas.set(x, y, [color.x, color.y, color.z]);
        }

        // Progress indicator: print a line every 10% of the image
        if height >= 10 && y % (height / 10) == 0 {
            eprintln!("Rendering... row {y}/{height}");
        }
    }

    canvas.save_png(output).expect("save failed");
    eprintln!("Done. Wrote {width}x{height} image to {output}");
}
```

### What's new

- `ray_color` takes `&dyn Hittable` (note: trait object reference, not a `Vec<Sphere>`).
- We accumulate `samples` colors and divide, instead of using one.
- Random offsets `du, dv` in `[0, 1)` make the sample position jitter within the pixel.
- Progress output to stderr — visible in the terminal without contaminating stdout.

### Run it

```bash
cargo run --release -- render --output low.png --samples 1
cargo run --release -- render --output med.png --samples 10
cargo run --release -- render --output hi.png --samples 100
```

Compare the three. `low.png` (1 sample) has visibly jagged sphere edges. `med.png` (10 samples) is much smoother but grainy. `hi.png` (100 samples) looks clean.

Timing on a laptop: `samples=1` is instant, `samples=10` under a second, `samples=100` about 5-10 seconds. This is single-threaded; Day 26's `rayon` parallelism will cut it by the number of cores.

## Step 5 — Verify edge smoothing

Zoom in on the top of the near sphere (pixel row around 150 at the sphere's top). In `low.png`, you'll see individual staircase steps where the silhouette crosses a pixel. In `hi.png`, those pixels have intermediate shades — the sample average of "75% sphere, 25% sky" produces a smooth gradient across the edge.

This is the whole point of antialiasing.

## Step 6 — A quick scalability benchmark

A naive worry: "adding the trait object dispatch must have slowed things down." Let's measure. Create `benches/render.rs`:

```rust
use criterion::{criterion_group, criterion_main, Criterion};
use raytracer::{
    camera::Camera,
    hit::{Hittable, HittableList, Sphere},
    vec3::Vec3,
};

fn bench_trace(c: &mut Criterion) {
    let camera = Camera::new(
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(0.0, 0.0, -1.0),
        Vec3::new(0.0, 1.0, 0.0),
        90.0,
        16.0 / 9.0,
    );
    let mut world = HittableList::new();
    world.add(Box::new(Sphere::new(Vec3::new(0.0, 0.0, -1.0), 0.5)));
    world.add(Box::new(Sphere::new(Vec3::new(0.0, -100.5, -1.0), 100.0)));

    c.bench_function("trace_single_ray", |b| {
        b.iter(|| {
            let ray = camera.ray(0.5, 0.5);
            world.hit(&ray, 0.001, f32::INFINITY)
        });
    });
}

criterion_group!(benches, bench_trace);
criterion_main!(benches);
```

Add to `Cargo.toml`:

```toml
[dev-dependencies]
criterion = "0.5"

[[bench]]
name = "render"
harness = false
```

Run `cargo bench --bench render`:

```
trace_single_ray    time:   [20 ns 22 ns 24 ns]
```

~20 nanoseconds for one ray through one `Hittable::hit` call. At 400x225 = 90,000 pixels × 10 samples = 900,000 rays, that's ~18 ms of pure intersection work. The rest (~1s total for `samples=10`) is PNG encoding, Vec3 math overhead, and the render loop. Fine.

If you're curious about `dyn` cost, add a monomorphic version and compare — you'll find dyn dispatch is about 2-5 ns slower per call, which matters for millions of rays but not thousands.

## Common pitfalls

### `Vec<Box<dyn Hittable>>` errors

A common first-try: `let world: Vec<Box<Hittable>>`. Missing `dyn`, which is now required in Rust 2018+. Fix: `Vec<Box<dyn Hittable>>`.

Another: `world.push(Sphere::new(...))`. Missing `Box::new`. Trait objects need to be heap-allocated. Fix: `world.push(Box::new(Sphere::new(...)))`.

### Forgetting `Send + Sync` bound

If the trait doesn't require `Send + Sync`, `rayon::par_iter` on the world will fail with: `Sync is not satisfied`. Adding the bound to `trait Hittable: Send + Sync` (day 24) fixes this for all current and future impls.

### `-w` vs. `+w`

If you get the camera basis direction wrong (reversing `lookfrom - lookat`), the camera looks *backward* — you render the back of the scene. Visually: the sphere appears mirrored or missing. Trace through: `w` should point from `lookat` toward `lookfrom`, i.e., "behind" the camera from its perspective.

### Vertical FOV vs. horizontal FOV

Beginners often use "fov" to mean horizontal, but the math here is in vertical FOV. If your sphere looks too small at `--fov 60`, you probably wanted `--fov 90` (60° horizontal ≈ 90° vertical at 16:9). Pick one convention and label it.

### Sample banding at low N

If `samples=10` gives you noticeable banding (not noise, but diagonal stripes), you're likely using a deterministic sampling pattern (e.g., `du = (i / samples) as f32 * step`). Random samples scramble any visible pattern. Always `rng.gen()` for each sample.

### `front_face` off by one

If you invert the dot product (`ray.direction.dot(outward_normal) > 0.0` instead of `< 0.0`), your ray-transmission math on Day 25 will break for glass. Verify: a ray hitting from outside should have `front_face = true`. Check the test.

### Random isn't deterministic → test flakiness

If a test uses `rand::thread_rng()` directly, it's non-reproducible. For tests that need determinism: `StdRng::seed_from_u64(42)`. For the render itself, non-determinism is usually fine (and what you want — it's how MSAA avoids banding).

## What you learned

- `trait Hittable` with `Box<dyn Hittable>` gives polymorphic collections at runtime.
- `Send + Sync` bound in the trait prepares for future parallelism.
- `HitRecord` bundles everything the caller needs from an intersection (t, point, normal, front_face).
- `HittableList` that itself implements `Hittable` lets you compose collections.
- Progressive rejection via `closest_so_far` is essentially free optimization.
- An orthonormal camera basis `(u, v, w)` lets you aim anywhere with `lookfrom`/`lookat`/`vup`.
- Multi-sample antialiasing: N rays per pixel, random offsets, averaged.
- Convergence is O(1/√N) — 4x more samples for half the noise.

## Exercises

1. **Jittered sampling.** Replace `rng.gen()` offsets with a `√N × √N` stratified grid: one random sample per cell. Compare noise at N=4, 16, 64 to plain random.
2. **Plane shape.** Add `struct Plane { point: Vec3, normal: Vec3 }` implementing `Hittable`. Math: intersect ray with plane equation `(P - point)·normal = 0`, solve for `t`. Add a plane under the spheres as your "ground" instead of the huge sphere.
3. **FOV sweep.** Render the same scene at vfov = 30°, 60°, 90°, 120°. Observe: zoomed in, then out, then fish-eye. Commit all four to disk.
4. **Lookfrom/lookat CLI args.** Add `--lookfrom x y z --lookat x y z --vfov deg` to the `render` subcommand. Take a shot from behind, above, etc.
5. **Sampler as trait.** Define `trait Sampler { fn next(&mut self) -> f32 }` and implement `RandomSampler` and `StratifiedSampler`. Now the renderer is parameterized on sampling strategy.

## What's next

Day 25 introduces **materials**. Today everything is colored by surface normal — that's a debug hack. Tomorrow you'll add Lambertian diffuse (matte plastic), Metal (mirrored surfaces with controllable roughness), and Dielectric (glass, with Fresnel-based refraction). Rays will bounce recursively up to 50 times, accumulating color through each reflection. The output will look like a real ray tracer's "hello world": three spheres of different materials, reflecting each other and the sky, with nice lighting.

→ [Day 25 — Materials: Diffuse, Metal, and Glass](day-25.md)
