use std::{
    fmt::{Debug, Display},
    ops::{Add, Div, Mul, Neg, Sub},
};

use perf_tracer_macros::trace_function;

use crate::{
    derive_binop_by_value, derive_binop_by_value_assymetric,
    math_things::{
        rational::{IRat, Precision, URat},
        vec2::Vec2,
    },
};

#[derive(Clone, PartialEq, Eq)]
pub struct Vec3 {
    x: IRat,
    y: IRat,
    z: IRat,
}

impl Debug for Vec3 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({:?}, {:?}, {:?})", self.x, self.y, self.z)
    }
}

impl Display for Vec3 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}, {}, {})", self.x, self.y, self.z)
    }
}

impl Vec3 {
    pub fn new(x: impl Into<IRat>, y: impl Into<IRat>, z: impl Into<IRat>) -> Self {
        Self {
            x: x.into(),
            y: y.into(),
            z: z.into(),
        }
    }

    pub fn splat(elem: impl Into<IRat>) -> Self {
        let elem = elem.into();
        Self::new(elem.clone(), elem.clone(), elem)
    }

    #[inline(always)]
    pub fn x(&self) -> &IRat {
        &self.x
    }

    #[inline(always)]
    pub fn y(&self) -> &IRat {
        &self.y
    }

    #[inline(always)]
    pub fn z(&self) -> &IRat {
        &self.z
    }

    pub fn dot(&self, other: &Self) -> IRat {
        self.x() * other.x() + self.y() * other.y() + self.z() * other.z()
    }

    pub fn cross(&self, other: &Self) -> Self {
        Self::new(
            self.y() * other.z() - self.z() * other.y(),
            self.z() * other.x() - self.x() * other.z(),
            self.x() * other.y() - self.y() * other.x(),
        )
    }

    pub fn to_f32s(&self) -> [f32; 3] {
        self.to_f64s().map(|x| x as f32)
    }

    pub fn to_f64s(&self) -> [f64; 3] {
        [self.x.to_f64(), self.y.to_f64(), self.z.to_f64()]
    }

    /// Imprecise function, uses conversion from floats
    /// * (0, 0) returns (0, 0, 1)
    /// * (pi/2, 0) returns (0, 1, 0)
    /// * (-pi/2, 0) returns (0, 1, 0)
    /// * (0, pi/2) returns (-1, 0, 0)
    ///
    /// The result will be approximately normalized
    pub fn from_spherical_coords_inexact(elevation: f64, azimuth: f64) -> Vec3 {
        let inclination = std::f64::consts::FRAC_PI_2 - elevation;
        Vec3::new(
            inclination.sin() * azimuth.sin(),
            inclination.cos(),
            inclination.sin() * azimuth.cos(),
        )
    }

    /// Returns this vector reflected across `normal`
    /// * `self` and `normal` should be unit vectors
    #[must_use]
    #[trace_function("Vec3::$f")]
    pub fn reflected(&self, normal: &Self) -> Vec3 {
        // https://en.wikipedia.org/wiki/Specular_reflection#Vector_formulation
        self - IRat::from(2u64) * normal * normal.dot(self)
    }

    pub fn sqr_magnitude(&self) -> IRat {
        self.dot(self)
    }

    pub fn sqr_dist(&self, other: &Self) -> IRat {
        (self - other).sqr_magnitude()
    }

    #[trace_function("Vec3::$f")]
    pub fn normalize_exact_magnitude(&self, prec: Precision) -> Vec3 {
        // FIXME: handle cases with exactly zero coordinates by dispatching to Vec2::normalize_exact_magnitude
        if self.x.is_zero() {
            let v = Vec2::new(self.y.clone(), self.z.clone()).normalize_exact_magnitude(prec);
            return Vec3 {
                x: IRat::zero(),
                y: v.x,
                z: v.y,
            };
        } else if self.y.is_zero() {
            let v = Vec2::new(self.x.clone(), self.z.clone()).normalize_exact_magnitude(prec);
            return Vec3 {
                x: v.x,
                y: IRat::zero(),
                z: v.y,
            };
        } else if self.z.is_zero() {
            let v = Vec2::new(self.x.clone(), self.y.clone()).normalize_exact_magnitude(prec);
            return Vec3 {
                x: v.x,
                y: v.y,
                z: IRat::zero(),
            };
        }

        let mut self_reflected = self.clone();
        if self.x < IRat::zero() {
            self_reflected.x = self_reflected.x.neg();
        }
        if self.y < IRat::zero() {
            self_reflected.y = self_reflected.y.neg();
        }
        if self.z > IRat::zero() {
            self_reflected.z = self_reflected.z.neg();
        }

        let mut res = normalize_exact_magnitude_algorithms::closed_form(&self_reflected, prec);
        if self.x < IRat::zero() {
            res.x = res.x.neg();
        }
        if self.y < IRat::zero() {
            res.y = res.y.neg();
        }
        if self.z > IRat::zero() {
            res.z = res.z.neg();
        }

        res
    }
}

