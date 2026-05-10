# Day 23 — Vec3, Rays, and the First Sphere

**Domain:** 3D graphics • **Time:** 2 hours • **Difficulty:** medium

## What you'll build

A `Vec3` type with overloaded operators (`+`, `-`, `*`, `/`, `-`) and all the helpers a renderer needs (dot, cross, length, normalize). A `Ray` struct. Then the star of the day: **ray-sphere intersection** via the quadratic formula. You'll shoot a ray through every pixel, test whether it hits a sphere, and render the result — a red sphere floating in a blue sky. This is your first real 3D image.

## What you'll learn

- Rust **operator overloading** via the `std::ops` traits
- Why you implement operators for both `Vec3` and `&Vec3`
- **Dot product** geometric meaning: projection, parallelism, angle
- **Cross product**: perpendicular, orientation, the right-hand rule
- Why ray tracing uses `P(t) = origin + t * direction`
- The math of **ray-sphere intersection**: expanding `(P(t) - C)·(P(t) - C) = r²`
- Handling the **discriminant** and picking the nearest positive root

## Background

### The ray-tracer algorithm in one paragraph

For every pixel on your image, you shoot a ray from the camera through that pixel into the scene. If the ray hits an object, you compute the color at the hit point (later: considering materials, shadows, reflections). If it misses, you return the sky color. Repeat for every pixel. Done.

Today we implement the ray, implement one kind of object (a sphere), and the intersection logic. That's enough for a real picture.

### `Vec3`: the workhorse

A `Vec3` represents any 3-element float vector: a 3D point, a direction, a color (R/G/B), a velocity. Conflating "point" and "color" might seem weird but it's idiomatic — the operations are the same math.

We want `Vec3` to support:

```rust
let a = Vec3::new(1.0, 2.0, 3.0);
let b = Vec3::new(4.0, 5.0, 6.0);

let sum = a + b;         // addition
let diff = a - b;        // subtraction
let scaled = a * 2.0;    // scalar mult
let neg = -a;            // negation
let dot = a.dot(b);      // scalar
let cross = a.cross(b);  // Vec3
let len = a.length();    // scalar
let unit = a.normalize();// Vec3
```

Rust doesn't have built-in operator overloading, but the `std::ops` traits are ergonomic. You implement `Add for Vec3`, `Sub for Vec3`, etc., and `a + b` lowers to `a.add(b)`.

### Dot product, geometrically

`a.dot(b) = a.x*b.x + a.y*b.y + a.z*b.z = |a| * |b| * cos(θ)`

Three things you can read off a dot product:

1. **Sign**: `a.dot(b) > 0` means they point in "roughly the same direction" (angle < 90°). `< 0` means opposing. `= 0` means perpendicular.
2. **Projection**: `a.dot(b) / |b|` is the length of `a`'s shadow onto the line through `b`. Crucial for computing how much a light source illuminates a surface.
3. **Length squared**: `a.dot(a) = |a|²`. Useful for comparing lengths without the `sqrt` (monotonic).

In a ray tracer, you see `dot` on almost every line. Lambertian diffuse lighting is just `max(0, normal.dot(light_direction))`. Ray-sphere intersection uses dot products twice.

### Cross product, geometrically

`a.cross(b)` gives you a vector perpendicular to both `a` and `b`, with length `|a|*|b|*sin(θ)`. Direction follows the right-hand rule. Use it for building coordinate frames (camera axes), computing surface normals from triangle vertices, and anything involving rotation.

### Ray parametrization

A ray is an origin point `O` plus a direction `D`:

```
P(t) = O + t * D    (for t >= 0)
```

As `t` grows, you move along the ray. `t = 0` is the origin; `t = 1` is one `D`-length away. If `D` is a unit vector, `t` is distance. If not, `t` is scaled by `|D|`. We don't require normalized directions — it keeps the math general — but it means you have to remember `t` is not always distance.

### Ray-sphere intersection, derived

A sphere centered at `C` with radius `r` is defined by `|P - C|² = r²`. Substituting the ray:

```
|O + t*D - C|² = r²
```

