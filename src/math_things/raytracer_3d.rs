use std::{
    f64::consts::FRAC_PI_4,
    ops::{Add, Mul, Neg},
    sync::mpsc,
};

use imageproc::image::{self, Rgb, RgbImage};
use minifb::{Key, Window, WindowOptions};
use perf_tracer_macros::trace_function;
use rayon::iter::{ParallelBridge, ParallelIterator};

use crate::math_things::{
    ge_or_gt, le_or_lt,
    rational::{IRat, Precision},
    vec3::Vec3,
};

mod intersect_3d {
    use std::ops::Neg;

    use perf_tracer_macros::trace_function;

    use crate::math_things::{
        rational::{IRat, Precision},
        vec3::Vec3,
    };

    /// https://en.wikipedia.org/wiki/Snell%27s_law#Vector_form
    ///
    /// * `ray_dir` mut be a unit vector
    /// * `surface_normal` mut be a unit vector
    #[trace_function]
    pub fn refract(
        ray_dir: &Vec3,
        surface_normal: &Vec3,
        leaving_ior: &IRat,
        entering_ior: &IRat,
    ) -> Vec3 {
        let r = leaving_ior / entering_ior;
        let c = surface_normal.clone().neg().dot(ray_dir);

        let sqrt_inner = IRat::one() - r.powi(2) * (IRat::one() - c.powi(2));
        if sqrt_inner < IRat::zero() {
            // Total internal reflection
            return ray_dir.reflected(surface_normal);
        }
        let n_mag = &r * &c - sqrt_inner.sqrt(Precision(64));
        r * ray_dir + n_mag * surface_normal
    }
}

#[derive(Debug, Clone)]
pub struct Intersection3d {
    pub pos: Vec3,
    pub sqr_dist: IRat,
    /// The normal, adjusted to point towards the ray
    pub normal: Vec3,
    /// Whether or not the
    pub is_leaving: bool,
}

#[derive(Debug, Clone)]
pub struct Ray3 {
    pub pos: Vec3,
    /// Expected to be approximately normalized
    pub dir: Vec3,
}

impl Ray3 {
    /// Returns this ray advanced by `dist` multiples of `self.dir`
    #[must_use]
    fn advanced_by(&self, dist: &IRat) -> Vec3 {
        &self.pos + &self.dir * dist
    }

    fn intersect_triangle(
        &self,
        tri: TriangleView,
        include_ray_start: bool,
    ) -> Option<Intersection3d> {
        const DEBUG: bool = false;

        // https://stackoverflow.com/questions/42740765/intersection-between-line-and-triangle-in-3d
        let e1 = tri.b() - tri.a();
        let e2 = tri.c() - tri.a();
        // The normal facing the ray origin, opposite the ray direction
        let (normal, is_leaving) = {
            let normal = Vec3::cross(&e1, &e2);
            if normal.dot(&self.dir) <= IRat::zero() {
                (normal, false)
            } else {
                (-normal, true)
            }
        };

        let det = -Vec3::dot(&self.dir, &normal);
        debug_assert!(det >= IRat::zero());
        if det == IRat::zero() {
            // The plane and the ray are perfectly parallel
            if DEBUG {
                println!("  det=0");
            }
            return None;
        }

        let ao = &self.pos - tri.a();
        let dao = Vec3::cross(&ao, &self.dir);
        if DEBUG {
            println!("  normal={normal}; det={det}; a0={ao}");
        }

        let u = Vec3::dot(&e2, &dao) / &det;
        let v = -Vec3::dot(&e1, &dao) / &det;
        let t = Vec3::dot(&ao, &normal) / &det; // t is the distance along self.dir

        let intersects = ge_or_gt(&t, &IRat::zero(), include_ray_start)
            && u >= IRat::zero()
            && v >= IRat::zero()
            && (&u + &v) <= IRat::one();

        if DEBUG {
            println!(
                "  intersects={intersects}; t={t}; u={u}; v={v}, u+v={}",
                &u + &v
            );
        }

        intersects.then(move || Intersection3d {
            pos: &self.pos + &self.dir * &t,
            sqr_dist: self.dir.sqr_magnitude() * t,
            normal,
            is_leaving,
        })
    }
}

#[derive(Debug, Clone)]
pub struct Mesh3d {
    /// From 0 to 1
    pub opacity: f64,
    pub color: Rgb<u8>,
    pub index_of_refraction: IRat,
    pub vtx: Vec<Vec3>,
    pub tris: Vec<[usize; 3]>,
}