/// # Requirements
/// * `pt.x > 0` and `pt.y > 0`
/// * `pt.z < 0`
///
/// Since the input is restricted to the (+x,+y,-z) octant, the output point should be in that octant as well
mod normalize_exact_magnitude_algorithms {
    use crate::math_things::{
        rational::{IRat, Precision},
        vec2::Vec2,
        vec3::Vec3,
    };

    fn stereo_projection(xy: &Vec2) -> Vec3 {
        let xy_sqr_mag = xy.sqr_magnitude();
        let den = IRat::one() + &xy_sqr_mag;

        Vec3::new(
            IRat::from(2) * &xy.x,
            IRat::from(2) * &xy.y,
            &xy_sqr_mag - IRat::one(),
        ) * den.recip()
    }

    /// See module docs
    ///
    /// BUGGED, doesn't work
    pub fn downhill_simplex_search(pt: &Vec3, prec: Precision) -> Vec3 {
        // Try:
        // * Assume pt is roughly normalized
        // * Find a point on the slice of the sphere with pt.z
        // Try:
        // * https://en.wikipedia.org/wiki/Stereographic_projection#First_formulation
        // * Use stereographic projection `(X,Y) -> (x,y,z)` with `pt` in the bottom half (X^2 + Y^2 <= 1)
        // * This turns it into a 2d minimization problem
        // * It's probably the case that starting at X=0 and finding the minimum along Y, then finding the minimum along X
        // would return the correct answer (i.e. if (X, a) is a minimum for all x-values along y=a, then (X, b) is a minimum for all y=b)
        // ^ Unfortunately, this isn't true. But, the R2->R1 distance function is pretty well behaved (in and around the 1st quadrant unit circle)
        // Try:
        // * https://en.wikipedia.org/wiki/Nelder%E2%80%93Mead_method
        // * Downhill simplex method search over the distance function?
        // Try:
        // * Gradient-based search, the gradient of sqr distance is known and rational
        // * Since there's only 1 local minimum, a local minimum search would work
        // * Coordinate descent might work: https://en.wikipedia.org/wiki/Coordinate_descent
        // since calculating both coordinates of the gradient is probably ~50% more expensive than just one coordinate
        // * The distance function is *convex* but it's also quite steep when the point is far away

        // https://en.wikipedia.org/wiki/Nelder%E2%80%93Mead_method#One_possible_variation_of_the_NM_algorithm
        struct Guess {
            pt_xy: Vec2,
            pt_3d: Vec3,
            sqr_dist: IRat,
        }

        let make_guess = |xy: Vec2| -> Guess {
            // FIXME: the distance calculation can be vastly simplified as compared to
            // calculating the point then the distance, since the `pt_3d` isn't needed while searching
            let pt_3d = stereo_projection(&xy);
            let sqr_dist = pt_3d.sqr_dist(pt);

            Guess {
                pt_xy: xy,
                pt_3d,
                sqr_dist,
            }
        };

        let refl_coeff = IRat::one();
        let expand_coeff = IRat::from(2);
        let contract_coeff = IRat::from(2).recip();
        let shrink_coeff = IRat::from(2).recip();

        // We're searching over the XY-plane, a 2d space so we have 3 test points
        let mut pts: [Guess; 3] =
            [Vec2::new(0, 1), Vec2::new(1, 0), Vec2::new(1, 1)].map(make_guess);

        // `prec` is a tolerance of distance, so `prec^2` is a tolerance of sqr_dist
        let prec_tolerance = prec.to_urat().powi(2);

        loop {
            // Step 1
            pts.sort_by(|a, b| a.sqr_dist.cmp(&b.sqr_dist).reverse());

            // FIXME: the termination condition is imprecise
            // Terimation condition:
            // If the distance in quality between the two best points is less than the precision, the guess is within the precision ~ish
            if (&pts[0].sqr_dist - &pts[1].sqr_dist).abs_unsigned() <= prec_tolerance {
                return pts[0].pt_3d.clone();
            }

            // Step 2
            let x_0 = (&pts[0].pt_xy + &pts[1].pt_xy) / IRat::from(3);
            // Step 3
            let x_r = &x_0 - &refl_coeff * (&x_0 - &pts[2].pt_xy);
            let x_r = make_guess(x_r);
            if pts[0].sqr_dist <= x_r.sqr_dist && x_r.sqr_dist < pts[1].sqr_dist {
                pts[2] = x_r;
                continue;
            }

            // Step 4
            if x_r.sqr_dist < pts[0].sqr_dist {
                let x_e = &x_0 + &expand_coeff * (&x_r.pt_xy - &x_0);
                let x_e = make_guess(x_e);
                if x_e.sqr_dist < x_r.sqr_dist {
                    pts[2] = x_e;
                } else {
                    pts[2] = x_r;
                }
                continue;
            }

            // Step 5
            if x_r.sqr_dist < pts[2].sqr_dist {
                let x_c = &x_0 + &contract_coeff * (&x_r.pt_xy - &x_0);
                let x_c = make_guess(x_c);
                if x_c.sqr_dist < x_r.sqr_dist {
                    pts[2] = x_c;
                    continue;
                }
            } else {
                let x_c = &x_0 + &contract_coeff * (&pts[2].pt_xy - &x_0);
                let x_c = make_guess(x_c);
                if x_c.sqr_dist < pts[2].sqr_dist {
                    pts[2] = x_c;
                    continue;
                }
            }

            // Step 6
            for i in 1..3 {
                let new_xy = &pts[0].pt_xy + &shrink_coeff * (&pts[i].pt_xy - &pts[0].pt_xy);
                pts[i] = make_guess(new_xy);
            }
        }
    }