Let `M = O - C` (offset from sphere center to ray origin). Expanding:

```
|M + t*D|² = r²
(M + t*D) · (M + t*D) = r²
M·M + 2t(D·M) + t²(D·D) = r²
t²(D·D) + 2t(D·M) + (M·M - r²) = 0
```

That's a quadratic in `t`. Coefficients:

```
a = D·D
b = 2 * D·M
c = M·M - r²
```

Solve `at² + bt + c = 0`:

```
discriminant = b² - 4ac
t = (-b ± sqrt(discriminant)) / (2a)
```

- **discriminant < 0**: no real root. The ray misses the sphere.
- **discriminant ≥ 0**: one or two roots (`-b - sqrt` is nearer, `-b + sqrt` is farther).

You pick the nearest positive root. Negative `t` means "behind the camera" — discard. If both roots are negative, the sphere is entirely behind you.

### Half-`b` trick

A common micro-optimization: substitute `h = b/2`:

```
discriminant = h² - a*c
t = (-h ± sqrt(disc)) / a
```

The quadratic formula loses the `4` and the `2` — minor speed and precision improvement. We'll use this form.

## Setting up

Still in the `raytracer` project from Day 22. No new deps today.

## Step 1 — Vec3 struct and constructors

Open `src/vec3.rs`. Replace the empty file with:

```rust
use std::ops::{Add, Div, Mul, Neg, Sub};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub const ZERO: Vec3 = Vec3 { x: 0.0, y: 0.0, z: 0.0 };
    pub const ONE: Vec3 = Vec3 { x: 1.0, y: 1.0, z: 1.0 };

    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    pub fn splat(v: f32) -> Self {
        Self { x: v, y: v, z: v }
    }

    pub fn dot(self, other: Vec3) -> f32 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    pub fn cross(self, other: Vec3) -> Vec3 {
        Vec3::new(
            self.y * other.z - self.z * other.y,
            self.z * other.x - self.x * other.z,
            self.x * other.y - self.y * other.x,
        )
    }

    pub fn length_squared(self) -> f32 {
        self.dot(self)
    }

    pub fn length(self) -> f32 {
        self.length_squared().sqrt()
    }

    pub fn normalize(self) -> Vec3 {
        let len = self.length();
        assert!(len > 0.0, "cannot normalize zero vector");
        self / len
    }
}
```

Why `#[derive(Copy, Clone)]`? `Vec3` is three floats — 12 bytes. It's cheap to copy, so we make it `Copy`. No more lifetime puzzles when passing vectors around.

Why `const fn` for `new`? So `Vec3::ZERO` and future constants can be declared at module scope. Useful when defining scenes or test fixtures.

### Compile check

Add `use raytracer::vec3::Vec3;` to a temporary test in `src/main.rs` to make sure it builds:

```rust
// Temporary, remove after verifying
#[test]
fn vec3_builds() {
    let _ = raytracer::vec3::Vec3::new(1.0, 2.0, 3.0);
}
```

Run `cargo check`. No errors? Good. Delete that test block before continuing.

## Step 2 — Arithmetic operators

Still in `src/vec3.rs`, add the operator impls:

```rust
impl Add for Vec3 {
    type Output = Vec3;
    fn add(self, rhs: Vec3) -> Vec3 {
        Vec3::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }
}

impl Sub for Vec3 {
    type Output = Vec3;
    fn sub(self, rhs: Vec3) -> Vec3 {
        Vec3::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
    }
}

impl Neg for Vec3 {
    type Output = Vec3;
    fn neg(self) -> Vec3 {
        Vec3::new(-self.x, -self.y, -self.z)
    }
}

// Vec3 * f32
impl Mul<f32> for Vec3 {
    type Output = Vec3;
    fn mul(self, s: f32) -> Vec3 {
        Vec3::new(self.x * s, self.y * s, self.z * s)
    }
}

// f32 * Vec3 (so you can write `2.0 * v`)
impl Mul<Vec3> for f32 {
    type Output = Vec3;
    fn mul(self, v: Vec3) -> Vec3 {
        v * self
    }
}

// Vec3 * Vec3 (componentwise; useful for color tinting)
impl Mul<Vec3> for Vec3 {
    type Output = Vec3;
    fn mul(self, rhs: Vec3) -> Vec3 {
        Vec3::new(self.x * rhs.x, self.y * rhs.y, self.z * rhs.z)
    }
}

impl Div<f32> for Vec3 {
    type Output = Vec3;
    fn div(self, s: f32) -> Vec3 {
        Vec3::new(self.x / s, self.y / s, self.z / s)
    }
}
```