impl Mesh3d {
    pub fn triangle(&self, idx: usize) -> TriangleView<'_> {
        TriangleView {
            idx,
            vertices: self.tris[idx],
            mesh: &self,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct TriangleView<'mesh> {
    idx: usize,
    vertices: [usize; 3],
    mesh: &'mesh Mesh3d,
}

impl<'mesh> TriangleView<'mesh> {
    pub fn a(&self) -> &Vec3 {
        &self.mesh.vtx[self.vertices[0]]
    }

    pub fn b(&self) -> &Vec3 {
        &self.mesh.vtx[self.vertices[1]]
    }

    pub fn c(&self) -> &Vec3 {
        &self.mesh.vtx[self.vertices[2]]
    }
}

#[derive(Debug, Clone)]
pub struct Triangle3d {
    vertices: [Vec3; 3],
}

impl Triangle3d {
    pub fn a(&self) -> &Vec3 {
        &self.vertices[0]
    }

    pub fn b(&self) -> &Vec3 {
        &self.vertices[1]
    }

    pub fn c(&self) -> &Vec3 {
        &self.vertices[2]
    }

    pub fn normal(&self) -> Vec3 {
        Vec3::cross(&(self.b() - self.a()), &(self.c() - self.a()))
    }
}

/// An sRGB color
#[derive(Debug, Clone, Copy)]
pub struct Color([f64; 3]);

impl Color {
    pub fn r(self) -> f64 {
        self.0[0]
    }
    pub fn g(self) -> f64 {
        self.0[1]
    }
    pub fn b(self) -> f64 {
        self.0[2]
    }
}

impl From<Rgb<u8>> for Color {
    fn from(value: Rgb<u8>) -> Self {
        Self(value.0.map(|x| x as f64 / 256.))
    }
}

impl From<Color> for Rgb<u8> {
    fn from(value: Color) -> Self {
        Rgb(value.0.map(|x| (x * 256.) as u8))
    }
}

impl Mul<f64> for Color {
    type Output = Color;

    fn mul(self, rhs: f64) -> Self::Output {
        Self(self.0.map(|x| x * rhs))
    }
}

impl Add for Color {
    type Output = Color;

    fn add(self, rhs: Self) -> Self::Output {
        Self(std::array::from_fn(|idx| self.0[idx] + rhs.0[idx]))
    }
}

#[derive(Debug, Clone)]
pub struct Scene3d {
    sky_ior: IRat,
    sky_color: Color,
    meshes: Vec<Mesh3d>,
}

pub struct RaycastResult {
    pub color: Rgb<u8>,
}

impl Scene3d {
    pub fn cast_ray(&self, ray: Ray3, max_bounces: usize) -> Color {
        self.cast_ray_inner(ray, self.sky_ior.clone(), max_bounces)
    }

    fn cast_ray_inner(&self, ray: Ray3, curr_ior: IRat, rem_bounces: usize) -> Color {
        if rem_bounces == 0 {
            return self.sky_color;
        }

        let mut closest_intersection: Option<(usize, Intersection3d)> = None;
        for (mesh_idx, mesh) in self.meshes.iter().enumerate() {
            for tri in 0..mesh.tris.len() {
                let tri_view = mesh.triangle(tri);
                if let Some(intersection) = ray.intersect_triangle(tri_view, false) {
                    if closest_intersection
                        .as_ref()
                        .is_none_or(|(_, prev)| prev.sqr_dist > intersection.sqr_dist)
                    {
                        closest_intersection = Some((mesh_idx, intersection));
                    }
                }
            }
        }

        let Some((hit_mesh, intersection)) = closest_intersection else {
            return self.sky_color;
        };

        let hit_mesh = &self.meshes[hit_mesh];

        // FIXME: doesn't account for overlapping objects
        let new_ior = if intersection.is_leaving {
            self.sky_ior.clone()
        } else {
            hit_mesh.index_of_refraction.clone()
        };

        // color = diffuse_portion * opacity + refracted_portion * (1 - opacity)

        let refracted_portion = if hit_mesh.opacity < 0.999 {
            let new_ray = Ray3 {
                pos: intersection.pos.clone(),
                dir: intersect_3d::refract(&ray.dir, &intersection.normal, &curr_ior, &new_ior),
            };
            self.cast_ray_inner(new_ray, new_ior, rem_bounces - 1)
        } else {
            Color([0.; 3])
        };

        let diffuse_portion = Color::from(hit_mesh.color);

        diffuse_portion * hit_mesh.opacity + refracted_portion * (1. - hit_mesh.opacity)
    }