    pub fn closed_form(pt: &Vec3, prec: Precision) -> Vec3 {
        let x_sqr = &pt.x * &pt.x;
        let y_sqr = &pt.y * &pt.y;
        let x_sqr_plus_y_sqr = &x_sqr + &y_sqr;
        let num_part = &pt.x * (&x_sqr_plus_y_sqr + &pt.z * &pt.z).sqrt(prec) + &pt.x * &pt.z;

        let x = &num_part / &x_sqr_plus_y_sqr;
        let y = (&pt.y / &pt.x) * &x;

        stereo_projection(&Vec2::new(x, y))
    }
}

impl Add<&Vec3> for &Vec3 {
    type Output = Vec3;

    fn add(self, rhs: &Vec3) -> Self::Output {
        Vec3::new(self.x() + rhs.x(), self.y() + rhs.y(), self.z() + rhs.z())
    }
}
derive_binop_by_value!(Vec3, Add, add, +);

impl Sub<&Vec3> for &Vec3 {
    type Output = Vec3;

    #[inline]
    fn sub(self, rhs: &Vec3) -> Self::Output {
        self + rhs.clone().neg()
    }
}
derive_binop_by_value!(Vec3, Sub, sub, -);

impl Neg for Vec3 {
    type Output = Vec3;

    #[inline]
    fn neg(self) -> Self::Output {
        Self::new(-self.x, -self.y, -self.z)
    }
}

impl Mul<&IRat> for &Vec3 {
    type Output = Vec3;

    fn mul(self, rhs: &IRat) -> Self::Output {
        Vec3::new(self.x() * rhs, self.y() * rhs, self.z() * rhs)
    }
}
derive_binop_by_value_assymetric!(Vec3, IRat, Mul, mul, *);

impl Div<&IRat> for &Vec3 {
    type Output = Vec3;

    fn div(self, rhs: &IRat) -> Self::Output {
        Vec3::new(self.x() / rhs, self.y() / rhs, self.z() / rhs)
    }
}