### Why three `Mul` impls?

Rust's type system is strict about which operand is which. `v * 2.0` calls `Mul<f32> for Vec3`. But `2.0 * v` calls `Mul<Vec3> for f32` — a *different* trait impl. To support both orderings, you need both.

The componentwise `Vec3 * Vec3` is useful for color math: `material_color * light_color` multiplies the channels separately. You could argue for a separate `color_mul` method instead (Rust has no operator distinction), but the convention in the graphics community is to use `*`. Just know it's componentwise, not dot product.

### Adding tests

At the bottom of `src/vec3.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn approx_eq(a: Vec3, b: Vec3) -> bool {
        (a.x - b.x).abs() < 1e-5
            && (a.y - b.y).abs() < 1e-5
            && (a.z - b.z).abs() < 1e-5
    }

    #[test]
    fn basic_arithmetic() {
        let a = Vec3::new(1.0, 2.0, 3.0);
        let b = Vec3::new(4.0, 5.0, 6.0);
        assert_eq!(a + b, Vec3::new(5.0, 7.0, 9.0));
        assert_eq!(a - b, Vec3::new(-3.0, -3.0, -3.0));
        assert_eq!(a * 2.0, Vec3::new(2.0, 4.0, 6.0));
        assert_eq!(2.0 * a, Vec3::new(2.0, 4.0, 6.0));
        assert_eq!(-a, Vec3::new(-1.0, -2.0, -3.0));
        assert_eq!(a / 2.0, Vec3::new(0.5, 1.0, 1.5));
    }

    #[test]
    fn dot_product() {
        let a = Vec3::new(1.0, 2.0, 3.0);
        let b = Vec3::new(4.0, 5.0, 6.0);
        assert_eq!(a.dot(b), 4.0 + 10.0 + 18.0);

        // Perpendicular vectors: dot is zero
        let x = Vec3::new(1.0, 0.0, 0.0);
        let y = Vec3::new(0.0, 1.0, 0.0);
        assert_eq!(x.dot(y), 0.0);
    }

    #[test]
    fn cross_right_hand_rule() {
        let x = Vec3::new(1.0, 0.0, 0.0);
        let y = Vec3::new(0.0, 1.0, 0.0);
        let z = x.cross(y);
        // Right-handed: x × y = z
        assert!(approx_eq(z, Vec3::new(0.0, 0.0, 1.0)));
        // And y × x = -z
        assert!(approx_eq(y.cross(x), Vec3::new(0.0, 0.0, -1.0)));
    }

    #[test]
    fn normalize() {
        let v = Vec3::new(3.0, 4.0, 0.0);
        let n = v.normalize();
        assert!((n.length() - 1.0).abs() < 1e-6);
        assert!(approx_eq(n, Vec3::new(0.6, 0.8, 0.0)));
    }
}
```

Run:

```bash
cargo test vec3
```

Expected:

```
running 4 tests
test vec3::tests::basic_arithmetic ... ok
test vec3::tests::cross_right_hand_rule ... ok
test vec3::tests::dot_product ... ok
test vec3::tests::normalize ... ok
```

Good. Vector math works.

## Step 3 — The Ray struct

Open `src/ray.rs`:

```rust
use crate::vec3::Vec3;

#[derive(Debug, Clone, Copy)]
pub struct Ray {
    pub origin: Vec3,
    pub direction: Vec3,
}

impl Ray {
    pub const fn new(origin: Vec3, direction: Vec3) -> Self {
        Self { origin, direction }
    }

    /// Compute the point P(t) = origin + t * direction.
    pub fn at(&self, t: f32) -> Vec3 {
        self.origin + self.direction * t
    }
}
```

