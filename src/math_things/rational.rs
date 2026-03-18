use std::{
    cmp::Ordering,
    ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Range, Sub},
};

use crate::{
    derive_binop_by_value, derive_binop_by_value_assymetric,
    math_things::{Sign, bigint::UBig},
};

#[inline(always)]
fn extract_bits(mut x: u64, bits: Range<u32>, keep_position: bool) -> u64 {
    x <<= 64 - bits.end;
    x >>= 64 - bits.end;

    x >>= bits.start;
    if keep_position {
        x <<= bits.start;
    }

    x
}

/// A reduced signed rational number
#[derive(Debug, Clone)]
pub struct IRat {
    magnitude: URat,
    sign: Sign,
}

impl IRat {
    pub fn new(magnitude: URat, sign: Sign) -> Self {
        Self { magnitude, sign }
    }

    #[must_use]
    pub fn is_zero(&self) -> bool {
        self.magnitude.is_zero()
    }

    pub fn zero() -> Self {
        Self {
            magnitude: URat::zero(),
            sign: Sign::Pos,
        }
    }

    pub fn one() -> Self {
        Self {
            magnitude: URat::one(),
            sign: Sign::Pos,
        }
    }

    /// A non-normal `x` will result in `0` being returned
    pub fn from_f64(x: f64) -> Self {
        let s = Self {
            magnitude: URat::from_f64(x),
            sign: match x.is_sign_positive() {
                true => Sign::Pos,
                false => Sign::Neg,
            },
        };

        println!("IRat::from_f64: {x} -> {s:?}");

        s
    }

    pub fn powi(&self, pow: u32) -> Self {
        let mut s = IRat::one();
        for _ in 0..pow {
            s = s * self;
        }
        s
    }

    /// IMPRECISE
    ///
    /// # Panics
    /// * If `self < 0`
    pub fn sqrt(&self, prec: &URat) -> Self {
        Self {
            magnitude: self.magnitude.sqrt(prec),
            sign: self.sign,
        }
    }

    /// Converts `self` into an f64, with possible loss
    pub fn to_f64(&self) -> f64 {
        // FIXME: Will fail with large fractions that should be representable
        self.magnitude.num.to_f64() / self.magnitude.den.to_f64()
            * match self.sign {
                Sign::Pos => 1.,
                Sign::Neg => -1.,
            }
    }

    /// Panics if `self` is zero
    #[must_use]
    #[track_caller]
    pub fn recip(&self) -> Self {
        Self {
            magnitude: self.magnitude.recip(),
            sign: self.sign,
        }
    }
}

impl From<f64> for IRat {
    fn from(value: f64) -> Self {
        Self::from_f64(value)
    }
}

impl From<i32> for IRat {
    fn from(value: i32) -> Self {
        Self::from(value as i64)
    }
}

impl From<i64> for IRat {
    fn from(value: i64) -> Self {
        Self {
            magnitude: URat::from(value.unsigned_abs()),
            sign: Sign::from_is_pos(value >= 0),
        }
    }
}

impl From<u32> for IRat {
    fn from(value: u32) -> Self {
        Self::from(value as u64)
    }
}

impl From<u64> for IRat {
    fn from(value: u64) -> Self {
        Self {
            magnitude: URat::from(value),
            sign: Sign::Pos,
        }
    }
}

impl Add<&IRat> for &IRat {
    type Output = IRat;

    fn add(self, rhs: &IRat) -> Self::Output {
        use Sign::*;
        match (self.sign, rhs.sign) {
            (Pos, Pos) | (Neg, Neg) => IRat {
                magnitude: &self.magnitude + &rhs.magnitude,
                sign: self.sign,
            },
            (Pos, Neg) | (Neg, Pos) => {
                if self.magnitude > rhs.magnitude {
                    IRat {
                        magnitude: &self.magnitude - &rhs.magnitude,
                        sign: self.sign,
                    }
                } else {
                    IRat {
                        magnitude: &rhs.magnitude - &self.magnitude,
                        sign: rhs.sign,
                    }
                }
            }
        }
    }
}
derive_binop_by_value!(IRat, Add, add, +);

impl Sub<&IRat> for &IRat {
    type Output = IRat;

    fn sub(self, rhs: &IRat) -> Self::Output {
        self + &(-rhs.clone())
    }
}
derive_binop_by_value!(IRat, Sub, sub, -);

impl Neg for IRat {
    type Output = IRat;

    fn neg(mut self) -> Self::Output {
        self.sign = -self.sign;
        self
    }
}

impl Mul<&IRat> for &IRat {
    type Output = IRat;

