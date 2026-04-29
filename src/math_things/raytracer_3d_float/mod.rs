use std::{
    f32::consts::FRAC_PI_4,
    ops::{Add, Div, Mul},
};

use glam::{Vec2, Vec3, vec2, vec3};
use imageproc::image::{Rgb, RgbImage};
use perf_tracer_macros::trace_function;

/// An index of refraction
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct IOR(pub f32);

impl IOR {
    pub fn vacuum() -> Self {
        Self(1.)
    }
}

struct RayIntersection {
    /// The normal of the intersection, facing towards the incoming ray
    pub normal: Vec3,
    pub pos: Vec3,
    pub pos_below_surface: Vec3,
    pub pos_above_surface: Vec3,
    /// Distance along the ray that the intersection was at
    pub dist: f32,
    pub is_leaving: bool,
    pub object: usize,
}

/// https://en.wikipedia.org/wiki/Snell%27s_law#Vector_form
///
/// * `ray_dir` mut be a unit vector
/// * `surface_normal` mut be a unit vector
#[trace_function]
fn refract(ray_dir: Vec3, surface_normal: Vec3, leaving_ior: IOR, entering_ior: IOR) -> Vec3 {
    let r = leaving_ior.0 / entering_ior.0;
    let c = (-surface_normal).dot(ray_dir);

    let sqrt_inner = 1. - r.powi(2) * (1. - c.powi(2));
    if sqrt_inner < 0. {
        // Total internal reflection
        return ray_dir.reflect(surface_normal);
    }
    let n_mag = &r * &c - sqrt_inner.sqrt();
    r * ray_dir + n_mag * surface_normal
}

/// An sRGB color
#[derive(Debug, Clone, Copy)]
pub struct Color([f32; 3]);

impl Color {
    pub fn black() -> Self {
        Self([0.; 3])
    }

    pub fn avg(samples: impl IntoIterator<Item = Color>) -> Color {
        let mut color = Color::black();
        let mut sample_count = 0;

        for sample in samples {
            color = color + sample;
            sample_count += 1;
        }

        color / sample_count as f32
    }

    pub fn r(self) -> f32 {
        self.0[0]
    }
    pub fn g(self) -> f32 {
        self.0[1]
    }
    pub fn b(self) -> f32 {
        self.0[2]
    }
}

impl From<Rgb<u8>> for Color {
    fn from(value: Rgb<u8>) -> Self {
        Self(value.0.map(|x| x as f32 / 256.))
    }
}

impl From<Color> for Rgb<u8> {
    fn from(value: Color) -> Self {
        Rgb(value.0.map(|x| (x * 256.) as u8))
    }
}

impl Mul<f32> for Color {
    type Output = Color;

    fn mul(self, rhs: f32) -> Self::Output {
        Self(self.0.map(|x| x * rhs))
    }
}

impl Div<f32> for Color {
    type Output = Color;

    fn div(self, rhs: f32) -> Self::Output {
        self * rhs.recip()
    }
}

impl Add for Color {
    type Output = Color;

    fn add(self, rhs: Self) -> Self::Output {
        Self(std::array::from_fn(|idx| self.0[idx] + rhs.0[idx]))
    }
}

pub struct Material {
    pub color: Color,
    pub opacity: f32,
    pub ior: IOR,
}

#[derive(Debug, Clone, Copy)]
pub struct Ray {
    pub pos: Vec3,
    /// Assumed to be a unit vector, otherwise functions may return unexpected results.
    /// This will never be a soundness error, only a logic error
    pub dir: Vec3,
}