    /// How to use:
    /// ```
    /// let scene: Scene3d;
    /// let settings: RenderSettings;
    /// let mut img;
    /// for (pixel, color) in scene.render(&settings) {
    ///     img.set_pixel(pixel, color);
    /// }
    /// ```
    fn render(&self, settings: &RenderSettings) -> mpsc::Receiver<((u32, u32), Rgb<u8>)> {
        let (finished_tx, finished_rx) = mpsc::channel();
        let this: Scene3d = self.clone();
        let settings: RenderSettings = settings.clone();
        std::thread::spawn(move || this.render_blocking(&settings, finished_tx));
        finished_rx
    }

    fn render_blocking(
        &self,
        settings: &RenderSettings,
        finished_tx: mpsc::Sender<((u32, u32), Rgb<u8>)>,
    ) {
        let pixels_iter = (0..settings.width)
            .flat_map(|x| (0..settings.height).map(move |y| (x, y)))
            .par_bridge();
        pixels_iter.for_each_with(finished_tx, |finished_tx, pixel| {
            let ray = Ray3 {
                pos: Vec3::splat(0),
                dir: Vec3::from_spherical_coords_inexact(
                    // The screen coordinates start at the top, so the elevation starts at the top and decreases
                    (0.5 * settings.height as f64 - pixel.1 as f64) / settings.height as f64
                        * settings.fov_h,
                    (pixel.0 as f64 - 0.5 * settings.width as f64) / settings.width as f64
                        * settings.fov_w,
                ),
            };

            let final_color: Rgb<u8> = self.cast_ray(ray, 4).into();
            if let Err(_) = finished_tx.send((pixel, final_color)) {
                return;
            }
        });
    }
}

#[derive(Debug, Clone)]
pub struct RenderSettings {
    pub width: u32,
    pub height: u32,
    pub fov_w: f64,
    pub fov_h: f64,
}

#[trace_function]
pub fn start(write_to_window: bool) {
    let scene = Scene3d {
        sky_ior: IRat::one(),
        sky_color: Color([0.5; 3]),
        meshes: [
            Mesh3d {
                opacity: 0.8,
                color: Rgb([180, 100, 100]),
                index_of_refraction: IRat::from(1.2),
                vtx: [
                    Vec3::new(0, 0, 5),
                    Vec3::new(0.7, 1, 5),
                    Vec3::new(1, -0.2, 5),
                ]
                .into(),
                tris: [[0, 1, 2]].into(),
            },
            Mesh3d {
                opacity: 0.8,
                color: Rgb([0, 0, 255]),
                index_of_refraction: IRat::from(1.),
                vtx: [
                    Vec3::new(0.2, 0, 10),
                    Vec3::new(0.7, 1, 10),
                    Vec3::new(1, -1., 10),
                ]
                .into(),
                tris: [[0, 1, 2]].into(),
            },
        ]
        .into(),
    };

    let w = 128;
    let settings = RenderSettings {
        width: w,
        height: w / 2,
        fov_w: FRAC_PI_4,
        fov_h: FRAC_PI_4 / 2.,
    };

    let mut img = RgbImage::from_pixel(settings.width, settings.height, Rgb([0; 3]));

    match write_to_window {
        true => {
            let pixels_rx = scene.render(&settings);
            let window_width = 1024;
            let window_height = (window_width * settings.height) / settings.width;

            let mut window = Window::new(
                "Render Progress",
                window_width as usize,
                window_height as usize,
                WindowOptions::default(),
            )
            .unwrap();
            window.set_target_fps(30);

            'outer: while window.is_open() && !window.is_key_down(Key::Escape) {
                loop {
                    let (pixel, color) = match pixels_rx.try_recv() {
                        Ok(x) => x,
                        Err(mpsc::TryRecvError::Empty) => break,
                        Err(mpsc::TryRecvError::Disconnected) => break 'outer,
                    };
                    *img.get_pixel_mut(pixel.0, pixel.1) = color;
                }

                /// From `minifb::Window::update_with_buffer` example
                fn from_u8_rgb(r: u8, g: u8, b: u8) -> u32 {
                    let (r, g, b) = (r as u32, g as u32, b as u32);
                    (r << 16) | (g << 8) | b
                }

                let buffer = image::imageops::resize(
                    &img,
                    window_width,
                    window_height,
                    image::imageops::FilterType::Nearest,
                );
                let buffer = buffer
                    .pixels()
                    .map(|px| from_u8_rgb(px[0], px[1], px[2]))
                    .collect::<Vec<_>>();

                window
                    .update_with_buffer(&buffer, window_width as usize, window_height as usize)
                    .unwrap();
            }
        }
        false => {
            for (pixel, color) in scene.render(&settings) {
                *img.get_pixel_mut(pixel.0, pixel.1) = color;
            }
        }
    }

    img.save("raytracer_3d_result.png").unwrap();
}
