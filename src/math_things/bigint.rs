use std::{
    cmp::Ordering,
    fmt,
    ops::{
        Add, AddAssign, Div, Mul, MulAssign, Rem, Shl, ShlAssign, Shr, ShrAssign, Sub, SubAssign,
    },
};

use crate::math_things::Sign;

pub struct IBig {
    sign: Sign,
    magnitude: UBig,
}

#[derive(Debug, Clone)]
pub struct UBig {
    /// Least significant digit is at 0
    ///
    /// Most significant digit is always non-zero
    digits: Vec<u64>,
}

impl UBig {
    pub fn new(n: u64) -> Self {
        Self::from(n)
    }

    pub fn from_digits(digits: Vec<u64>) -> Self {
        let mut s = Self { digits };
        s.trim();
        s
    }

    pub fn from_radix(raw: &str, radix: u32) -> Self {
        let mut s = Self::zero();
        let mut place = Self::one();
        for digit in raw.chars().rev() {
            let digit = digit.to_digit(radix).unwrap();
            let digit = &UBig::from(digit) * &place;

            s += &digit;
            place *= &UBig::from(radix);
        }
        s
    }

    /// Returns `a * b`, even if the product would overflow a u64
    pub fn from_u64_product(a: u64, b: u64) -> Self {
        let (n, carry) = a.carrying_mul(b, 0);
        let mut s = Self::from_digits(vec![n, carry]);
        s.trim();
        s
    }

    pub fn mul_assign_u64(&mut self, n: u64) {
        // From carrying_mul docs
        let mut carry = 0;
        for d in self.digits.iter_mut() {
            (*d, carry) = d.carrying_mul(n, carry);
        }
        if carry != 0 {
            self.digits.push(carry);
        }
    }

    #[track_caller]
    pub fn div_rem(&self, rhs: &Self) -> (UBig, UBig) {
        div_algorithms::div_rem_binary_long(self, rhs)
    }

    pub fn digit_or_zero(&self, digit: usize) -> u64 {
        self.digits.get(digit).copied().unwrap_or(0)
    }

    pub fn zero() -> Self {
        Self { digits: vec![] }
    }

    pub fn is_zero(&self) -> bool {
        self.digits.is_empty()
    }

    pub fn one() -> Self {
        Self { digits: vec![1] }
    }

    /// Returns `(div, rem)` by the word size of UBig (64 bits for a u64 bit digit)
    fn div_rem_word(n: u64) -> (u64, u64) {
        (n / u64::BITS as u64, n % u64::BITS as u64)
    }

    #[inline]
    pub fn get_bit(&self, bit: u64) -> bool {
        let (dig, shmt) = Self::div_rem_word(bit);
        self.digit_or_zero(dig as usize) >> shmt & 1 == 1
    }

    #[inline]
    pub fn set_bit(&mut self, bit: u64, to: bool) {
        let (dig, shmt) = Self::div_rem_word(bit);
        let mask = !(1u64 << shmt);

        let dig = self.digit_or_insert(dig as usize);
        *dig &= mask;
        *dig |= (to as u64) << shmt;
    }

    /// Should be called after any operation which may leave zeroed digits at the top
    #[inline]
    fn trim(&mut self) {
        while self.digits.last().is_some_and(|x| *x == 0) {
            self.digits.pop();
        }
    }

    #[inline]
    fn digit_or_insert(&mut self, digit: usize) -> &mut u64 {
        while self.digits.len() <= digit {
            self.digits.push(0);
        }
        &mut self.digits[digit]
    }

    fn add_digit(&mut self, mut dig_idx: usize, rhs: u64) {
        if rhs == 0 {
            return;
        }
        let mut overflow;
        (self.digits[dig_idx], overflow) = self.digit_or_insert(dig_idx).overflowing_add(rhs);

        while overflow {
            dig_idx += 1;
            (self.digits[dig_idx], overflow) = self.digit_or_insert(dig_idx).overflowing_add(1);
        }
    }

    fn sub_digit(&mut self, mut dig_idx: usize, rhs: u64) {
        if rhs == 0 {
            return;
        }
        let mut borrow;
        (self.digits[dig_idx], borrow) = self.digit_or_insert(dig_idx).overflowing_sub(rhs);

        while borrow {
            dig_idx += 1;
            if dig_idx >= self.digits.len() {
                *self = Self::zero();
                return;
            }
            (self.digits[dig_idx], borrow) = self.digits[dig_idx].overflowing_sub(1);
        }
        self.trim();
    }

