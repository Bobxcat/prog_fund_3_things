use imageproc::{
    definitions::Image,
    image::{Rgb, RgbImage},
};
use perf_tracer::print_trace_time;
use perf_tracer_macros::trace_function;

use crate::math_things::{
    mat2::Mat2,
    rational::{IRat, Precision},
    vec2::Vec2,
};

const PREC: Precision = Precision(128);

mod intersect2d {
    use perf_tracer_macros::trace_function;

    use crate::math_things::{
        le_or_lt, mat2::Mat2, rational::IRat, raytracer_2d::PREC, vec2::Vec2,
    };

    /// Returns the position of a line-line intersection
    ///
    /// This uses:
    /// https://stackoverflow.com/questions/4977491/determining-if-two-line-segments-intersect
    ///
    /// A different solution:
    /// https://stackoverflow.com/questions/563198/how-do-you-detect-where-two-line-segments-intersect
    #[inline]
    #[trace_function]
    pub fn intersect_lines(
        a_start: &Vec2,
        a_dir: &Vec2,
        b_start: &Vec2,
        b_dir: &Vec2,
        a_start_included: bool,
        a_end_included: bool,
        b_start_included: bool,
        b_end_included: bool,
    ) -> Option<Vec2> {
        if a_start == b_start {
            return Some(a_start.clone());
        }
        let v = a_start - b_start;
        let mat = Mat2::new([
            b_dir.x.clone(),
            a_dir.x.clone(),
            b_dir.y.clone(),
            a_dir.y.clone(),
        ]);
        let mat_inv = mat.inverse()?;

        let Vec2 { x: s, y: neg_t } = mat_inv * v;
        let t = -neg_t;

        if le_or_lt(&IRat::zero(), &t, a_start_included)
            && le_or_lt(&t, &IRat::one(), a_end_included)
            && le_or_lt(&IRat::zero(), &s, b_start_included)
            && le_or_lt(&s, &IRat::one(), b_end_included)
        {
            Some(a_start + t * a_dir)
        } else {
            None
        }
    }

    // pub struct RefractionResult

    /// https://en.wikipedia.org/wiki/Snell%27s_law#Vector_form
    ///
    /// * `ray_dir` mut be a unit vector
    /// * `surface_normal` mut be a unit vector
    #[trace_function]
    pub fn refract_ray(
        ray_dir: &Vec2,
        surface_normal: &Vec2,
        leaving_ior: &IRat,
        entering_ior: &IRat,
    ) -> Vec2 {
        let r = leaving_ior / entering_ior;
        let c = surface_normal.clone().negated().dot(ray_dir);

        let sqrt_inner = IRat::one() - r.powi(2) * (IRat::one() - c.powi(2));
        if sqrt_inner < IRat::zero() {
            // Total internal reflection
            return ray_dir.reflected(surface_normal);
        }
        let n_mag = &r * &c - sqrt_inner.sqrt(PREC);
        r * ray_dir + n_mag * surface_normal
    }
}

#[allow(unused)]
mod ior {
    use crate::math_things::rational::{IRat, URat};

    pub fn vacuum() -> IRat {
        IRat::one()
    }

    pub fn water() -> IRat {
        IRat::from(URat::new(4u64, 3u64))
    }

    pub fn olive_oil() -> IRat {
        IRat::from(URat::new(147u64, 100u64))
    }

    pub fn cubic_zirconia() -> IRat {
        IRat::from(URat::new(215u64, 100u64))
    }

    pub fn diamond() -> IRat {
        IRat::from(URat::new(2417u64, 1000u64))
    }

    pub fn moissanite() -> IRat {
        IRat::from(URat::new(265u64, 100u64))
    }
}

#[derive(Debug, Clone)]
pub struct Ray2 {
    pub pos: Vec2,
    pub dir: Vec2,
}

impl Ray2 {
    fn normal_rhs(&self) -> Vec2 {
        Mat2::rotation_270() * &self.dir
    }

    /// Returns this ray advanced by `dist` multiples of `self.dir`
    #[must_use]
    fn advanced_by(&self, dist: &IRat) -> Vec2 {
        &self.pos + &self.dir * dist
    }
}

/// A boundary between indices of refraction
#[derive(Debug, Clone)]
pub struct Boundary {
    /// The placement and size of the boundary
    pub placement: Ray2,
    pub lhs_ior: IRat,
    pub rhs_ior: IRat,
}

/// All coordinates in range 0 to 1
///
/// Raymarch vs raycast:
/// * Raymarching gets infinitely stuck going against a circle
/// *
pub struct Scene {
    light: Vec2,
    boundaries: Vec<Boundary>,
}

struct RayFinished {
    final_pos: Vec2,
}