Compact. Every time we shoot a ray, we'll construct a `Ray`, pass it to an intersection function, and (if there's a hit) call `at(t)` to get the hit point.

A quick test:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn at_interpolates() {
        let r = Ray::new(Vec3::new(1.0, 0.0, 0.0), Vec3::new(0.0, 1.0, 0.0));
        assert_eq!(r.at(0.0), Vec3::new(1.0, 0.0, 0.0));
        assert_eq!(r.at(1.0), Vec3::new(1.0, 1.0, 0.0));
        assert_eq!(r.at(2.5), Vec3::new(1.0, 2.5, 0.0));
    }
}
```

## Step 4 — Sphere and intersection

Open `src/hit.rs` — we'll eventually abstract this with a `Hittable` trait on Day 24, but today we hardcode `Sphere`.

```rust
use crate::ray::Ray;
use crate::vec3::Vec3;

pub struct Sphere {
    pub center: Vec3,
    pub radius: f32,
}

impl Sphere {
    pub fn new(center: Vec3, radius: f32) -> Self {
        Self { center, radius }
    }

    /// Returns the `t` value of the nearest intersection in `[t_min, t_max]`,
    /// or `None` if the ray misses (or only hits outside that range).
    pub fn hit(&self, ray: &Ray, t_min: f32, t_max: f32) -> Option<f32> {
        let m = ray.origin - self.center;
        let a = ray.direction.dot(ray.direction);
        let half_b = ray.direction.dot(m);
        let c = m.dot(m) - self.radius * self.radius;

        let disc = half_b * half_b - a * c;
        if disc < 0.0 {
            return None;
        }

        let sqrt_d = disc.sqrt();

        // Try nearest root first.
        let t_near = (-half_b - sqrt_d) / a;
        if t_min <= t_near && t_near <= t_max {
            return Some(t_near);
        }

        // Fall back to farther root (for rays originating inside the sphere).
        let t_far = (-half_b + sqrt_d) / a;
        if t_min <= t_far && t_far <= t_max {
            return Some(t_far);
        }

        None
    }
}
```

### Why `t_min` and `t_max`?

- `t_min` prevents self-intersection: if you're bouncing a ray off a surface, you don't want it to instantly re-hit the same surface at `t = 0`. We'll use a small epsilon like `0.001`.
- `t_max` lets intersection tests short-circuit for objects behind closer ones. Today we use `f32::INFINITY`, but day 24 will use the nearest hit so far.

### Test the intersection

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ray_hits_sphere_in_front() {
        let sphere = Sphere::new(Vec3::new(0.0, 0.0, -5.0), 1.0);
        let ray = Ray::new(Vec3::ZERO, Vec3::new(0.0, 0.0, -1.0));
        let t = sphere.hit(&ray, 0.0, f32::INFINITY).unwrap();
        // Sphere surface at z = -4, so t = 4 (since direction z = -1)
        assert!((t - 4.0).abs() < 1e-4);
    }

    #[test]
    fn ray_misses_sphere() {
        let sphere = Sphere::new(Vec3::new(0.0, 0.0, -5.0), 1.0);
        let ray = Ray::new(Vec3::ZERO, Vec3::new(0.0, 1.0, 0.0));
        assert!(sphere.hit(&ray, 0.0, f32::INFINITY).is_none());
    }

    #[test]
    fn ray_pointing_away_misses() {
        let sphere = Sphere::new(Vec3::new(0.0, 0.0, -5.0), 1.0);
        // Ray goes in +z direction, sphere is in -z
        let ray = Ray::new(Vec3::ZERO, Vec3::new(0.0, 0.0, 1.0));
        assert!(sphere.hit(&ray, 0.0, f32::INFINITY).is_none());
    }

    #[test]
    fn ray_grazes_tangent() {
        // Ray that just touches top of sphere
        let sphere = Sphere::new(Vec3::new(0.0, 0.0, -5.0), 1.0);
        let ray = Ray::new(Vec3::new(0.0, 1.0, 0.0), Vec3::new(0.0, 0.0, -1.0));
        // Exactly on surface - tangent. Should hit at t=5.
        let t = sphere.hit(&ray, 0.0, f32::INFINITY).unwrap();
        assert!((t - 5.0).abs() < 1e-4);
    }
}
```