    /// Shifts `self` left by `shamt` bits, extending with 1s if `ext` is true and
    /// extending with 0s otherwise
    #[inline]
    fn shl_assign_ext(&mut self, shamt: u64, ext: bool) {
        let (digits_to_add, shamt_in_digit) = Self::div_rem_word(shamt);

        self.digits.push(0);
        for i in (0..self.digits.len() - 1).rev() {
            let to_move_up = self.digits[i].unbounded_shr(u64::BITS - shamt_in_digit as u32);
            self.digits[i] <<= shamt_in_digit;
            self.digits[i + 1] |= to_move_up;
        }

        if ext {
            self.digits[0] |= (!0u64).unbounded_shr(u64::BITS - shamt_in_digit as u32);
        }

        self.trim();
        for _ in 0..digits_to_add {
            let ext_word = if ext { !0 } else { 0 };
            self.digits.insert(0, ext_word)
        }
    }

    /// Returns `0` when self is zero
    pub fn trailing_zeroes(&self) -> usize {
        let mut z = 0;
        for &dig in &self.digits {
            let trail = dig.trailing_zeros();
            z += trail;
            if trail < u64::BITS {
                break;
            }
        }
        z as usize
    }

    pub fn is_odd(&self) -> bool {
        self.get_bit(0)
    }

    pub fn is_even(&self) -> bool {
        !self.is_odd()
    }

    pub fn gcd(self, other: Self) -> UBig {
        if self.is_zero() && other.is_zero() {
            return UBig::one();
        }

        gcd_algorithms::gcd_binary(self, other)
    }

    /// https://en.wikipedia.org/wiki/Square_root_algorithms#Implementation
    pub fn sqrt(&self) -> Self {
        todo!()
    }
}

impl fmt::Binary for UBig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let v = self
            .digits
            .iter()
            .rev()
            .map(|dig| format!("{dig:0>64b}"))
            .collect::<Vec<_>>()
            .join("_");
        write!(f, "0b{v}")
    }
}

mod mul_algorithms {
    use crate::math_things::bigint::UBig;

    /// Simple O(n^2) algorithm
    pub fn mul_basic(a: &UBig, b: &UBig) -> UBig {
        if a.is_zero() || b.is_zero() {
            // Skip needing to trim by checking for mul by zero
            // Account for multiply by 0, otherwise the result will only ever be large
            return UBig::zero();
        }

        let len = a.digits.len().max(b.digits.len());
        let mut out = UBig::zero();

        for j in 0..len {
            let mut carry = 0;
            for i in 0..(len - j) {
                (out.digits[j + i], carry) = u64::carrying_mul_add(
                    a.digit_or_zero(i),
                    b.digit_or_zero(j),
                    *out.digit_or_insert(j + i),
                    carry,
                );
            }
            // The final digit set in the inner loop is
            // `out.get(j + i)` -> `out.get(j + len - j - 1)` -> `out.get(len - 1)`
            // so our final carry should be to `out.get(len)`
            out.add_digit(len, carry);
        }

        out
    }

    /// Split into low `d` digits and the remaining digits
    ///
    /// Returns `(hi, lo)`
    fn split_at(num: UBig, d: usize) -> (UBig, UBig) {
        let mut digits = num.digits.into_iter();
        let lo = UBig::from_digits(digits.by_ref().take(d).collect());
        let hi = UBig::from_digits(digits.collect());
        (hi, lo)
    }

    /// More complicated O(n^1.58) algorithm
    pub fn mul_karatsuba(a: &UBig, b: &UBig) -> UBig {
        todo!()
    }
}

mod div_algorithms {
    use crate::math_things::bigint::UBig;

    /// Computes `n / d`, returning `(quotient, remainder)`
    #[track_caller]
    pub fn div_rem_binary_long(n: &UBig, d: &UBig) -> (UBig, UBig) {
        // From https://en.wikipedia.org/wiki/Division_algorithm#Long_division
        assert!(!d.is_zero(), "Attempted to divide by zero");

        let mut q = UBig::zero();
        let mut r = UBig::zero();
        for i in (0..n.digits.len() * u64::BITS as usize).rev() {
            // Left shift r by 1 bit and set the least sig bit of r to bit i of the numerator
            r.shl_assign_ext(1, n.get_bit(i as u64));

            if &r >= d {
                r -= &d;
                q.set_bit(i as u64, true);
            }
        }

        (q, r)
    }
}