impl Scene {
    #[trace_function("Scene::step_ray")]
    fn step_ray(&self, ray: &Ray2) -> Result<Ray2, RayFinished> {
        struct IntersectionInfo {
            boundary: usize,
            pos: Vec2,
            dist: IRat,
        }
        let mut nearest_intersection: Option<IntersectionInfo> = None;
        for (boundary_idx, boundary) in self.boundaries.iter().enumerate() {
            if let Some(pos) = intersect2d::intersect_lines(
                &ray.pos,
                &(&ray.dir * IRat::from(0b100000000)),
                &boundary.placement.pos,
                &boundary.placement.dir,
                false,
                true,
                true,
                true,
            ) {
                let dist = pos.sqr_dist(&ray.pos);
                if nearest_intersection
                    .as_ref()
                    .is_none_or(|prev_intersection| prev_intersection.dist > dist)
                {
                    nearest_intersection = Some(IntersectionInfo {
                        boundary: boundary_idx,
                        pos,
                        dist,
                    });
                }
            }
        }

        if let Some(intersection) = nearest_intersection {
            let boundary = &self.boundaries[intersection.boundary];
            let normal_rhs = boundary.placement.normal_rhs();
            let coming_from_lhs = normal_rhs.dot(&ray.dir) >= IRat::zero();

            let (leaving_ior, entering_ior, aligned_normal) = match coming_from_lhs {
                true => (&boundary.lhs_ior, &boundary.rhs_ior, normal_rhs.negated()),
                false => (&boundary.rhs_ior, &boundary.lhs_ior, normal_rhs),
            };
            let new_dir = intersect2d::refract_ray(
                &ray.dir.normalized(PREC),
                &aligned_normal.normalized(PREC),
                leaving_ior,
                entering_ior,
            );

            Ok(Ray2 {
                pos: intersection.pos,
                dir: new_dir,
            })
        } else {
            Err(RayFinished {
                final_pos: ray.advanced_by(&IRat::from(0b1000)),
            })
        }
    }
}

pub fn start() {
    let ior_a = ior::vacuum();
    let ior_b = ior::water();
    let ior_c = ior::olive_oil();
    render_scene(&Scene {
        light: Vec2::new(0.5, 0.5),
        boundaries: [
            Boundary {
                placement: Ray2 {
                    pos: Vec2::new(0.3, 0.),
                    dir: Vec2::new(0.1, 1.),
                },
                lhs_ior: ior_a.clone(),
                rhs_ior: ior_b.clone(),
            },
            Boundary {
                placement: Ray2 {
                    pos: Vec2::new(0.51, 0.),
                    dir: Vec2::new(0.1, 1.),
                },
                lhs_ior: ior_b.clone(),
                rhs_ior: ior_c.clone(),
            },
            Boundary {
                placement: Ray2 {
                    pos: Vec2::new(0.7, 0.),
                    dir: Vec2::new(0.1, 1.),
                },
                lhs_ior: ior_c.clone(),
                rhs_ior: ior_a.clone(),
            },
        ]
        .to_vec(),
    });
}

#[trace_function]
fn draw_line_segment_mut(img: &mut RgbImage, start: (i32, i32), end: (i32, i32), color: [u8; 3]) {
    imageproc::drawing::draw_antialiased_line_segment_mut(
        img,
        start,
        end,
        Rgb(color),
        imageproc::pixelops::interpolate,
    );
}

#[trace_function]
pub fn render_scene(scene: &Scene) {
    render_scene_inner(scene)
}

fn render_scene_inner(scene: &Scene) {
    let pixels = 1024;

    let mut img: RgbImage = Image::new(pixels, pixels);
    let ray_ct = 64;

    let scene2img = IRat::from(pixels);

    macro_rules! draw_line {
        ($start:expr, $end:expr, $col:expr) => {
            perf_tracer::trace_op("draw_line", || {
                let start = &$start;
                let end = &$end;
                let a = start.clone().with_y(IRat::one() - &start.y) * &scene2img;
                let b = end.clone().with_y(IRat::one() - &end.y) * &scene2img;
                draw_line_segment_mut(&mut img, a.to_i32s(), b.to_i32s(), $col);
            })
        };
    }

    for ray_idx in 0..ray_ct {
        // let dir = Vec2::new(0.1, (ray_idx as f64 - ray_ct as f64 / 2.) / 100.);
        let angle = 2. * std::f64::consts::PI * ray_idx as f64 / ray_ct as f64;
        let dir = Vec2::new(angle.cos(), angle.sin());
        let mut ray = Ray2 {
            pos: scene.light.clone(),
            dir,
        };

        for _ in 0..4 {
            match scene.step_ray(&ray) {
                Ok(new_ray) => {
                    draw_line!(ray.pos, new_ray.pos, [255; 3]);
                    ray = new_ray;
                }
                Err(finished) => {
                    // draw_line!(ray.pos, finished.final_pos, [128, 255, 128]);
                    draw_line!(ray.pos, finished.final_pos, [255; 3]);
                    break;
                }
            }
        }
    }

    for boundary in &scene.boundaries {
        draw_line!(
            boundary.placement.pos,
            &boundary.placement.dir + &boundary.placement.pos,
            [255, 128, 128]
        );
    }

    img.save("raytracer_2d_result.png").unwrap();

    print_trace_time(&perf_tracer::PrintOpts::default());
}