Run `cargo test hit`. All four should pass.

## Step 5 — A camera and ray generation

We need rays *from* somewhere (the camera origin) *through* each pixel. For today we'll hardcode a simple pinhole camera at the origin, looking down the -Z axis, with a vertical field of view of 90 degrees.

Open `src/camera.rs`:

```rust
use crate::ray::Ray;
use crate::vec3::Vec3;

/// Simple pinhole camera at the origin, looking down -Z.
pub struct SimpleCamera {
    origin: Vec3,
    lower_left: Vec3,      // world-space position of the (0, 0) pixel
    horizontal: Vec3,      // full image width as a world-space vector
    vertical: Vec3,        // full image height as a world-space vector
}

impl SimpleCamera {
    pub fn new(aspect_ratio: f32) -> Self {
        let viewport_height = 2.0;
        let viewport_width = viewport_height * aspect_ratio;
        let focal_length = 1.0;

        let origin = Vec3::ZERO;
        let horizontal = Vec3::new(viewport_width, 0.0, 0.0);
        let vertical = Vec3::new(0.0, viewport_height, 0.0);
        let lower_left =
            origin - horizontal / 2.0 - vertical / 2.0 - Vec3::new(0.0, 0.0, focal_length);

        Self { origin, lower_left, horizontal, vertical }
    }

    /// Returns a ray through normalized screen coords (u, v) in [0, 1].
    /// (0, 0) is bottom-left. (1, 1) is top-right.
    pub fn ray(&self, u: f32, v: f32) -> Ray {
        let target = self.lower_left + self.horizontal * u + self.vertical * v;
        Ray::new(self.origin, target - self.origin)
    }
}
```

### What's a viewport?

The viewport is an imaginary rectangle floating in 3D space one unit in front of the camera. Its corners define the bounds of what the camera can see. We pick height = 2 units, width = height * aspect ratio. The focal length (1) puts it one unit away.

A ray through pixel (x, y) goes from the camera origin through the corresponding point on this viewport. As `(u, v)` sweeps `[0, 1]²`, the ray sweeps the full field of view.

The math `horizontal * u + vertical * v` walks across the viewport: `u=0, v=0` is the bottom-left corner; `u=1, v=1` is the top-right.

Note: `v=0` is the *bottom* of the image here, matching the world-space Y-up convention. When rendering, we'll flip: pixel `y = 0` (top of image) → `v = 1 - y/height`.

## Step 6 — Render the first sphere

Back to `src/main.rs`. Add a new subcommand:

```rust
use raytracer::{
    camera::SimpleCamera,
    canvas::Canvas,
    hit::Sphere,
    ray::Ray,
    vec3::Vec3,
};

// ... inside the Command enum:
    /// Render a single red sphere against the sky.
    FirstSphere {
        #[arg(short, long, default_value = "sphere.png")]
        output: String,
        #[arg(long, default_value_t = 400)]
        width: u32,
        #[arg(long, default_value_t = 225)]
        height: u32,
    },

// ... in main():
        Command::FirstSphere { output, width, height } => {
            render_first_sphere(&output, width, height)
        }
```

Now the actual renderer:

```rust
fn ray_color(ray: &Ray, world: &[Sphere]) -> Vec3 {
    // Find the nearest hit.
    let mut closest_t = f32::INFINITY;
    let mut hit_any = false;
    for sphere in world {
        if let Some(t) = sphere.hit(ray, 0.001, closest_t) {
            closest_t = t;
            hit_any = true;
        }
    }

    if hit_any {
        // For today: solid red.
        Vec3::new(0.8, 0.2, 0.2)
    } else {
        // Sky gradient based on ray direction's Y component.
        let unit = ray.direction.normalize();
        let t = 0.5 * (unit.y + 1.0); // map [-1, 1] → [0, 1]
        let top = Vec3::new(0.5, 0.7, 1.0);
        let bottom = Vec3::new(1.0, 1.0, 1.0);
        bottom * (1.0 - t) + top * t
    }
}

fn render_first_sphere(output: &str, width: u32, height: u32) {
    let aspect = width as f32 / height as f32;
    let camera = SimpleCamera::new(aspect);

    let world = vec![Sphere::new(Vec3::new(0.0, 0.0, -1.0), 0.5)];

    let mut canvas = Canvas::new(width, height);

    for y in 0..height {
        for x in 0..width {
            let u = x as f32 / (width - 1) as f32;
            // Flip Y: pixel row 0 is top, but viewport v=1 is top.
            let v = 1.0 - y as f32 / (height - 1) as f32;
            let ray = camera.ray(u, v);
            let color = ray_color(&ray, &world);
            canvas.set(x, y, [color.x, color.y, color.z]);
        }
    }

    canvas.save_png(output).expect("save failed");
    println!("Wrote {width}x{height} image to {output}");
}
```

Run it:

```bash
cargo run --release -- first-sphere --output sphere.png
```

Expected output:

```
Wrote 400x225 image to sphere.png
```

Open `sphere.png`. You should see a solid red disk (that's the sphere's silhouette) on a blue-to-white sky gradient. The sphere is perfectly round. The aspect ratio of the image makes it a wide frame. This is the "hello world" of ray tracing.

### If the sphere looks like an ellipse

Your `aspect` is being computed wrong, or the viewport height/width doesn't reflect the image's proportions. Check that you're creating the camera with `width as f32 / height as f32`, not the reverse.

### If the sphere is in the wrong place

Move the camera's target. The viewport math puts `(u=0.5, v=0.5)` looking down `-Z` through the sphere center at `(0, 0, -1)`. If your sphere is somewhere else, you're not rendering what you think.

## Step 7 — Normals and a cheap shading hack

A solid-color sphere is boring. Let's color it by the surface normal: each point on the sphere's surface has a direction perpendicular to the surface, the **normal vector** `N`. We can color by `N`.

In `ray_color`, when we detect a hit, we need to know `t`, not just whether there was a hit. Update to return the hit record:

```rust
fn ray_color(ray: &Ray, world: &[Sphere]) -> Vec3 {
    let mut closest_t = f32::INFINITY;
    let mut closest_sphere: Option<&Sphere> = None;
    for sphere in world {
        if let Some(t) = sphere.hit(ray, 0.001, closest_t) {
            closest_t = t;
            closest_sphere = Some(sphere);
        }
    }

    if let Some(sphere) = closest_sphere {
        let hit_point = ray.at(closest_t);
        let normal = (hit_point - sphere.center).normalize();
        // Map normal components from [-1, 1] to [0, 1] for RGB.
        return (normal + Vec3::splat(1.0)) * 0.5;
    }

    let unit = ray.direction.normalize();
    let t = 0.5 * (unit.y + 1.0);
    let top = Vec3::new(0.5, 0.7, 1.0);
    let bottom = Vec3::new(1.0, 1.0, 1.0);
    bottom * (1.0 - t) + top * t
}
```

Re-render:

```bash
cargo run --release -- first-sphere --output sphere-normals.png
```

You should now see a rainbow-shaded sphere. Red dominates where the normal points toward +X (right side). Green where the normal points up. Blue where it points out of the screen (toward +Z, toward the camera). This is the classic "normals as RGB" debug visualization. It's not physically meaningful, but it confirms your normals are computed correctly.

## Step 8 — Add a ground sphere

Let's verify our world-of-spheres handles multiple objects. Change the world list:

```rust
    let world = vec![
        Sphere::new(Vec3::new(0.0, 0.0, -1.0), 0.5),            // main sphere
        Sphere::new(Vec3::new(0.0, -100.5, -1.0), 100.0),      // huge "ground" sphere
    ];
```

The trick is to make the second sphere enormous — radius 100, centered 100.5 below the camera. From the camera's perspective, the top of it looks flat. This is the standard "ground" in introductory ray tracers.

