use std::{
    fmt::Display,
    ops::{Add, Div, Mul, Neg, Sub},
};

use perf_tracer::trace_op;
use perf_tracer_macros::trace_function;

use crate::{
    derive_binop_by_value, derive_binop_by_value_assymetric,
    math_things::{
        Sign,
        rational::{IRat, Precision, URat},
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Quadrant {
    First,
    Second,
    Third,
    Fourth,
}

#[derive(Clone, PartialEq, Eq)]
pub struct Vec2 {
    pub x: IRat,
    pub y: IRat,
}

impl std::fmt::Debug for Vec2 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({:?}, {:?})", self.x, self.y)
    }
}

impl Display for Vec2 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}, {})", self.x, self.y)
    }
}

impl Vec2 {
    pub fn new(x: impl Into<IRat>, y: impl Into<IRat>) -> Self {
        Self {
            x: x.into(),
            y: y.into(),
        }
    }

    pub fn with_x(mut self, x: impl Into<IRat>) -> Self {
        self.x = x.into();
        self
    }

    pub fn with_y(mut self, y: impl Into<IRat>) -> Self {
        self.y = y.into();
        self
    }

    /// `to_f64s`, but casted to i32s
    pub fn to_i32s(&self) -> (i32, i32) {
        (self.x.to_f64() as i32, self.y.to_f64() as i32)
    }

    pub fn to_f32s(&self) -> (f32, f32) {
        (self.x.to_f64() as f32, self.y.to_f64() as f32)
    }

    /// Returns `(self.x, self.y)` as their float representations
    #[trace_function("Vec2::$f")]
    pub fn to_f64s(&self) -> (f64, f64) {
        (self.x.to_f64(), self.y.to_f64())
    }

    pub fn quadrant(&self) -> Quadrant {
        match (self.x >= IRat::zero(), self.y >= IRat::zero()) {
            (true, true) => Quadrant::First,
            (false, true) => Quadrant::Second,
            (false, false) => Quadrant::Third,
            (true, false) => Quadrant::Fourth,
        }
    }

    pub fn dot(&self, other: &Self) -> IRat {
        &self.x * &other.x + &self.y * &other.y
    }

    /// Returns the rejection of `self` from `from`
    pub fn rejection(&self, from: &Self) -> Self {
        self - self.projection(from)
    }

    /// Returns the rejection of `self` on `onto`
    pub fn projection(&self, onto: &Self) -> Self {
        (self.dot(onto) / onto.sqr_magnitude()) * self
    }

    pub fn sqr_magnitude(&self) -> IRat {
        self.dot(self)
    }

    /// IMPRECISE
    #[trace_function("Vec2::$f")]
    pub fn magnitude(&self, prec: Precision) -> IRat {
        self.sqr_magnitude().sqrt(prec)
    }

    pub fn sqr_dist(&self, other: &Self) -> IRat {
        (self - other).sqr_magnitude()
    }

    pub fn negated(self) -> Self {
        Self {
            x: -self.x,
            y: -self.y,
        }
    }

    /// IMPRECISE
    ///
    /// The returned value will not be exactly normalized.
    /// If you want an exactly normalized vector without maintaining exact direction, use
    /// `normalize_exact_magnitude`
    #[must_use]
    #[trace_function("Vec2::$f")]
    pub fn normalize_exact_dir(&self, prec: Precision) -> Self {
        self / self.magnitude(prec)
    }

    /// Returns this vector reflected across `normal`
    /// * `self` and `normal` should be unit vectors
    #[must_use]
    #[trace_function("Vec2::$f")]
    pub fn reflected(&self, normal: &Self) -> Vec2 {
        // https://en.wikipedia.org/wiki/Specular_reflection#Vector_formulation
        self - IRat::from(2u64) * normal * normal.dot(self)
    }

    /// Performs `self ^ other`, otherwise known as the 2d wedge product or the perp dot product:
    ///
    /// https://mathworld.wolfram.com/PerpDotProduct.html
    #[trace_function("Vec2::$f")]
    pub fn cross(&self, other: &Self) -> IRat {
        &self.x * &other.y - &self.y * &other.x
    }

    /// Finds a point nearby `self` that lies on the unit circle, within `prec` for
    #[trace_function("Vec2::$f")]
    pub fn normalize_exact_magnitude(&self, prec: Precision) -> Vec2 {
        let mut self_reflected = self.clone();
        if self.x < IRat::zero() {
            self_reflected.x = self_reflected.x.neg();
        }
        if self.y < IRat::zero() {
            self_reflected.y = self_reflected.y.neg();
        }

        let mut res = Self::normalize_exact_magnitude_inner(&self_reflected, prec);
        if self.x < IRat::zero() {
            res.x = res.x.neg();
        }
        if self.y < IRat::zero() {
            res.y = res.y.neg();
        }

        debug_assert_eq!(res.sqr_magnitude(), IRat::one());
        println!("{} -> {}", self, res);
        res
    }