    fn mul(self, rhs: &IRat) -> Self::Output {
        IRat {
            magnitude: &self.magnitude * &rhs.magnitude,
            sign: self.sign.multiply(rhs.sign),
        }
    }
}
derive_binop_by_value!(IRat, Mul, mul, *);

impl Mul<&UBig> for &IRat {
    type Output = IRat;

    fn mul(self, rhs: &UBig) -> Self::Output {
        IRat {
            magnitude: &self.magnitude * rhs,
            sign: self.sign,
        }
    }
}
derive_binop_by_value_assymetric!(IRat, UBig, Mul, mul, *);

impl Div<&IRat> for &IRat {
    type Output = IRat;

    fn div(self, rhs: &IRat) -> Self::Output {
        IRat {
            magnitude: &self.magnitude / &rhs.magnitude,
            sign: self.sign.multiply(rhs.sign),
        }
    }
}
derive_binop_by_value!(IRat, Div, div, /);

impl PartialEq for IRat {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other).is_eq()
    }
}
impl Eq for IRat {}

impl PartialOrd for IRat {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for IRat {
    fn cmp(&self, other: &Self) -> Ordering {
        // Negative and positive zero should be equal
        if self.is_zero() && other.is_zero() {
            return Ordering::Equal;
        }

        match (self.sign, other.sign) {
            (Sign::Pos, Sign::Pos) => self.magnitude.cmp(&other.magnitude),
            (Sign::Pos, Sign::Neg) => Ordering::Greater,
            (Sign::Neg, Sign::Pos) => Ordering::Less,
            (Sign::Neg, Sign::Neg) => self.magnitude.cmp(&other.magnitude).reverse(),
        }
    }
}

/// A reduced unsigned rational number
#[derive(Clone)]
pub struct URat {
    num: UBig,
    /// The denominator, always non-zero
    den: UBig,
}

impl std::fmt::Debug for URat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}/{:?}", self.num, self.den)
    }
}

impl URat {
    #[track_caller]
    pub fn new(num: UBig, den: UBig) -> Self {
        if den.is_zero() {
            panic!("Called URat::new with a denominator of zero");
        }
        Self { num, den }.reduced()
    }

    pub fn from_u64(num: u64) -> Self {
        Self {
            num: num.into(),
            den: UBig::one(),
        }
    }

    pub fn from_u64_recip(den: u64) -> Self {
        Self {
            num: UBig::one(),
            den: den.into(),
        }
    }

    /// Returns `1 / 2^sig_figs`
    ///
    /// Intended for specifying precisions for imprecise functions
    pub fn from_sig_figs(sig_figs: u32) -> Self {
        Self::from_u64_recip(2).powi(sig_figs as i32)
    }

    pub fn is_zero(&self) -> bool {
        self.num.is_zero()
    }

    pub fn zero() -> Self {
        Self {
            num: UBig::zero(),
            den: UBig::one(),
        }
    }

    pub fn one() -> Self {
        Self {
            num: UBig::one(),
            den: UBig::one(),
        }
    }

    pub fn abs_difference(&self, other: &Self) -> Self {
        if self > other {
            self - other
        } else {
            other - self
        }
    }

    #[must_use]
    pub fn reduced(&self) -> Self {
        let divisor = UBig::gcd(self.num.clone(), self.den.clone());
        Self {
            num: &self.num / &divisor,
            den: &self.den / &divisor,
        }
    }

    /// Panics if `self` is zero
    #[must_use]
    #[track_caller]
    pub fn recip(&self) -> Self {
        if self.is_zero() {
            panic!("Tried to take the reciprocal of zero");
        }
        Self {
            num: self.den.clone(),
            den: self.num.clone(),
        }
    }

    #[must_use]
    pub fn floor(&self) -> UBig {
        &self.num / &self.den
    }

    #[must_use]
    pub fn powi(&self, exp: i32) -> Self {
        let mut n = Self::one();

        for _ in 0..exp.abs() {
            n *= &self;
        }

        n
    }

    /// IMPRECISE
    pub fn sqrt(&self, prec: &URat) -> Self {
        sqrt_algorithms::sqrt_herons_method(self, prec)
    }