Re-run. You should see the small rainbow sphere sitting on a large greenish-yellow surface, which is actually the top of another sphere. Every ray that went "downward" now hits the big sphere instead of the sky.

## Common pitfalls

### Forgetting to normalize direction before computing sky color

If your sky gradient shifts oddly when you zoom out, you likely forgot `ray.direction.normalize()` before using `unit.y`. An un-normalized direction has inconsistent y-magnitudes, causing color to shift with camera distance. Always normalize.

### `t_min` too small → "acne"

When you add reflections (Day 25), hitting the same surface twice from numerical drift shows up as tiny speckles. A `t_min` of `0.001` prevents this. Today with no reflections, `0.0` works too, but get in the habit.

### Getting `a - b` vs. `b - a` backwards in the direction

In the sky gradient, `top * t + bottom * (1 - t)` with `t = 0.5 * (y + 1)`. If your top and bottom are swapped, the sky is upside down. It looks surreal — like the horizon is at the top of the image. Fix by flipping the lerp.

### Sphere silhouette is elliptical

You computed `aspect_ratio = height / width` instead of `width / height`. The viewport ends up wider-than-tall in the wrong dimension.

### Rainbow sphere has wrong colors

If red is pointing left instead of right, you've inverted an axis in `cross` or `new`. Check the test `x.cross(y) == z` still passes. If it doesn't, your cross product is left-handed.

### Normals are pointing inward

If the normal-colored sphere looks like a weird inversion (the highlight is on the side facing away from the camera), check that you're using `hit_point - center`, not `center - hit_point`. The former gives you the outward normal.

### Missing `Vec3 * Vec3`

You forgot to implement componentwise multiplication. Errors look like: `cannot multiply Vec3 by Vec3`. Add the third `Mul` impl.

## What you learned

- Operator overloading via `std::ops` traits (`Add`, `Sub`, `Mul`, `Div`, `Neg`) makes vector math ergonomic.
- To support `f32 * Vec3` AND `Vec3 * f32`, implement the trait for both orderings.
- `Vec3` implements `Copy` because it's small (12 bytes) — no more reference gymnastics.
- The ray parametrization `P(t) = O + t*D` is the foundation of ray tracing.
- Ray-sphere intersection reduces to a **quadratic in t**; the discriminant tells hit vs. miss.
- Use the half-`b` form of the quadratic for slightly cleaner code.
- Normals at a sphere surface = `(hit_point - center).normalize()`, pointing outward.
- Coloring by normal is a great debug tool for checking orientation math.

## Exercises

1. **Two spheres side by side.** Add a second smaller sphere at `(-0.6, 0.0, -1.0)` with radius 0.3. Which sphere "occludes" the other in the overlap? (It's whichever has smaller `t`.)
2. **Tangent epsilon test.** Deliberately construct a ray that should graze the top of a sphere. Does your intersection return a hit? Should it? Graze cases live on the discriminant = 0 edge.
3. **Pure-refl test pattern.** Color each pixel by `ray.direction.normalize()` (map to [0, 1]). You'll get a smooth gradient that visualizes the camera's field of view.
4. **Light Lambertian term.** At hit points, instead of normal coloring, compute `max(0, normal.dot(light_dir))` with `light_dir = Vec3::new(0.3, 0.7, 0.2).normalize()`. Render: now you have a direction-of-light preview.
5. **View frustum debug.** Add a `--fov` CLI flag. At 60° FOV, objects should fit tighter in frame; at 120° they should look "warped" (fish-eye-like). Verify both work.

## What's next

Two big additions tomorrow. First, the `Hittable` trait — so we can store spheres, planes, triangles, meshes in a single `Vec<Box<dyn Hittable>>` and loop once. Second, **antialiasing**: firing multiple rays per pixel and averaging, which smooths the jaggies around sphere silhouettes dramatically. We'll also build a proper `Camera` struct with `lookfrom`, `lookat`, and a configurable vertical field of view.

→ [Day 24 — Hittable Trait, Camera, and Antialiasing](day-24.md)