mod gcd_algorithms {
    use std::mem;

    use crate::math_things::bigint::UBig;

    /// https://en.wikipedia.org/wiki/Binary_GCD_algorithm
    pub fn gcd_binary(mut u: UBig, mut v: UBig) -> UBig {
        if u.is_zero() {
            return v;
        } else if v.is_zero() {
            return u;
        }

        let i = u.trailing_zeroes();
        u >>= i;
        let j = v.trailing_zeroes();
        v >>= j;
        let k = i.min(j);

        loop {
            debug_assert!(u.is_odd());
            debug_assert!(v.is_odd());

            // Ensure u <= v
            if u > v {
                mem::swap(&mut u, &mut v);
            }

            v -= &u;

            if v.is_zero() {
                return u << k;
            }

            v >>= v.trailing_zeroes();
        }
    }
}

impl From<u32> for UBig {
    fn from(value: u32) -> Self {
        Self::from(value as u64)
    }
}
impl From<u64> for UBig {
    fn from(value: u64) -> Self {
        Self::from_digits(vec![value])
    }
}

impl AddAssign<&Self> for UBig {
    fn add_assign(&mut self, rhs: &Self) {
        // FIXME: Use `carrying_add` instead?
        // That should only require a single iteration, instead of nested conditions
        for (dig_idx, rhs_digit) in rhs.digits.iter().copied().enumerate() {
            self.add_digit(dig_idx, rhs_digit);
        }
    }
}

impl SubAssign<&Self> for UBig {
    fn sub_assign(&mut self, rhs: &Self) {
        // FIXME: Use `borrowing_sub` instead?
        // That should only require a single iteration, instead of nested conditions
        for (dig_idx, rhs_digit) in rhs.digits.iter().copied().enumerate() {
            self.sub_digit(dig_idx, rhs_digit);
        }
    }
}

impl ShlAssign<u64> for UBig {
    fn shl_assign(&mut self, rhs: u64) {
        self.shl_assign_ext(rhs, false);
    }
}
impl ShlAssign<usize> for UBig {
    fn shl_assign(&mut self, rhs: usize) {
        *self <<= rhs as u64;
    }
}

impl ShrAssign<u64> for UBig {
    fn shr_assign(&mut self, rhs: u64) {
        let (digits_to_remove, shamt_in_digit) = Self::div_rem_word(rhs);

        for _ in 0..digits_to_remove {
            self.digits.remove(0);
        }

        for i in 0..self.digits.len() {
            let to_move_down = self.digits[i].unbounded_shl(u64::BITS - shamt_in_digit as u32);
            self.digits[i] >>= shamt_in_digit;
            if i > 0 {
                self.digits[i - 1] |= to_move_down;
            }
        }

        self.trim();
    }
}
impl ShrAssign<usize> for UBig {
    fn shr_assign(&mut self, rhs: usize) {
        *self >>= rhs as u64;
    }
}

impl Mul<&UBig> for &UBig {
    type Output = UBig;

    fn mul(self, rhs: &UBig) -> Self::Output {
        mul_algorithms::mul_basic(self, rhs)
    }
}

impl Div<&UBig> for &UBig {
    type Output = UBig;

    #[inline]
    fn div(self, rhs: &UBig) -> Self::Output {
        self.div_rem(rhs).0
    }
}

impl Rem<&UBig> for &UBig {
    type Output = UBig;

    #[inline]
    fn rem(self, rhs: &UBig) -> Self::Output {
        self.div_rem(rhs).1
    }
}

impl MulAssign<&Self> for UBig {
    fn mul_assign(&mut self, rhs: &Self) {
        *self = &*self * rhs;
    }
}

impl Add<&Self> for UBig {
    type Output = Self;

    fn add(mut self, rhs: &Self) -> Self::Output {
        self += rhs;
        self
    }
}

impl Add<Self> for UBig {
    type Output = Self;

    fn add(mut self, rhs: Self) -> Self::Output {
        self += &rhs;
        self
    }
}

impl Sub<Self> for UBig {
    type Output = Self;

    fn sub(mut self, rhs: Self) -> Self::Output {
        self -= &rhs;
        self
    }
}

impl Shl<usize> for UBig {
    type Output = Self;

