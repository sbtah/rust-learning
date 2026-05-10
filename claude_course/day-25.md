# Day 25 — Materials: Diffuse, Metal, and Glass

**Domain:** 3D graphics • **Time:** 2.5 hours • **Difficulty:** hard

## What you'll build

Three physically-inspired materials — Lambertian (matte), Metal (specular), and Dielectric (glass) — attached to spheres, with recursive ray bouncing up to depth 50. By the end of today you'll render the iconic "three-spheres scene": a matte red center sphere, a polished gold sphere on the right, a glass sphere on the left, all sitting on a matte ground. It's the image every Rust ray tracer tutorial targets because it shows off Lambert shading, Fresnel reflections, and refraction all at once.

## What you'll learn

- The `Material` trait and why it returns `Option<(Ray, Vec3)>`
- **Lambertian scattering**: sample the hemisphere for realistic matte surfaces
- **Metal**: pure reflection via `r = v - 2(v·n)n`, with optional fuzz
- **Dielectric** (glass): **Snell's law** for refraction + **Schlick approximation** for Fresnel
- `Arc<dyn Material>` so materials can be shared between objects
- Recursive `ray_color` with a depth limit
- Why every renderer uses "gamma 2 sqrt" correction and not just at save time

## Background

### Materials = scattering functions

In real physics, when light hits a surface, some of it is absorbed and the rest is scattered in various directions. The distribution of scattered directions is called the BRDF (bidirectional reflectance distribution function). A material definition in a ray tracer is, roughly, a BRDF.

