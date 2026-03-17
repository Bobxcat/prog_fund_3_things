use imageproc::{
    definitions::Image,
    image::{Rgb, RgbImage},
};

use crate::math_things::{rational::IRat, vec2::Vec2};

pub mod intersect2d {
    use crate::math_things::{
        mat2::Mat2,
        rational::{IRat, URat},
        vec2::Vec2,
    };

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

        if IRat::zero() <= t && t <= IRat::one() && IRat::zero() <= s && s <= IRat::one() {
            Some(a_start + t * a_dir)
        } else {
            None
        }
    }

    // pub struct RefractionResult

    /// https://en.wikipedia.org/wiki/Snell%27s_law#Vector_form
    ///
    /// `ray_dir` mut be a unit vector
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

        let n_mag = &r * &c - sqrt_inner.sqrt(&URat::from_sig_figs(128));
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

pub enum Shape {
    Line { a: Vec2, b: Vec2 },
    Circle { center: Vec2, radius: IRat },
}

impl Shape {
    //
}

pub struct Object {
    pub shape: Shape,
}

/// Raymarch vs raycast:
/// * Raymarching gets infinitely stuck going against a circle
/// *
pub struct Scene {
    light: Vec2,
    objects: Vec<Object>,
}

impl Scene {
    pub fn cast(&self, start: &Vec2, dir: &Vec2) -> Vec2 {
        todo!()
    }
}

pub fn _start() {
    render_scene(&Scene {
        light: Vec2::new(0.2, 0.1),
        objects: vec![],
    });
}

pub fn render_scene(scene: &Scene) {
    use imageproc::drawing;

    let mut img: RgbImage = Image::new(1024, 1024);
    let ray_ct = 128;

    let scene2img = IRat::from(1024.);

    macro_rules! draw_line {
        ($start:expr, $end:expr) => {{
            let a = &$start * &scene2img;
            let b = &$end * &scene2img;
            drawing::draw_line_segment_mut(&mut img, a.to_f32s(), b.to_f32s(), Rgb([255; 3]));
        }};
    }

    for ray_idx in 0..ray_ct {
        let mut ray_pos = scene.light.clone();
        let ray_dir = Vec2::new(0., 0.1 + ray_idx as f64 / 400.);

        // let cast = scene.cast(ray_pos, ray_dir * );
        draw_line!(ray_pos, ray_dir);
    }

    img.save("raytracer_2d_result.png").unwrap();
}
