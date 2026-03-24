use std::{
    fmt::{Debug, Display},
    ops::{Add, Div, Mul, Neg, Sub},
};

use perf_tracer_macros::trace_function;

use crate::{derive_binop_by_value, derive_binop_by_value_assymetric, math_things::rational::IRat};

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