    /// Discards the sign
    ///
    /// A non-normal `x` will result in `0` being returned
    pub fn from_f64(x: f64) -> URat {
        match x.classify() {
            // FIXME: Should this panic instead?
            std::num::FpCategory::Nan | std::num::FpCategory::Infinite => return URat::zero(),
            // FIXME: Handle Subnormals
            std::num::FpCategory::Subnormal => todo!(),
            std::num::FpCategory::Zero => return URat::zero(),
            std::num::FpCategory::Normal => (),
        }

        let bits = x.to_bits();
        let exp_bits = extract_bits(bits, 52..63, false);
        let frac_bits = extract_bits(bits, 0..52, false);

        // `x = 1.frac_bits * 2^exp`
        //`0.f[51]f[50]...f[0]`

        // FIXME: the fractional parsing can be replaced with a UBig::from_u64(..)
        // and a division by 2^52 (which is a constant)
        // ^ Or something like that

        // First, add all the bits of the fraction
        let mut num = Self::one();
        let mut place = Self::one();

        for i in (0..52).rev() {
            place /= &Self::from_u64(2);
            let frac_bit = frac_bits >> i & 1;
            if frac_bit == 1 {
                num = &num + &place;
            }
        }

        // Now, result = frac * 2^(exp - 1023)
        let exp = exp_bits as i32 - 1023;
        num *= &URat::from_u64(2).powi(exp);

        num
    }
}

impl From<u32> for URat {
    fn from(value: u32) -> Self {
        URat::from(value as u64)
    }
}

impl From<u64> for URat {
    fn from(value: u64) -> Self {
        URat::from_u64(value)
    }
}

impl Add<&URat> for &URat {
    type Output = URat;

    fn add(self, rhs: &URat) -> Self::Output {
        // a/b + c/d = (ad + bc) / bd
        URat {
            num: &self.num * &rhs.den + &rhs.num * &self.den,
            den: &self.den * &rhs.den,
        }
        .reduced()
    }
}
derive_binop_by_value!(URat, Add, add, +);

impl Sub<&URat> for &URat {
    type Output = URat;

    fn sub(self, rhs: &URat) -> Self::Output {
        // a/b - c/d = (ad - bc) / bd
        URat {
            num: &self.num * &rhs.den - &rhs.num * &self.den,
            den: &self.den * &rhs.den,
        }
        .reduced()
    }
}
derive_binop_by_value!(URat, Sub, sub, -);

impl Mul<&URat> for &URat {
    type Output = URat;

    fn mul(self, rhs: &URat) -> Self::Output {
        URat {
            num: &self.num * &rhs.num,
            den: &self.den * &rhs.den,
        }
        .reduced()
    }
}
derive_binop_by_value!(URat, Mul, mul, *);

impl MulAssign<&URat> for URat {
    fn mul_assign(&mut self, rhs: &URat) {
        *self = &*self * rhs;
    }
}

impl Mul<&UBig> for &URat {
    type Output = URat;

    fn mul(self, rhs: &UBig) -> Self::Output {
        URat {
            num: &self.num * rhs,
            den: self.den.clone(),
        }
        .reduced()
    }
}
derive_binop_by_value_assymetric!(URat, UBig, Mul, mul, *);

impl Div<&URat> for &URat {
    type Output = URat;

    fn div(self, rhs: &URat) -> Self::Output {
        // (a/b) / (c/d) = ad / bc
        URat {
            num: &self.num * &rhs.den,
            den: &self.den * &rhs.num,
        }
        .reduced()
    }
}
derive_binop_by_value!(URat, Div, div, /);

impl Div<u64> for &URat {
    type Output = URat;

    fn div(self, rhs: u64) -> Self::Output {
        self / URat::from_u64(rhs)
    }
}
impl Div<u64> for URat {
    type Output = URat;

    fn div(self, rhs: u64) -> Self::Output {
        &self / rhs
    }
}

impl DivAssign<&URat> for URat {
    fn div_assign(&mut self, rhs: &URat) {
        *self = &*self / rhs;
    }
}

impl PartialEq for URat {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other).is_eq()
    }
}

impl Eq for URat {}

impl PartialOrd for URat {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for URat {
    fn cmp(&self, other: &Self) -> Ordering {
        // FIXME: Is this a necessary/performant check?
        // We're guaranteed that self and other are reduced, so this is a sufficient check
        if self.num == other.num && self.den == other.den {
            return Ordering::Equal;
        }

        // a/b > c/d == ad > bc

        let lhs = &self.num * &other.den;
        let rhs = &self.den * &other.num;
        lhs.cmp(&rhs)
    }
}

mod sqrt_algorithms {
    use crate::math_things::rational::URat;

    pub fn sqrt_herons_method(s: &URat, prec: &URat) -> URat {
        if s.is_zero() {
            return URat::zero();
        }

        let mut guess = s.clone() / 2;

        loop {
            let next_guess = (&guess + s / &guess) / 2;

            // The change between guesses is at most the
            // absolute error of the current guess
            if next_guess.abs_difference(&guess) < *prec {
                return next_guess;
            }
            guess = next_guess;
        }
    }
}
