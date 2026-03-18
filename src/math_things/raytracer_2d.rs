use imageproc::{
    definitions::Image,
    image::{Rgb, RgbImage},
};

use crate::math_things::{
    mat2::Mat2,
    rational::{IRat, URat},
    vec2::Vec2,
};

const PREC: u32 = 128;

mod intersect2d {
    use crate::math_things::{
        mat2::Mat2,
        rational::{IRat, URat},
        raytracer_2d::PREC,
        vec2::Vec2,
    };

    /// Returns the position of a line-line intersection
    ///
    /// This uses:
    /// https://stackoverflow.com/questions/4977491/determining-if-two-line-segments-intersect
    ///
    /// A different solution:
    /// https://stackoverflow.com/questions/563198/how-do-you-detect-where-two-line-segments-intersect
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

        /// Computes `lower < upper` if `closed == false`,
        /// or `lower <= upper` if `closed == true`
        fn ge_or_gt(lower: &IRat, upper: &IRat, closed: bool) -> bool {
            match closed {
                true => lower <= upper,
                false => lower < upper,
            }
        }

        if ge_or_gt(&IRat::zero(), &t, a_start_included)
            && ge_or_gt(&t, &IRat::one(), a_end_included)
            && ge_or_gt(&IRat::zero(), &s, b_start_included)
            && ge_or_gt(&s, &IRat::one(), b_end_included)
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
    pub fn refract_ray(
        ray_dir: &Vec2,
        surface_normal: &Vec2,
        leaving_ior: &IRat,
        entering_ior: &IRat,
    ) -> Vec2 {
        let r = leaving_ior / entering_ior;
        let c = surface_normal.clone().negated().dot(ray_dir);

        let sqrt_inner = IRat::one() - r.powi(2) * (IRat::one() - c.powi(2));
        assert!(sqrt_inner >= IRat::zero());

        let n_mag = &r * &c - sqrt_inner.sqrt(&URat::from_sig_figs(PREC));
        r * ray_dir + n_mag * surface_normal
    }

    // pub fn intersect_line_aabb(line_start: &Vec2, line_dir: &Vec2) -> {
    //     //
    // }
}

/// Axis-aligned bounding box
pub struct Aabb {
    top_left: Vec2,
    bot_right: Vec2,
}

pub struct Ray {
    pub pos: Vec2,
    pub dir: Vec2,
}

impl Ray {
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
pub struct Boundary {
    /// The placement and size of the boundary
    pub placement: Ray,
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
    fn step_ray(&self, ray: &Ray) -> Result<Ray, RayFinished> {
        struct IntersectionInfo {
            boundary: usize,
            pos: Vec2,
            dist: IRat,
        }
        let mut nearest_intersection: Option<IntersectionInfo> = None;
        for (boundary_idx, boundary) in self.boundaries.iter().enumerate() {
            if let Some(pos) = intersect2d::intersect_lines(
                &ray.pos,
                &ray.dir,
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
                false => (&boundary.rhs_ior, &boundary.rhs_ior, normal_rhs),
            };

            let new_dir = intersect2d::refract_ray(
                &ray.dir.normalized(&URat::from_sig_figs(PREC)),
                &aligned_normal.normalized(&URat::from_sig_figs(PREC)),
                leaving_ior,
                entering_ior,
            );

            Ok(Ray {
                pos: intersection.pos,
                dir: new_dir,
            })
        } else {
            Err(RayFinished {
                final_pos: ray.advanced_by(&IRat::from(10000)),
            })
        }
    }
}

pub fn start() {
    render_scene(&Scene {
        light: Vec2::new(0.2, 0.1),
        boundaries: vec![],
    });
}

pub fn render_scene(scene: &Scene) {
    use imageproc::drawing;

    let pixels = 128;

    let mut img: RgbImage = Image::new(pixels, pixels);
    let ray_ct = 0;

    let scene2img = IRat::from(1);

    macro_rules! draw_line {
        ($start:expr, $end:expr) => {{
            let a = &$start * &scene2img;
            let b = &$end * &scene2img;
            drawing::draw_line_segment_mut(&mut img, a.to_f32s(), b.to_f32s(), Rgb([255; 3]));
        }};
    }

    // drawing::draw_line_segment_mut(&mut img, (0., y), (10., y), Rgb([255; 3]));
    for y in 0..pixels {
        // let y = y as f32;
        // drawing::draw_line_segment_mut(&mut img, (0., y), (10., y), Rgb([255; 3]));
        let y = y as f64;
        draw_line!(Vec2::new(0.1, y), Vec2::new(1., y));
    }

    for ray_idx in 0..ray_ct {
        let mut ray = Ray {
            pos: scene.light.clone(),
            dir: Vec2::new(0.1, 0. + ray_idx as f64 / 100.),
        };

        for _ in 0..32 {
            match scene.step_ray(&ray) {
                Ok(new_ray) => {
                    draw_line!(ray.pos, new_ray.pos);
                    ray = new_ray;
                }
                Err(finished) => {
                    draw_line!(ray.pos, finished.final_pos);
                    break;
                }
            }
        }
    }

    img.save("raytracer_2d_result.png").unwrap();
}