impl Ray {
    fn intersect_triangle(&self, tri: TriangleView) -> Option<RayIntersection> {
        const DEBUG: bool = false;

        // https://stackoverflow.com/questions/42740765/intersection-between-line-and-triangle-in-3d
        let e1 = tri.b() - tri.a();
        let e2 = tri.c() - tri.a();
        // The normal facing the ray origin, opposite the ray direction
        let (normal, is_leaving) = {
            let normal = Vec3::cross(e1, e2);
            if normal.dot(self.dir) <= 0. {
                (normal, false)
            } else {
                (-normal, true)
            }
        };

        let det = -Vec3::dot(self.dir, normal);
        debug_assert!(det >= 0.);

        let ao = &self.pos - tri.a();
        let dao = Vec3::cross(ao, self.dir);
        if DEBUG {
            println!("  normal={normal}; det={det}; a0={ao}");
        }

        let u = Vec3::dot(e2, dao) / &det;
        let v = -Vec3::dot(e1, dao) / &det;
        let t = Vec3::dot(ao, normal) / &det; // t is the distance along self.dir

        let intersects = det.abs() > 1e-6 && t > 0. && u >= 0. && v >= 0. && (u + v) <= 1.;

        if DEBUG {
            println!(
                "  intersects={intersects}; t={t}; u={u}; v={v}, u+v={}",
                &u + &v
            );
        }

        let pos = &self.pos + &self.dir * &t;

        intersects.then(move || RayIntersection {
            pos,
            pos_below_surface: pos - normal * 1e-4,
            pos_above_surface: pos + normal * 1e-4,
            dist: t,
            normal,
            is_leaving,
            object: usize::MAX,
        })
    }

    fn intersect_sphere(&self, sphere: &Sphere) -> Option<RayIntersection> {
        // Translate by `-sphere.center` then scale by `1 / sphere.radius`
        // Both operations only affect the ray position, so `dir` doesn't need to be changed
        // however, the returned `dist` will need to be scaled back

        let ray = Ray {
            pos: (self.pos - sphere.center) / sphere.radius,
            dir: self.dir,
        };

        let intersection = ray.intersect_unit_sphere()?;

        Some(RayIntersection {
            normal: intersection.normal,
            pos: intersection.pos * sphere.radius + sphere.center,
            pos_below_surface: intersection.pos_below_surface * sphere.radius + sphere.center,
            pos_above_surface: intersection.pos_above_surface * sphere.radius + sphere.center,
            dist: intersection.dist * sphere.radius,
            is_leaving: intersection.is_leaving,
            object: intersection.object,
        })
    }

    fn intersect_unit_sphere(&self) -> Option<RayIntersection> {
        // Derived from `x^2 + y^2 + z^2 = 0` and `(x, y, z) = pos + dir * t`,
        // solving for t. This creates a quadratic for t with the coefficients:
        // a = sqr_magnitude(dir)
        // b = 2 * dot(pos, dir)
        // c = sqr_magnitude(pos) - 1
        //
        // Since `self.dir` is expected to be normalized, `a = 1` always holds
        let [t_lo, t_hi] = quadratic_solve(
            1.,
            2. * self.pos.dot(self.dir),
            self.pos.length_squared() - 1.,
        )?;

        if t_lo < 0. && t_hi < 0. {
            return None;
        }

        let (t, is_leaving) = if t_lo < 0. {
            (t_hi, true)
        } else {
            (t_lo, false)
        };

        let pos = self.pos + self.dir * t;
        let pos = pos.normalize();
        // `pos` lies on the unit sphere
        debug_assert!(
            (pos.length() - 1.).abs() <= 1e-6,
            "intersection={pos}; pos={}; dir={}; t={}",
            self.pos,
            self.dir,
            t
        );

        let normal = if is_leaving { -pos } else { pos };

        Some(RayIntersection {
            normal,
            pos,
            pos_below_surface: pos * (1. - 1e-4),
            pos_above_surface: pos * (1. + 1e-4),
            dist: t,
            is_leaving,
            object: usize::MAX,
        })
    }
}

pub struct Sphere {
    pub center: Vec3,
    pub radius: f32,
}

struct TriangleView<'mesh> {
    tri: usize,
    vertices: [Vec3; 3],
    mesh: &'mesh Mesh,
}

impl<'mesh> TriangleView<'mesh> {
    pub fn a(&self) -> Vec3 {
        self.vertices[0]
    }
    pub fn b(&self) -> Vec3 {
        self.vertices[1]
    }
    pub fn c(&self) -> Vec3 {
        self.vertices[2]
    }
}

pub struct Mesh {
    pub vertices: Vec<Vec3>,
    pub tris: Vec<[u32; 3]>,
}

pub enum Object {
    Sphere(Sphere, Material),
    Mesh(Mesh, Material),
}

impl Object {
    fn material(&self) -> &Material {
        match self {
            Object::Sphere(_, material) | Object::Mesh(_, material) => material,
        }
    }
}

pub struct PointLight {
    pub pos: Vec3,
    pub intensity: f32,
}