    fn shl(mut self, rhs: usize) -> Self::Output {
        self <<= rhs;
        self
    }
}

impl Shr<usize> for UBig {
    type Output = Self;

    fn shr(mut self, rhs: usize) -> Self::Output {
        self >>= rhs;
        self
    }
}

impl PartialEq for UBig {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other).is_eq()
    }
}

impl Eq for UBig {}

impl PartialOrd for UBig {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for UBig {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.digits.len().cmp(&other.digits.len()) {
            Ordering::Less => return Ordering::Less,
            Ordering::Greater => return Ordering::Greater,
            Ordering::Equal => (),
        }

        for i in (0..self.digits.len()).rev() {
            match self.digits[i].cmp(&other.digits[i]) {
                Ordering::Less => return Ordering::Less,
                Ordering::Greater => return Ordering::Greater,
                Ordering::Equal => (),
            }
        }

        Ordering::Equal
    }
}

#[cfg(test)]
mod tests {
    use std::ops::AddAssign;

    use crate::math_things::bigint::UBig;

    #[test]
    fn test_fib() {
        fn fib(n: usize) -> UBig {
            if n == 0 {
                return UBig::zero();
            }

            // a is the n-2 value, b is the n-1 value
            let mut a = UBig::zero();
            let mut b = UBig::one();
            for _ in 0..n.saturating_sub(1) {
                let tmp = b.clone();
                b += &a;
                a = tmp;
            }
            b
        }
        println!("Small");
        // https://oeis.org/A000045/list
        [
            0, 1, 1, 2, 3, 5, 8, 13, 21, 34, 55, 89, 144, 233, 377, 610, 987, 1597, 2584, 4181,
            6765, 10946, 17711, 28657, 46368, 75025, 121393, 196418, 317811, 514229, 832040,
            1346269, 2178309, 3524578, 5702887, 9227465, 14930352, 24157817, 39088169, 63245986,
            102334155,
        ]
        .into_iter()
        .enumerate()
        .for_each(|(n, should)| {
            print!("Testing fib({n})={should}...");
            assert_eq!(fib(n), UBig::new(should));
            println!("Sucess!")
        });

        println!("Large");
        [
            (500, "139423224561697880139724382870407283950070256587697307264108962948325571622863290691557658876222521294125"),
            (1000, "43466557686937456435688527675040625802564660517371780402481729089536555417949051890403879840079255169295922593080322634775209689623239873322471161642996440906533187938298969649928516003704476137795166849228875"),
            (5000, "3878968454388325633701916308325905312082127714646245106160597214895550139044037097010822916462210669479293452858882973813483102008954982940361430156911478938364216563944106910214505634133706558656238254656700712525929903854933813928836378347518908762970712033337052923107693008518093849801803847813996748881765554653788291644268912980384613778969021502293082475666346224923071883324803280375039130352903304505842701147635242270210934637699104006714174883298422891491273104054328753298044273676822977244987749874555691907703880637046832794811358973739993110106219308149018570815397854379195305617510761053075688783766033667355445258844886241619210553457493675897849027988234351023599844663934853256411952221859563060475364645470760330902420806382584929156452876291575759142343809142302917491088984155209854432486594079793571316841692868039545309545388698114665082066862897420639323438488465240988742395873801976993820317174208932265468879364002630797780058759129671389634214252579116872755600360311370547754724604639987588046985178408674382863125",)
        ].into_iter().for_each(|(n, should_str)| {
            let should = UBig::from_radix(should_str, 10);
            println!("===Fib Test Start===\nfib({n}) =\n{should_str}");
            assert_eq!(fib(n), should);
            println!("Sucess!")
        });
    }

    #[test]
    fn test_add() {
        assert_eq!(
            UBig {
                digits: vec![u64::MAX; 4],
            } + UBig::one(),
            UBig {
                digits: vec![0, 0, 0, 0, 1]
            }
        );
    }

    #[test]
    fn test_sub() {
        assert_eq!(
            UBig {
                digits: vec![0, 0, 0, 0, 1]
            } - UBig::one(),
            UBig {
                digits: vec![u64::MAX; 4],
            }
        );
    }