    /// Requires `pt` to be in the first quadrant
    fn normalize_exact_magnitude_inner(pt: &Vec2, prec: Precision) -> Vec2 {
        if pt.y.is_zero() {
            return Vec2::new(1, 0);
        }
        if pt.x.is_zero() {
            return Vec2::new(0, 1);
        }

        // FIXME:
        // Update these comments based on adjusted algorithm,
        // and justify the number of steps for a given precision (and a given step reduction)

        // https://en.wikipedia.org/wiki/Pythagorean_triple#Rational_points_on_a_unit_circle
        // Using the parametric equation, an appropriately accurate value of t can be found
        // for a point P in quadrant 1 by a binary search:
        // Starting with a=0,b=1 perform a->avg(a,b) if b is closer than a, else perform b->avg(a,b)

        // CORRECTION:
        // * Binary search does not work (only stochastically for cases already near the unit circle):
        // * Fundamentally, the question is of minimizing the function from t=[0,1]: `D(t)=dist(P(t), pt)`
        // * This function has one inflection point when pt is exclusively within the 1st quadrant
        // (not on an axis, handle that on its own) so our solution is the zero of `D'(t)`.
        // * Unfortunately, D'(t) is not a well-behaved function and any `pt.magnitude() == 1` case will
        // involve an undefined point, so numerical derivative based methods (i.e. Newton's) won't work.
        // However, in the cases where `pt.magnitude ~ 1`, the graph of D(t) nearly forms a triangle, and an iteration
        // of newton's method would give a good initial estimate
        // Methods of improving guesses:
        // Try:
        // * Increase `t` by a step size until the guess starts getting further away,
        // then decrease the step size and reverse direction. Dividing the step size by 8 or another small power of 2 would be a good choice.
        // This method is just a binary search that fixes the problem of "missing" the actual minimum

        // * Each step doubles the precision, so for `n` steps,
        // the precision is a quarter of a circle / 2^n
        // * Since the points are further apart at t=0, the worst case
        // of the innacuracy is `dist(P(1/2^n), P(0))` where P is the point at a given t value
        // * Since `P(0)=(1,0)`, `dist(P(1/2^n), P(0))` is approximately `P(1/2^n).y`
        // `P(t).y = 2t / (1+t^2)` and near t=0, `P(t).x ~ 2t` so `P(1/2^n).y ~ 1/2^(n-1)`
        // * To conclude, the worst case precision for `n` steps is approximately `1/2 ^ (n-1)`
        // * To find the proper `n` steps for our precision `1/2^prec`, we need a value `n` that satisfies the inequality:
        // 1/2 ^ prec <= Error(n) <= 1/2 ^ (n-1)
        // * So, `n = prec + 1` works fine

        struct Guess {
            t: IRat,
            pt: Vec2,
            sqr_dist: IRat,
        }

        let new_guess = |t: IRat| -> Guess {
            let t_sqr = &t * &t;
            let one_plus_t_sqr = IRat::one() + &t_sqr;
            let t_pt = Vec2::new(
                (IRat::one() - &t_sqr) / &one_plus_t_sqr,
                (IRat::from(2) * &t) / &one_plus_t_sqr,
            );
            let sqr_dist = t_pt.sqr_dist(pt);
            Guess {
                t,
                pt: t_pt,
                sqr_dist,
            }
        };

        let mut step = IRat::from(URat::new(1u64, 8u64));

        let mut prev = new_guess(IRat::zero());

        for _ in 0..prec.0 / 2 + 1 {
            loop {
                let next_t = &prev.t + &step;
                let next = new_guess(next_t);

                if next.sqr_dist > prev.sqr_dist {
                    step = step * IRat::new(URat::new(1u64, 4u64), Sign::Neg);
                    break;
                } else {
                    prev = next;
                }
            }
        }

        prev.t.round(prec);
        new_guess(prev.t).pt
    }
}

impl Mul<&IRat> for &Vec2 {
    type Output = Vec2;

    fn mul(self, rhs: &IRat) -> Self::Output {
        Vec2 {
            x: &self.x * rhs,
            y: &self.y * rhs,
        }
    }
}
derive_binop_by_value_assymetric!(Vec2, IRat, Mul, mul, *);

impl Div<&IRat> for &Vec2 {
    type Output = Vec2;

    fn div(self, rhs: &IRat) -> Self::Output {
        Vec2 {
            x: &self.x / rhs,
            y: &self.y / rhs,
        }
    }
}
derive_binop_by_value_assymetric!(Vec2, IRat, Div, div, /);

impl Add<&Vec2> for &Vec2 {
    type Output = Vec2;

    fn add(self, rhs: &Vec2) -> Self::Output {
        Vec2 {
            x: &self.x + &rhs.x,
            y: &self.y + &rhs.y,
        }
    }
}
derive_binop_by_value!(Vec2, Add, add, +);

impl Sub<&Vec2> for &Vec2 {
    type Output = Vec2;

    fn sub(self, rhs: &Vec2) -> Self::Output {
        Vec2 {
            x: &self.x - &rhs.x,
            y: &self.y - &rhs.y,
        }
    }
}
derive_binop_by_value!(Vec2, Sub, sub, -);