pub enum Light {
    Point(PointLight),
}

pub struct Scene {
    sky_color: Color,
    sky_ior: IOR,
    objects: Vec<Object>,
    lights: Vec<Light>,
    ambient_light: f32,
}

pub struct RenderCfg {
    pub width: u32,
    pub height: u32,
    pub fov_w: f32,
    pub fov_h: f32,
    pub anti_aliasing: bool,
}

impl Scene {
    #[trace_function]
    fn intersect_ray(&self, ray: Ray) -> Option<RayIntersection> {
        let mut nearest: Option<RayIntersection> = None;

        for (obj_idx, obj) in self.objects.iter().enumerate() {
            let mut update_nearest = |maybe_nearest: Option<RayIntersection>| {
                if let Some(mut intersection) = maybe_nearest
                    && nearest
                        .as_ref()
                        .is_none_or(|nearest| intersection.dist < nearest.dist)
                {
                    intersection.object = obj_idx;
                    nearest = Some(intersection);
                }
            };
            match obj {
                Object::Sphere(sphere, _) => update_nearest(ray.intersect_sphere(sphere)),
                Object::Mesh(mesh, _) => {
                    for (tri, vertices) in mesh.tris.iter().copied().enumerate() {
                        let tri = TriangleView {
                            tri,
                            vertices: vertices.map(|vtx| mesh.vertices[vtx as usize]),
                            mesh,
                        };

                        update_nearest(ray.intersect_triangle(tri));
                    }
                }
            };
        }

        nearest
    }

    #[trace_function]
    fn cast_ray_inner(&self, cfg: &RenderCfg, ray: Ray, bounces_rem: u32, curr_ior: IOR) -> Color {
        if bounces_rem == 0 {
            return self.sky_color;
        }

        // Cast initial ray
        // Based on intersection, cast any sub-rays
        // Rays:
        // * Refraction
        // * Reflection
        // * Shadow (from surface towards lights)

        let Some(hit) = self.intersect_ray(ray) else {
            return self.sky_color;
        };
        let object = &self.objects[hit.object];

        // Diffuse part
        let diffuse_color = {
            // FIXME: Lights that shine through transparent materials don't work correctly
            // maybe compute a lightmap by casting from each light?

            // Ambient lighting
            // Light lighting
            let light_intensity = self.ambient_light
                + self
                    .lights
                    .iter()
                    .flat_map(|light| match light {
                        Light::Point(point_light) => {
                            let delta = point_light.pos - hit.pos_above_surface;
                            let ray_to_light = Ray {
                                pos: hit.pos_above_surface,
                                dir: delta.normalize(),
                            };
                            self.intersect_ray(ray_to_light)
                                .is_none_or(|intersection| intersection.dist > delta.length())
                                .then(|| point_light.intensity / delta.length().powi(2))
                        }
                    })
                    .sum::<f32>();
            // FIXME: Inaccurate conversion from light intensity to RGB values
            // https://en.wikipedia.org/wiki/Relative_luminance
            // https://en.wikipedia.org/wiki/Lumen_(unit)#Lighting
            let factor = 1. - (1. / (1. + light_intensity / 100.));
            object.material().color * factor
        };

        // Refraction part
        let refracted_color = if object.material().opacity >= 1. {
            Color::black()
        } else {
            // FIXME: Allow intersecting objects by tracking iors with a stack
            let new_ior = match hit.is_leaving {
                true => self.sky_ior,
                false => object.material().ior,
            };
            let dir = refract(ray.dir, hit.normal, curr_ior, new_ior);

            let ray = Ray {
                pos: hit.pos_below_surface,
                dir,
            };
            self.cast_ray_inner(cfg, ray, bounces_rem - 1, new_ior)
        };

        diffuse_color * object.material().opacity
            + refracted_color * (1. - object.material().opacity)
    }

    #[trace_function]
    fn cast_ray(&self, cfg: &RenderCfg, ray: Ray, max_bounces: u32) -> Color {
        self.cast_ray_inner(cfg, ray, max_bounces, self.sky_ior)
    }