For path tracing (what we're doing), we simplify: when a ray hits a surface, we generate *one* "scattered" ray in a direction sampled from the BRDF, then multiply the color returned from that scattered ray by an "attenuation" factor. Monte Carlo over enough samples, this converges to the true integral.

Our trait:

```rust
pub trait Material: Send + Sync {
    fn scatter(&self, ray_in: &Ray, hit: &HitRecord) -> Option<(Ray, Vec3)>;
}
```

Returns `None` if the ray is fully absorbed. Otherwise returns `(scattered_ray, attenuation)`. Attenuation is a Vec3 — it's the material's color applied componentwise to the light coming back. A red ball absorbs green and blue, so its attenuation is roughly `(0.8, 0.2, 0.2)`.

### Lambertian: matte surfaces

A perfectly Lambertian (matte) surface scatters incoming light uniformly to the entire hemisphere above the surface. Probabilistically, we pick a direction by:

1. Generate a random unit vector `r` on the unit sphere.
2. New direction = `normal + r`.

Why? The distribution of `normal + r` for uniform `r` on the sphere is concentrated near the normal — a classic "cosine-weighted hemisphere" sample. More rays go in the normal direction than along the surface, which matches the `cos(θ)` term in Lambert's law.

That's Shirley's specific formulation; alternatives like "uniform hemisphere" or "true cosine-weighted via spherical coordinates" exist. His version is visually equivalent and easier to implement.

Attenuation: the material's albedo (base color).

### Metal: sharp reflection

A perfect mirror reflects the incoming ray `v` with respect to the normal `n`:

```
r = v - 2 * (v · n) * n
```

For a *polished* mirror, use `r` directly. For a *brushed* metal, add a small random perturbation: `r + fuzz * random_unit_vec`, where `fuzz` is a material parameter from 0 (sharp) to 1 (very rough).

Attenuation: the metal's color (gold ≈ `(0.8, 0.6, 0.2)`, silver ≈ `(0.8, 0.8, 0.8)`).

### Dielectric: refraction and Fresnel

Glass and water are dielectrics. When light hits a glass surface, part reflects and part refracts. The ratio depends on the angle — grazing light reflects strongly (you see sky in a lake near the horizon), head-on light refracts (you can see the bottom of a pool directly below you).

**Snell's law** governs the direction of the refracted ray:

```
n1 * sin(θ1) = n2 * sin(θ2)
```

where `n1`/`n2` are the refractive indices (1.0 for air, 1.5 for glass) and `θ1`/`θ2` are the angles from the normal on either side.

In vector form, with `d` the incoming ray direction (unit) and `n` the normal (unit, pointing away from incident medium):

```
cos_θ1 = -d · n
sin_θ1 = sqrt(1 - cos_θ1²)
η_ratio = n1 / n2  // 1/1.5 = 0.67 for air → glass
```

If `η_ratio * sin_θ1 > 1`, Snell's law has no solution — **total internal reflection** (TIR). This happens for rays inside glass hitting the air boundary at a steep angle. The ray bounces back inside instead of exiting.

Otherwise, the refracted ray direction is:

```
refracted = η_ratio * (d + cos_θ1 * n) - sqrt(1 - η_ratio² * sin_θ1²) * n
```

For the reflection/refraction split (which fraction reflects vs. refracts), the real formula is Fresnel's equations — complicated. In practice, **Schlick's approximation** is close enough:

```
r0 = ((1 - η_ratio) / (1 + η_ratio))²
fresnel ≈ r0 + (1 - r0) * (1 - cos_θ1)⁵
```

Returns a probability in `[0, 1]` that the ray reflects (rather than refracts). Pick randomly.

Attenuation for glass: `(1.0, 1.0, 1.0)` — perfectly clear. For colored glass, tint it.

### Why use `Arc<dyn Material>`?

Multiple spheres might share the same material (50 spheres of identical matte red). Storing each sphere with a unique `Box<dyn Material>` duplicates the material; sharing via `Arc<dyn Material>` is ~8 bytes per sphere and allows mutation-free sharing across threads (which Day 26 parallelism needs).

### Gamma correction: why during accumulation

When we sample a pixel 100 times, we're computing the arithmetic mean of linear-space colors. If we gamma-correct each sample, we'd be averaging *square roots*, not the actual light arriving — numerically wrong. Average in linear space; gamma-correct once at the very end (in `Canvas::save_png`, already done).

But wait — our normal-colored Day 24 output did not show "too dark" renders, because we didn't have physically-meaningful colors. Now with multi-bounce lighting, energy is lost at each bounce, and the final image is surprisingly dim. We need gamma correction more than ever. Fortunately, Day 22's `linear_to_srgb` already handles this.

## Setting up

Still in `raytracer`. We already have `rand`. We'll need a cheap way to generate random unit vectors.

## Step 1 — Random vector helpers on `Vec3`

Add to `src/vec3.rs`:

```rust
use rand::Rng;

impl Vec3 {
    /// Return a random vector in the unit cube [-1, 1]^3.
    pub fn random(rng: &mut impl Rng) -> Vec3 {
        Vec3::new(
            rng.gen_range(-1.0..1.0),
            rng.gen_range(-1.0..1.0),
            rng.gen_range(-1.0..1.0),
        )
    }

    /// Return a random point inside the unit sphere via rejection sampling.
    pub fn random_in_unit_sphere(rng: &mut impl Rng) -> Vec3 {
        loop {
            let p = Vec3::random(rng);
            if p.length_squared() < 1.0 {
                return p;
            }
        }
    }

    /// Return a random unit vector on the sphere surface.
    pub fn random_unit(rng: &mut impl Rng) -> Vec3 {
        Vec3::random_in_unit_sphere(rng).normalize()
    }

    /// Reflect `v` across a surface with normal `n`. Both should be unit vectors for best results.
    pub fn reflect(v: Vec3, n: Vec3) -> Vec3 {
        v - n * (2.0 * v.dot(n))
    }

    /// Refract `uv` across a surface with normal `n`, given `η_ratio = n_in / n_out`.
    pub fn refract(uv: Vec3, n: Vec3, eta_ratio: f32) -> Vec3 {
        let cos_theta = (-uv).dot(n).min(1.0);
        let r_out_perp = (uv + n * cos_theta) * eta_ratio;
        let r_out_parallel = n * -((1.0 - r_out_perp.length_squared()).abs().sqrt());
        r_out_perp + r_out_parallel
    }

    /// Is this vector effectively zero? Used to avoid degenerate scatter rays.
    pub fn near_zero(self) -> bool {
        const EPS: f32 = 1e-8;
        self.x.abs() < EPS && self.y.abs() < EPS && self.z.abs() < EPS
    }
}
```

### Rejection sampling

`random_in_unit_sphere` picks random points in a cube, rejects those outside the sphere, and returns the first inside. The inner/outer volume ratio is `(4/3)π / 8 ≈ 0.52`, so about half of attempts succeed. Fast enough.

### Why `(-uv).dot(n).min(1.0)`?

Floating-point math can produce `1.0000001` from a dot product that should be exactly 1. That would make `sqrt(1 - 1.0000001²)` yield a NaN. `.min(1.0)` clamps it.

### `near_zero`

Sometimes a random scatter direction happens to cancel out with the normal, producing a ray with ~zero length. Such a ray would fail when we try to normalize it or take its dot product. Use `near_zero` to detect and re-roll.

## Step 2 — The Material trait and Lambertian

Create or open `src/material.rs`:

```rust
use crate::hit::HitRecord;
use crate::ray::Ray;
use crate::vec3::Vec3;
use rand::Rng;
use std::sync::Arc;

pub trait Material: Send + Sync {
    fn scatter(
        &self,
        ray_in: &Ray,
        hit: &HitRecord,
        rng: &mut dyn rand::RngCore,
    ) -> Option<(Ray, Vec3)>;
}

pub struct Lambertian {
    pub albedo: Vec3,
}

impl Lambertian {
    pub fn new(albedo: Vec3) -> Arc<Self> {
        Arc::new(Self { albedo })
    }
}

impl Material for Lambertian {
    fn scatter(
        &self,
        _ray_in: &Ray,
        hit: &HitRecord,
        rng: &mut dyn rand::RngCore,
    ) -> Option<(Ray, Vec3)> {
        let mut scatter_dir = hit.normal + Vec3::random_unit(&mut rand::rngs::adapter::ReadRng::new(rng));
        // Hack to pass rng. Actually let's make the trait take Rng differently.
        // ...
        None  // placeholder
    }
}
```

Wait — passing `&mut dyn rand::RngCore` through a trait is awkward and the API is ugly. Let's use a cleaner design: materials generate rays from a random source passed in as a concrete type. Simpler:

```rust
use crate::hit::HitRecord;
use crate::ray::Ray;
use crate::vec3::Vec3;
use rand::rngs::SmallRng;
use rand::Rng;
use std::sync::Arc;

pub trait Material: Send + Sync {
    fn scatter(
        &self,
        ray_in: &Ray,
        hit: &HitRecord,
        rng: &mut SmallRng,
    ) -> Option<(Ray, Vec3)>;
}

pub struct Lambertian {
    pub albedo: Vec3,
}

impl Lambertian {
    pub fn new(albedo: Vec3) -> Arc<Self> {
        Arc::new(Self { albedo })
    }
}

impl Material for Lambertian {
    fn scatter(
        &self,
        _ray_in: &Ray,
        hit: &HitRecord,
        rng: &mut SmallRng,
    ) -> Option<(Ray, Vec3)> {
        let mut scatter_dir = hit.normal + Vec3::random_unit(rng);
        if scatter_dir.near_zero() {
            scatter_dir = hit.normal;
        }
        let scattered = Ray::new(hit.point, scatter_dir);
        Some((scattered, self.albedo))
    }
}
```

Why `SmallRng` specifically? It's a fast non-cryptographic RNG (`rand::rngs::SmallRng`), seedable, `Send`-capable. Perfect for parallel rendering where each thread needs its own independent RNG. We'll use it throughout.

Also, update `Vec3::random_in_unit_sphere` and `Vec3::random_unit` to use `&mut SmallRng` specifically — or better, keep them generic on `impl Rng`, since `SmallRng: Rng`. Our existing definitions (`&mut impl Rng`) are compatible.

### Why concrete `SmallRng` in the trait?

A trait with `fn f<R: Rng>(&self, rng: &mut R)` isn't object-safe (it has a generic type parameter). So `Box<dyn Material>` wouldn't work. Concrete `SmallRng` keeps the trait object-safe.

Alternative: `&mut dyn RngCore`. Works, but requires the `RngCore` dyn trait and a wrapper to use `Rng` methods on it. We'll go concrete for simplicity.

## Step 3 — Metal

Add to `src/material.rs`:

```rust
pub struct Metal {
    pub albedo: Vec3,
    pub fuzz: f32,
}

impl Metal {
    pub fn new(albedo: Vec3, fuzz: f32) -> Arc<Self> {
        Arc::new(Self {
            albedo,
            fuzz: fuzz.clamp(0.0, 1.0),
        })
    }
}

impl Material for Metal {
    fn scatter(
        &self,
        ray_in: &Ray,
        hit: &HitRecord,
        rng: &mut SmallRng,
    ) -> Option<(Ray, Vec3)> {
        let reflected = Vec3::reflect(ray_in.direction.normalize(), hit.normal);
        let fuzzed = reflected + Vec3::random_in_unit_sphere(rng) * self.fuzz;
        // Only scatter if the ray still goes above the surface.
        if fuzzed.dot(hit.normal) > 0.0 {
            Some((Ray::new(hit.point, fuzzed), self.albedo))
        } else {
            None
        }
    }
}
```

The `fuzzed.dot(hit.normal) > 0.0` check: if the fuzz is high enough that the scattered ray dives *into* the surface, we return `None` (ray absorbed). Prevents ugly artifacts at glancing angles with rough metals.

## Step 4 — Dielectric

```rust
pub struct Dielectric {
    /// Index of refraction. 1.5 for glass, 1.33 for water, 2.4 for diamond.
    pub ior: f32,
}

impl Dielectric {
    pub fn new(ior: f32) -> Arc<Self> {
        Arc::new(Self { ior })
    }

    /// Schlick's approximation to Fresnel reflectance.
    fn reflectance(cos: f32, ior_ratio: f32) -> f32 {
        let r0 = ((1.0 - ior_ratio) / (1.0 + ior_ratio)).powi(2);
        r0 + (1.0 - r0) * (1.0 - cos).powi(5)
    }
}

impl Material for Dielectric {
    fn scatter(
        &self,
        ray_in: &Ray,
        hit: &HitRecord,
        rng: &mut SmallRng,
    ) -> Option<(Ray, Vec3)> {
        // Going air → glass: η_in = 1.0, η_out = ior. Ratio = 1.0 / ior.
        // Going glass → air: ratio flipped.
        let ior_ratio = if hit.front_face {
            1.0 / self.ior
        } else {
            self.ior
        };

        let unit_dir = ray_in.direction.normalize();
        let cos_theta = (-unit_dir).dot(hit.normal).min(1.0);
        let sin_theta = (1.0 - cos_theta * cos_theta).sqrt();

        let cannot_refract = ior_ratio * sin_theta > 1.0;
        let reflectance = Self::reflectance(cos_theta, ior_ratio);

        let direction = if cannot_refract || reflectance > rng.gen::<f32>() {
            Vec3::reflect(unit_dir, hit.normal)
        } else {
            Vec3::refract(unit_dir, hit.normal, ior_ratio)
        };

        Some((Ray::new(hit.point, direction), Vec3::ONE))
    }
}
```

Attenuation of `Vec3::ONE` (white, 1.0) means the glass doesn't absorb any color — pure transparent. For tinted glass, use a color like `Vec3::new(1.0, 0.8, 0.8)`.

## Step 5 — Connect materials to spheres

Update `src/hit.rs` to hold materials on spheres:

```rust
use crate::material::Material;
use std::sync::Arc;

// ... (existing HitRecord definition) ...

pub struct HitRecord {
    pub t: f32,
    pub point: Vec3,
    pub normal: Vec3,
    pub front_face: bool,
    pub material: Arc<dyn Material>,
}

impl HitRecord {
    pub fn new(
        t: f32,
        point: Vec3,
        outward_normal: Vec3,
        ray: &Ray,
        material: Arc<dyn Material>,
    ) -> Self {
        let front_face = ray.direction.dot(outward_normal) < 0.0;
        let normal = if front_face {
            outward_normal
        } else {
            -outward_normal
        };
        Self { t, point, normal, front_face, material }
    }
}

pub struct Sphere {
    pub center: Vec3,
    pub radius: f32,
    pub material: Arc<dyn Material>,
}

impl Sphere {
    pub fn new(center: Vec3, radius: f32, material: Arc<dyn Material>) -> Self {
        Self { center, radius, material }
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

        let mut t = (-half_b - sqrt_d) / a;
        if t < t_min || t > t_max {
            t = (-half_b + sqrt_d) / a;
            if t < t_min || t > t_max {
                return None;
            }
        }

        let point = ray.at(t);
        let outward_normal = (point - self.center) / self.radius;
        Some(HitRecord::new(
            t,
            point,
            outward_normal,
            ray,
            Arc::clone(&self.material),
        ))
    }
}
```

We clone the `Arc` on every hit — that's a single atomic refcount bump, ~1 ns. Acceptable for our rates.

Update the test that constructs `Sphere` to also pass a material:

```rust
    #[test]
    fn sphere_hits_record_correct_normal() {
        let mat = Lambertian::new(Vec3::new(0.5, 0.5, 0.5));
        let sphere = Sphere::new(Vec3::new(0.0, 0.0, -5.0), 1.0, mat);
        let ray = Ray::new(Vec3::ZERO, Vec3::new(0.0, 0.0, -1.0));
        let hit = sphere.hit(&ray, 0.0, f32::INFINITY).unwrap();

        assert!((hit.t - 4.0).abs() < 1e-4);
        assert!((hit.normal - Vec3::new(0.0, 0.0, 1.0)).length() < 1e-4);
        assert!(hit.front_face);
    }
```

Need to add `use crate::material::Lambertian;` at the top of the test module. Similarly update `hittable_list_picks_closest` and `hittable_list_miss`.

## Step 6 — Recursive ray_color

Now the real rendering change. In `src/main.rs`:

```rust
use rand::rngs::SmallRng;
use rand::SeedableRng;

fn ray_color(
    ray: &Ray,
    world: &dyn Hittable,
    depth: u32,
    rng: &mut SmallRng,
) -> Vec3 {
    if depth == 0 {
        return Vec3::ZERO;
    }

    if let Some(hit) = world.hit(ray, 0.001, f32::INFINITY) {
        if let Some((scattered, attenuation)) = hit.material.scatter(ray, &hit, rng) {
            return attenuation * ray_color(&scattered, world, depth - 1, rng);
        }
        return Vec3::ZERO;
    }

    // Sky
    let unit = ray.direction.normalize();
    let t = 0.5 * (unit.y + 1.0);
    let top = Vec3::new(0.5, 0.7, 1.0);
    let bottom = Vec3::new(1.0, 1.0, 1.0);
    bottom * (1.0 - t) + top * t
}
```

Three stopping conditions:

1. **Depth exhausted**: return black. Each bounce is a chance to hit a light source; after ~50 bounces, any remaining path is negligible.
2. **Ray absorbed**: material's `scatter` returned `None`. Return black.
3. **Miss**: return sky color. This is how light enters our scene (the sky is our implicit light).

The `attenuation * recursive_call` does a componentwise multiplication — as light bounces through diffuse surfaces, each one attenuates by its color. A ray that bounces off red-red-red spheres gets color `(0.8)³ = 0.51` times the sky color.

## Step 7 — The classic three-spheres scene

Replace the render setup:

```rust
use raytracer::material::{Dielectric, Lambertian, Metal};

fn render(output: &str, width: u32, height: u32, samples: u32) {
    let aspect = width as f32 / height as f32;

    let camera = Camera::new(
        Vec3::new(-2.0, 2.0, 1.0),
        Vec3::new(0.0, 0.0, -1.0),
        Vec3::new(0.0, 1.0, 0.0),
        30.0,
        aspect,
    );

    let mat_ground = Lambertian::new(Vec3::new(0.8, 0.8, 0.0));
    let mat_center = Lambertian::new(Vec3::new(0.1, 0.2, 0.5));
    let mat_left = Dielectric::new(1.5);
    let mat_right = Metal::new(Vec3::new(0.8, 0.6, 0.2), 0.1);

    let mut world = HittableList::new();
    world.add(Box::new(Sphere::new(Vec3::new(0.0, -100.5, -1.0), 100.0, mat_ground)));
    world.add(Box::new(Sphere::new(Vec3::new(0.0,    0.0, -1.0),   0.5, mat_center)));
    world.add(Box::new(Sphere::new(Vec3::new(-1.0,   0.0, -1.0),   0.5, Arc::clone(&mat_left))));
    world.add(Box::new(Sphere::new(Vec3::new(-1.0,   0.0, -1.0), -0.45, mat_left)));  // hollow
    world.add(Box::new(Sphere::new(Vec3::new(1.0,    0.0, -1.0),   0.5, mat_right)));

    let mut canvas = Canvas::new(width, height);
    let mut rng = SmallRng::seed_from_u64(42);
    let inv_samples = 1.0 / samples as f32;
    let max_depth = 50;

    for y in 0..height {
        for x in 0..width {
            let mut accum = Vec3::ZERO;
            for _ in 0..samples {
                let du: f32 = rng.gen();
                let dv: f32 = rng.gen();
                let s = (x as f32 + du) / (width - 1) as f32;
                let t = 1.0 - (y as f32 + dv) / (height - 1) as f32;
                let ray = camera.ray(s, t);
                accum = accum + ray_color(&ray, &world, max_depth, &mut rng);
            }
            let color = accum * inv_samples;
            canvas.set(x, y, [color.x, color.y, color.z]);
        }
        if height >= 10 && y % (height / 10) == 0 {
            eprintln!("Row {y}/{height}");
        }
    }

    canvas.save_png(output).expect("save failed");
    eprintln!("Wrote {width}x{height} image to {output}");
}
```

Add these imports at the top:

```rust
use std::sync::Arc;
use raytracer::material::{Dielectric, Lambertian, Metal};
```

### The hollow glass trick

Two spheres at the same position: radius `0.5` (outer surface, outward-facing normals) and radius `-0.45` (inner surface, *inward-facing* normals — the negative radius flips the outward normal). A ray passing through the outer surface enters glass, passes through the inner surface where it enters the thin-wall of air inside, and refracts correctly to simulate a hollow glass sphere. A single-radius sphere would be solid glass. Try both and see the difference.

### Run it

```bash
cargo run --release -- render --output three.png --samples 100
```

Expect ~60-120 seconds on a laptop for a 400x225 image at 100 samples. Afterward, `three.png` should show:

- Blue matte sphere in the center, with slightly visible diffuse roughness
- Gold metal sphere on the right with smooth reflection of sky and nearby spheres
- Hollow glass sphere on the left showing refraction (scene visible through it, inverted, with rim highlights)
- Yellow-green matte ground below

If the image is dark and grainy, increase `--samples`. If it's too bright near 1.0 (blown out), check that your Schlick reflectance formula is right.

## Step 8 — Quick diagnostic tests

Let's verify materials individually. Add to `src/material.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::hit::HitRecord;
    use rand::SeedableRng;

    fn make_hit(normal: Vec3) -> HitRecord {
        HitRecord {
            t: 1.0,
            point: Vec3::ZERO,
            normal,
            front_face: true,
            material: Lambertian::new(Vec3::ZERO),
        }
    }

    #[test]
    fn lambertian_scatters_near_normal() {
        let mat = Lambertian { albedo: Vec3::new(0.5, 0.5, 0.5) };
        let mut rng = SmallRng::seed_from_u64(42);
        let ray_in = Ray::new(Vec3::ZERO, Vec3::new(0.0, -1.0, 0.0));
        let hit = make_hit(Vec3::new(0.0, 1.0, 0.0));

        let (scattered, att) = mat.scatter(&ray_in, &hit, &mut rng).unwrap();
        // Scattered should point roughly upward
        assert!(scattered.direction.y > 0.0);
        assert_eq!(att, Vec3::new(0.5, 0.5, 0.5));
    }

    #[test]
    fn metal_reflects_correctly() {
        let mat = Metal { albedo: Vec3::new(0.8, 0.8, 0.8), fuzz: 0.0 };
        let mut rng = SmallRng::seed_from_u64(42);
        // Ray coming in at 45 degrees toward surface
        let ray_in = Ray::new(Vec3::ZERO, Vec3::new(1.0, -1.0, 0.0).normalize());
        let hit = make_hit(Vec3::new(0.0, 1.0, 0.0));

        let (scattered, _att) = mat.scatter(&ray_in, &hit, &mut rng).unwrap();
        let expected = Vec3::new(1.0, 1.0, 0.0).normalize();
        assert!((scattered.direction.normalize() - expected).length() < 1e-4);
    }

    #[test]
    fn dielectric_enters_glass() {
        let mat = Dielectric { ior: 1.5 };
        let mut rng = SmallRng::seed_from_u64(42);
        // Ray head-on (straight down) hitting water surface
        let ray_in = Ray::new(Vec3::ZERO, Vec3::new(0.0, -1.0, 0.0));
        let hit = make_hit(Vec3::new(0.0, 1.0, 0.0));

        // With head-on, Schlick reflectance is tiny, so we should usually refract.
        // But this is probabilistic, so run several iterations and check at least one refracts.
        let mut refracted_any = false;
        for _ in 0..100 {
            let (scattered, _) = mat.scatter(&ray_in, &hit, &mut rng).unwrap();
            // Refraction of head-on ray into denser medium = continues down
            if scattered.direction.y < 0.0 {
                refracted_any = true;
                break;
            }
        }
        assert!(refracted_any);
    }
}
```

Run:

```bash
cargo test material
```

All three should pass.

## Common pitfalls

### "Cannot normalize zero vector" panic

If `scatter_dir = hit.normal + Vec3::random_unit(rng)` happens to land on the antipode of the normal, their sum is near-zero. Use `near_zero` check to fall back to the normal as direction. (We did this — verify it's in your Lambertian impl.)

### Glass sphere renders as pure white or pure black

- Pure white: `front_face` logic inverted. Normals face the wrong way inside the glass. Re-check your `HitRecord::new`.
- Pure black: `refract` returning zero-length direction, or the reflectance is always triggering and rays bounce into themselves. Check `t_min = 0.001`, not 0.

### "Nothing works" with dynamic dispatch

Error: `the size for values of type `dyn Material` cannot be known at compilation time`. You stored `Material` directly instead of `Arc<dyn Material>` or `Box<dyn Material>`. Use an Arc/Box wrapper.

### Recursion stack overflow

`ray_color` is recursive. With `max_depth = 50`, each stack frame is ~few hundred bytes → a few KB, fine. If you crank depth to 10,000, you'll blow the stack. 50 is the standard for "enough for most scenes" and stays well within limits. If you need more, convert to an iterative loop.

### Gold is green

Metal albedo `(0.8, 0.6, 0.2)` is yellowish, but if it looks greenish, you might have swapped components. Always `(r, g, b)` order.

### Render takes 10x longer than expected

- You're in debug, not release. Always `--release`.
- Your trait method has a non-inlined hot path. Rust usually handles this, but if you accidentally made everything `&dyn Material` (with double-indirection), you can slow down noticeably. `Arc<dyn Material>` is fine.

### Banding in glass sphere

If the glass sphere has visible circular bands, your Schlick formula is discretizing poorly. Usually a sign that samples are too low — glass benefits from higher sample counts. Try `--samples 500`.

## What you learned

- The `Material` trait returns `Option<(Ray, Vec3)>` — scattered ray and attenuation, or `None` for absorption.
- **Lambertian** (matte): scatter via `normal + random_unit`. Cosine-weighted hemisphere by construction.
- **Metal**: perfect reflection `v - 2(v·n)n`, plus random fuzz vector scaled by roughness.
- **Dielectric** (glass): Snell's law for refraction, Schlick approximation for the Fresnel split, total internal reflection when `η_ratio * sin_θ > 1`.
- `Arc<dyn Material>` lets multiple spheres share one material object, thread-safely.
- Recursive `ray_color(depth)` with a depth limit and absorption fallback is the standard path-tracing inner loop.
- Front-face orientation matters for glass — the normal must consistently point against the incoming ray.
- Hollow glass spheres are modeled as two spheres at the same center with opposite-signed radii.

## Exercises

1. **Colored glass.** Change dielectric attenuation from `Vec3::ONE` to `(0.9, 0.8, 0.95)`. The blue channel absorbs more → a slight amber tint. Render and compare.
2. **Add a diffuse light.** Implement a fourth material, `Emissive`, whose `scatter` returns `None` but also exposes an `emit(&self) -> Vec3` method. Modify `ray_color` to accumulate emission. Add a glowing sphere and watch it illuminate the scene without an explicit light source.
3. **Randomized scene.** Generate 50 random spheres scattered on the ground plane, each with random materials (Lambertian, Metal with fuzz, Dielectric). This is Shirley's book-cover scene — slow but beautiful at 500 samples.
4. **Refraction index animation.** Render the same glass sphere 10 times with ior = 1.0, 1.1, 1.2, ..., 2.0. Stitch into a GIF. You'll see the scene progressively bending more.
5. **Russian roulette.** Replace the fixed `max_depth` with **Russian roulette termination**: at each bounce, continue with probability `p = max(attenuation.x, attenuation.y, attenuation.z)`, and scale the returned color by `1/p` to stay unbiased. Deeper bounces are cheap when attenuation is small. This is the right way to do it in production.

## What's next

Your render time just blew up. On a laptop, 800x450 at 500 samples can take several minutes. Time to parallelize. **Day 26 brings `rayon`** — the data-parallel library that turns `iter` into `par_iter` with one word. You'll split the image into per-pixel chunks and render them in parallel, with each thread using its own seeded `SmallRng` for deterministic (bit-identical!) output across thread counts. 4-16x speedup with ~20 lines of change.

→ [Day 26 — Parallel Rendering with Rayon](day-26.md)