    #[test]
    fn test_mul() {
        [
            ("50", "50", "2500"),
            (
                "18446744073709551616",
                "944284833567073",
                "17418980657497214140330136691539968",
            ),
            ("24157817", "39088169", "944284833567073"),
            (
                "3878968454388325633701916308325905312082127714646245106160597214895550139044037097010822916462210669479293452858882973813483102008954982940361430156911478938364216563944106910214505634133706558656238254656700712525929903854933813928836378347518908762970712033337052923107693008518093849801803847813996748881765554653788291644268912980384613778969021502293082475666346224923071883324803280375039130352903304505842701147635242270210934637699104006714174883298422891491273104054328753298044273676822977244987749874555691907703880637046832794811358973739993110106219308149018570815397854379195305617510761053075688783766033667355445258844886241619210553457493675897849027988234351023599844663934853256411952221859563060475364645470760330902420806382584929156452876291575759142343809142302917491088984155209854432486594079793571316841692868039545309545388698114665082066862897420639323438488465240988742395873801976993820317174208932265468879364002630797780058759129671389634214252579116872755600360311370547754724604639987588046985178408674382863125",
                "0",
                "0",
            ),
        ].into_iter().for_each(|(a_str, b_str, should_str)| {
            let a = UBig::from_radix(a_str, 10);
            let b = UBig::from_radix(b_str, 10);
            let should = UBig::from_radix(should_str, 10);
            println!("===Mul Test Start===\n{a_str}\n*\n{b_str}\n= {should_str}");
            assert_eq!(&a * &b, should);
            println!("Sucess!")
        });
    }

    #[test]
    fn test_div() {
        [
            ("13452374534985798789291", "4", "3363093633746449697322"),
            ("1", "2", "0"),
            (
                "1234012309123893557834529398123912",
                "2345789743583",
                "526054098624816102427",
            ),
        ]
        .into_iter()
        .for_each(|(a_str, b_str, should_str)| {
            let a = UBig::from_radix(a_str, 10);
            let b = UBig::from_radix(b_str, 10);
            let should = UBig::from_radix(should_str, 10);
            println!("===Div Test Start===\n{a_str}\n/\n{b_str}\n= {should_str}");
            assert_eq!(&a / &b, should);
            println!("Sucess!")
        });
    }

    #[test]
    fn test_shl_ext() {
        [(
            "13452374534985798789291",
            4,
            false,
            "215237992559772780628656",
        )]
        .into_iter()
        .for_each(|(num_str, shmt, ext, should_str)| {
            let mut num = UBig::from_radix(num_str, 10);
            let should = UBig::from_radix(should_str, 10);
            println!("===Shl Ext Test Start===\n{num_str}\n<< {shmt}\n= {should_str}");
            num.shl_assign_ext(shmt, ext);
            assert_eq!(num, should);
            println!("Sucess!")
        });
    }

    #[test]
    fn test_gcd() {
        [("1", "2", "1"), ("0", "2", "2"), ("1111111111", "11", "11") 
        , ("130694740075899648", "144", "144"),
        (
            "2725739259917615184694970466861171661541050941152902373726090171201311337136954991310274698639275906719971738885962812006335147100222417333750396456797535776317815569042989028483998628730543991560128373271244843621806093543790146132870216597267701199471052127293712805561386776439304548692652882699435341942345",
            "5165177476584119633204676289241664185765654228520721648366669380602289961455301773119811223602346252886217600059358831274737741900988397303264139564924900",
            "5"
        )
        ].into_iter().for_each(
            |(a_str, b_str, should_str)| {
                let a = UBig::from_radix(a_str, 10);
                let b = UBig::from_radix(b_str, 10);
                let should = UBig::from_radix(should_str, 10);
                println!("===GCD Test Start===\n{a_str}\n{b_str}\n= {should_str}");
                assert_eq!(UBig::gcd(a.clone(), b.clone()), should);
                assert_eq!(UBig::gcd(b, a), should);
                println!("Sucess!")
            },
        );
    }

    #[test]
    fn test_from_radix() {
        [(
            10,
            "139423224561697880139724382870407283950070256587697307264108962948325571622863290691557658876222521294125",
            UBig::from_digits(vec![
                2171430676560690477,
                536987397691362894,
                1492802675778576035,
                16460650315921838430,
                6872226595543302833,
                65273441,
            ]),
        )].into_iter().for_each(|(radix, from_str, should)| {
            let x = UBig::from_radix(from_str, radix);
            println!("===From Radix Test Start===\n{from_str}\n=\n{should:?}");
            assert_eq!(should, x);
        });
    }
}