    #[trace_function]
    pub fn render(&self, cfg: &RenderCfg) -> RgbImage {
        let mut img = RgbImage::new(cfg.width, cfg.height);

        let pixels = (0..cfg.width).flat_map(|x| (0..cfg.height).map(move |y| (x, y)));

        // Returns `(elevation, azimuth)`
        let pixel_to_spherical = |pixel: Vec2| -> Vec2 {
            vec2(
                (0.5 * cfg.height as f32 - pixel.y) / cfg.height as f32 * cfg.fov_h,
                (pixel.x - 0.5 * cfg.width as f32) / cfg.width as f32 * cfg.fov_w,
            )
        };

        for pixel in pixels {
            let pixelf = vec2(pixel.0 as f32, pixel.1 as f32);
            // Spatial anti-aliasing by emitting multiple rays per pixel and averaging them

            let samples = if cfg.anti_aliasing {
                [
                    pixelf,
                    pixelf + vec2(0.2, 0.),
                    pixelf + vec2(0., 0.2),
                    pixelf + vec2(-0.2, 0.),
                    pixelf + vec2(0., -0.2),
                ]
                .to_vec()
            } else {
                [pixelf].to_vec()
            };

            let samples = samples.into_iter().map(|sample_pxf| {
                let ray = Ray {
                    pos: Vec3::splat(0.),
                    dir: Vec3::from_spherical_coords_vec2(pixel_to_spherical(sample_pxf)),
                };
                self.cast_ray(cfg, ray, 16)
            });

            let anti_aliased = Color::avg(samples);

            img[pixel] = anti_aliased.into();
        }

        img
    }
}

trait GlamVec3Ext: Sized {
    fn from_spherical_coords(elevation: f32, azimuth: f32) -> Self;
    fn from_spherical_coords_vec2(elev_azim: Vec2) -> Self {
        Self::from_spherical_coords(elev_azim.x, elev_azim.y)
    }
}

impl GlamVec3Ext for Vec3 {
    fn from_spherical_coords(elevation: f32, azimuth: f32) -> Self {
        let inclination = std::f32::consts::FRAC_PI_2 - elevation;
        Vec3::new(
            inclination.sin() * azimuth.sin(),
            inclination.cos(),
            inclination.sin() * azimuth.cos(),
        )
    }
}

/// Solves `ax^2 + bx + c = 0` for `x`, returning `Some([lo_solution, hi_solution])`
/// if the solution exists
fn quadratic_solve(a: f32, b: f32, c: f32) -> Option<[f32; 2]> {
    let det = b * b - 4. * a * c;
    if det < 0. {
        return None;
    }
    let det_sqrt = det.sqrt();

    let inv_2a = (2. * a).recip();

    Some([(-b - det_sqrt) * inv_2a, (-b + det_sqrt) * inv_2a])
}

#[trace_function]
pub fn start() {
    let scene = Scene {
        sky_color: Color([0.5; 3]),
        sky_ior: IOR::vacuum(),
        objects: [
            Object::Mesh(
                Mesh {
                    vertices: [vec3(0., 0., 5.), vec3(0.7, 1., 5.), vec3(1., -0.2, 5.)].into(),
                    tris: [[0, 1, 2]].into(),
                },
                Material {
                    color: Color([0.6, 0.4, 0.4]),
                    opacity: 0.8,
                    ior: IOR(1.2),
                },
            ),
            Object::Mesh(
                Mesh {
                    vertices: [vec3(0.2, 0., 10.), vec3(0.7, 1., 10.), vec3(1., -1., 10.)].into(),
                    tris: [[0, 1, 2]].into(),
                },
                Material {
                    color: Color([0.1, 0.4, 0.8]),
                    opacity: 0.8,
                    ior: IOR::vacuum(),
                },
            ),
            Object::Sphere(
                Sphere {
                    center: vec3(0., 1., 15.),
                    radius: 2.,
                },
                Material {
                    color: Color([0.3, 0.8, 0.3]),
                    opacity: 1.,
                    ior: IOR::vacuum(),
                },
            ),
        ]
        .into(),
        ambient_light: 10.,
        lights: [Light::Point(PointLight {
            pos: vec3(0., 1., 1.),
            intensity: 1000.,
        })]
        .into(),
    };

    let cfg = RenderCfg {
        width: 1024,
        height: 512,
        fov_w: FRAC_PI_4,
        fov_h: FRAC_PI_4 / 2.,
        anti_aliasing: false,
    };

    let img = scene.render(&cfg);

    img.save("outputs/raytracer_3d_float_result.png").unwrap();
}
