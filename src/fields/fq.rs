use core::convert::TryInto;
use core::fmt;
use core::ops::{Add, Mul, Neg, Sub};

use ff::PrimeField;
use rand::RngCore;
use subtle::{Choice, ConditionallySelectable, ConstantTimeEq, CtOption};

#[cfg(feature = "std")]
use lazy_static::lazy_static;

#[cfg(feature = "bits")]
use ff::{FieldBits, PrimeFieldBits};

use crate::arithmetic::{adc, mac, sbb};

#[cfg(feature = "std")]
use pasta_curves::arithmetic::{FieldExt, Group, SqrtRatio, SqrtTables};

/// This represents an element of $\mathbb{F}_q$ where
///
/// `q = 0xfffffffffffffffffffffffffffffffebaaedce6af48a03bbfd25e8cd0364141`
///
/// is the scalar field of the secp256k1 curve.
// The internal representation of this type is four 64-bit unsigned
// integers in little-endian order. `Fq` values are always in
// Montgomery form; i.e., Fq(a) = aR mod q, with R = 2^256.
#[derive(Clone, Copy, Eq)]
pub struct Fq(pub(crate) [u64; 4]);

impl fmt::Debug for Fq {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let tmp = self.to_repr();
        write!(f, "0x")?;
        for &b in tmp.iter().rev() {
            write!(f, "{:02x}", b)?;
        }
        Ok(())
    }
}

impl From<bool> for Fq {
    fn from(bit: bool) -> Fq {
        if bit {
            Fq::one()
        } else {
            Fq::zero()
        }
    }
}

impl From<u64> for Fq {
    fn from(val: u64) -> Fq {
        Fq([val, 0, 0, 0]) * R2
    }
}

impl ConstantTimeEq for Fq {
    fn ct_eq(&self, other: &Self) -> Choice {
        self.0[0].ct_eq(&other.0[0])
            & self.0[1].ct_eq(&other.0[1])
            & self.0[2].ct_eq(&other.0[2])
            & self.0[3].ct_eq(&other.0[3])
    }
}

impl PartialEq for Fq {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.ct_eq(other).unwrap_u8() == 1
    }
}

impl core::cmp::Ord for Fq {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        let left = self.to_repr();
        let right = other.to_repr();
        left.iter()
            .zip(right.iter())
            .rev()
            .find_map(|(left_byte, right_byte)| match left_byte.cmp(right_byte) {
                core::cmp::Ordering::Equal => None,
                res => Some(res),
            })
            .unwrap_or(core::cmp::Ordering::Equal)
    }
}

impl core::cmp::PartialOrd for Fq {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl ConditionallySelectable for Fq {
    fn conditional_select(a: &Self, b: &Self, choice: Choice) -> Self {
        Fq([
            u64::conditional_select(&a.0[0], &b.0[0], choice),
            u64::conditional_select(&a.0[1], &b.0[1], choice),
            u64::conditional_select(&a.0[2], &b.0[2], choice),
            u64::conditional_select(&a.0[3], &b.0[3], choice),
        ])
    }
}

/// Constant representing the modulus
/// q = 0xfffffffffffffffffffffffffffffffebaaedce6af48a03bbfd25e8cd0364141

const MODULUS: Fq = Fq([
    0xbfd25e8cd0364141,
    0xbaaedce6af48a03b,
    0xfffffffffffffffe,
    0xffffffffffffffff,
]);

/// The modulus as u32 limbs.
#[cfg(not(target_pointer_width = "64"))]
const MODULUS_LIMBS_32: [u32; 8] = [
    0xd036_4141,
    0xbfd2_5e8c,
    0xaf48_a03b,
    0xbaae_dce6,
    0xffff_fffe,
    0xffff_ffff,
    0xffff_ffff,
    0xffff_ffff,
];

impl<'a> Neg for &'a Fq {
    type Output = Fq;

    #[inline]
    fn neg(self) -> Fq {
        self.neg()
    }
}

impl Neg for Fq {
    type Output = Fq;

    #[inline]
    fn neg(self) -> Fq {
        -&self
    }
}

impl<'a, 'b> Sub<&'b Fq> for &'a Fq {
    type Output = Fq;

    #[inline]
    fn sub(self, rhs: &'b Fq) -> Fq {
        self.sub(rhs)
    }
}

impl<'a, 'b> Add<&'b Fq> for &'a Fq {
    type Output = Fq;

    #[inline]
    fn add(self, rhs: &'b Fq) -> Fq {
        self.add(rhs)
    }
}

impl<'a, 'b> Mul<&'b Fq> for &'a Fq {
    type Output = Fq;

    #[inline]
    fn mul(self, rhs: &'b Fq) -> Fq {
        self.mul(rhs)
    }
}

impl_binops_additive!(Fq, Fq);
impl_binops_multiplicative!(Fq, Fq);

/// INV = -(q^{-1} mod 2^64) mod 2^64
const INV: u64 = 0x4b0dff665588b13f;

/// R = 2^256 mod q
/// 0x14551231950b75fc4402da1732fc9bebf
const R: Fq = Fq([0x402da1732fc9bebf, 0x4551231950b75fc4, 0x1, 0]);

/// R^2 = 2^512 mod q
/// 0x9d671cd581c69bc5e697f5e45bcd07c6741496c20e7cf878896cf21467d7d140
const R2: Fq = Fq([
    0x896cf21467d7d140,
    0x741496c20e7cf878,
    0xe697f5e45bcd07c6,
    0x9d671cd581c69bc5,
]);

/// R^3 = 2^768 mod q
/// 0x555d800c18ef116db1b31347f1d0b2da0017648444d4322c7bc0cfe0e9ff41ed
const R3: Fq = Fq([
    0x7bc0cfe0e9ff41ed,
    0x0017648444d4322c,
    0xb1b31347f1d0b2da,
    0x555d800c18ef116d,
]);

impl Default for Fq {
    #[inline]
    fn default() -> Self {
        Self::zero()
    }
}

impl Fq {
    /// Returns zero, the additive identity.
    #[inline]
    pub const fn zero() -> Fq {
        Fq([0, 0, 0, 0])
    }

    /// Returns one, the multiplicative identity.
    #[inline]
    pub const fn one() -> Fq {
        R
    }

    /// Doubles this field element.
    #[inline]
    pub const fn double(&self) -> Fq {
        // TODO: This can be achieved more efficiently with a bitshift.
        self.add(self)
    }

    fn from_u512(limbs: [u64; 8]) -> Fq {
        // We reduce an arbitrary 512-bit number by decomposing it into two 256-bit digits
        // with the higher bits multiplied by 2^256. Thus, we perform two reductions
        //
        // 1. the lower bits are multiplied by R^2, as normal
        // 2. the upper bits are multiplied by R^2 * 2^256 = R^3
        //
        // and computing their sum in the field. It remains to see that arbitrary 256-bit
        // numbers can be placed into Montgomery form safely using the reduction. The
        // reduction works so long as the product is less than R=2^256 multiplied by
        // the modulus. This holds because for any `c` smaller than the modulus, we have
        // that (2^256 - 1)*c is an acceptable product for the reduction. Therefore, the
        // reduction always works so long as `c` is in the field; in this case it is either the
        // constant `R2` or `R3`.
        let d0 = Fq([limbs[0], limbs[1], limbs[2], limbs[3]]);
        let d1 = Fq([limbs[4], limbs[5], limbs[6], limbs[7]]);
        // Convert to Montgomery form
        d0 * R2 + d1 * R3
    }

    /// Converts from an integer represented in little endian
    /// into its (congruent) `Fq` representation.
    pub const fn from_raw(val: [u64; 4]) -> Self {
        (&Fq(val)).mul(&R2)
    }

    /// Squares this element.
    #[inline]
    pub const fn square(&self) -> Fq {
        let (r1, carry) = mac(0, self.0[0], self.0[1], 0);
        let (r2, carry) = mac(0, self.0[0], self.0[2], carry);
        let (r3, r4) = mac(0, self.0[0], self.0[3], carry);

        let (r3, carry) = mac(r3, self.0[1], self.0[2], 0);
        let (r4, r5) = mac(r4, self.0[1], self.0[3], carry);

        let (r5, r6) = mac(r5, self.0[2], self.0[3], 0);

        let r7 = r6 >> 63;
        let r6 = (r6 << 1) | (r5 >> 63);
        let r5 = (r5 << 1) | (r4 >> 63);
        let r4 = (r4 << 1) | (r3 >> 63);
        let r3 = (r3 << 1) | (r2 >> 63);
        let r2 = (r2 << 1) | (r1 >> 63);
        let r1 = r1 << 1;

        let (r0, carry) = mac(0, self.0[0], self.0[0], 0);
        let (r1, carry) = adc(0, r1, carry);
        let (r2, carry) = mac(r2, self.0[1], self.0[1], carry);
        let (r3, carry) = adc(0, r3, carry);
        let (r4, carry) = mac(r4, self.0[2], self.0[2], carry);
        let (r5, carry) = adc(0, r5, carry);
        let (r6, carry) = mac(r6, self.0[3], self.0[3], carry);
        let (r7, _) = adc(0, r7, carry);

        Fq::montgomery_reduce(r0, r1, r2, r3, r4, r5, r6, r7)
    }

    #[allow(clippy::too_many_arguments)]
    #[inline(always)]
    const fn montgomery_reduce(
        r0: u64,
        r1: u64,
        r2: u64,
        r3: u64,
        r4: u64,
        r5: u64,
        r6: u64,
        r7: u64,
    ) -> Self {
        // The Montgomery reduction here is based on Algorithm 14.32 in
        // Handbook of Applied Cryptography
        // <http://cacr.uwaterloo.ca/hac/about/chap14.pdf>.

        let k = r0.wrapping_mul(INV);
        let (_, carry) = mac(r0, k, MODULUS.0[0], 0);
        let (r1, carry) = mac(r1, k, MODULUS.0[1], carry);
        let (r2, carry) = mac(r2, k, MODULUS.0[2], carry);
        let (r3, carry) = mac(r3, k, MODULUS.0[3], carry);
        let (r4, carry2) = adc(r4, 0, carry);

        let k = r1.wrapping_mul(INV);
        let (_, carry) = mac(r1, k, MODULUS.0[0], 0);
        let (r2, carry) = mac(r2, k, MODULUS.0[1], carry);
        let (r3, carry) = mac(r3, k, MODULUS.0[2], carry);
        let (r4, carry) = mac(r4, k, MODULUS.0[3], carry);
        let (r5, carry2) = adc(r5, carry2, carry);

        let k = r2.wrapping_mul(INV);
        let (_, carry) = mac(r2, k, MODULUS.0[0], 0);
        let (r3, carry) = mac(r3, k, MODULUS.0[1], carry);
        let (r4, carry) = mac(r4, k, MODULUS.0[2], carry);
        let (r5, carry) = mac(r5, k, MODULUS.0[3], carry);
        let (r6, carry2) = adc(r6, carry2, carry);

        let k = r3.wrapping_mul(INV);
        let (_, carry) = mac(r3, k, MODULUS.0[0], 0);
        let (r4, carry) = mac(r4, k, MODULUS.0[1], carry);
        let (r5, carry) = mac(r5, k, MODULUS.0[2], carry);
        let (r6, carry) = mac(r6, k, MODULUS.0[3], carry);
        let (r7, carry2) = adc(r7, carry2, carry);

        // Result may be within MODULUS of the correct value
        let (d0, borrow) = sbb(r4, MODULUS.0[0], 0);
        let (d1, borrow) = sbb(r5, MODULUS.0[1], borrow);
        let (d2, borrow) = sbb(r6, MODULUS.0[2], borrow);
        let (d3, borrow) = sbb(r7, MODULUS.0[3], borrow);
        let (_, borrow) = sbb(carry2, 0, borrow);

        let (d0, carry) = adc(d0, MODULUS.0[0] & borrow, 0);
        let (d1, carry) = adc(d1, MODULUS.0[1] & borrow, carry);
        let (d2, carry) = adc(d2, MODULUS.0[2] & borrow, carry);
        let (d3, _) = adc(d3, MODULUS.0[3] & borrow, carry);

        Fq([d0, d1, d2, d3])
    }

    /// Multiplies `rhs` by `self`, returning the result.
    #[inline]
    pub const fn mul(&self, rhs: &Self) -> Self {
        // Schoolbook multiplication

        let (r0, carry) = mac(0, self.0[0], rhs.0[0], 0);
        let (r1, carry) = mac(0, self.0[0], rhs.0[1], carry);
        let (r2, carry) = mac(0, self.0[0], rhs.0[2], carry);
        let (r3, r4) = mac(0, self.0[0], rhs.0[3], carry);

        let (r1, carry) = mac(r1, self.0[1], rhs.0[0], 0);
        let (r2, carry) = mac(r2, self.0[1], rhs.0[1], carry);
        let (r3, carry) = mac(r3, self.0[1], rhs.0[2], carry);
        let (r4, r5) = mac(r4, self.0[1], rhs.0[3], carry);

        let (r2, carry) = mac(r2, self.0[2], rhs.0[0], 0);
        let (r3, carry) = mac(r3, self.0[2], rhs.0[1], carry);
        let (r4, carry) = mac(r4, self.0[2], rhs.0[2], carry);
        let (r5, r6) = mac(r5, self.0[2], rhs.0[3], carry);

        let (r3, carry) = mac(r3, self.0[3], rhs.0[0], 0);
        let (r4, carry) = mac(r4, self.0[3], rhs.0[1], carry);
        let (r5, carry) = mac(r5, self.0[3], rhs.0[2], carry);
        let (r6, r7) = mac(r6, self.0[3], rhs.0[3], carry);

        Fq::montgomery_reduce(r0, r1, r2, r3, r4, r5, r6, r7)
    }

    /// Subtracts `rhs` from `self`, returning the result.
    #[inline]
    pub const fn sub(&self, rhs: &Self) -> Self {
        let (d0, borrow) = sbb(self.0[0], rhs.0[0], 0);
        let (d1, borrow) = sbb(self.0[1], rhs.0[1], borrow);
        let (d2, borrow) = sbb(self.0[2], rhs.0[2], borrow);
        let (d3, borrow) = sbb(self.0[3], rhs.0[3], borrow);

        // If underflow occurred on the final limb, borrow = 0xfff...fff, otherwise
        // borrow = 0x000...000. Thus, we use it as a mask to conditionally add the modulus.
        let (d0, carry) = adc(d0, MODULUS.0[0] & borrow, 0);
        let (d1, carry) = adc(d1, MODULUS.0[1] & borrow, carry);
        let (d2, carry) = adc(d2, MODULUS.0[2] & borrow, carry);
        let (d3, _) = adc(d3, MODULUS.0[3] & borrow, carry);

        Fq([d0, d1, d2, d3])
    }

    /// Adds `rhs` to `self`, returning the result.
    #[inline]
    pub const fn add(&self, rhs: &Self) -> Self {
        let (d0, carry) = adc(self.0[0], rhs.0[0], 0);
        let (d1, carry) = adc(self.0[1], rhs.0[1], carry);
        let (d2, carry) = adc(self.0[2], rhs.0[2], carry);
        let (d3, carry) = adc(self.0[3], rhs.0[3], carry);

        // Attempt to subtract the modulus, to ensure the value
        // is smaller than the modulus.
        let (d0, borrow) = sbb(d0, MODULUS.0[0], 0);
        let (d1, borrow) = sbb(d1, MODULUS.0[1], borrow);
        let (d2, borrow) = sbb(d2, MODULUS.0[2], borrow);
        let (d3, borrow) = sbb(d3, MODULUS.0[3], borrow);
        let (_, borrow) = sbb(carry, 0, borrow);

        let (d0, carry) = adc(d0, MODULUS.0[0] & borrow, 0);
        let (d1, carry) = adc(d1, MODULUS.0[1] & borrow, carry);
        let (d2, carry) = adc(d2, MODULUS.0[2] & borrow, carry);
        let (d3, _) = adc(d3, MODULUS.0[3] & borrow, carry);

        Fq([d0, d1, d2, d3])
    }

    /// Negates `self`.
    #[inline]
    pub const fn neg(&self) -> Self {
        // Subtract `self` from `MODULUS` to negate. Ignore the final
        // borrow because it cannot underflow; self is guaranteed to
        // be in the field.
        let (d0, borrow) = sbb(MODULUS.0[0], self.0[0], 0);
        let (d1, borrow) = sbb(MODULUS.0[1], self.0[1], borrow);
        let (d2, borrow) = sbb(MODULUS.0[2], self.0[2], borrow);
        let (d3, _) = sbb(MODULUS.0[3], self.0[3], borrow);

        // `tmp` could be `MODULUS` if `self` was zero. Create a mask that is
        // zero if `self` was zero, and `u64::max_value()` if self was nonzero.
        let mask = (((self.0[0] | self.0[1] | self.0[2] | self.0[3]) == 0) as u64).wrapping_sub(1);

        Fq([d0 & mask, d1 & mask, d2 & mask, d3 & mask])
    }
}

impl From<Fq> for [u8; 32] {
    fn from(value: Fq) -> [u8; 32] {
        value.to_repr()
    }
}

impl<'a> From<&'a Fq> for [u8; 32] {
    fn from(value: &'a Fq) -> [u8; 32] {
        value.to_repr()
    }
}

#[cfg(feature = "std")]
impl Group for Fq {
    type Scalar = Fq;

    fn group_zero() -> Self {
        Self::zero()
    }
    fn group_add(&mut self, rhs: &Self) {
        *self += *rhs;
    }
    fn group_sub(&mut self, rhs: &Self) {
        *self -= *rhs;
    }
    fn group_scale(&mut self, by: &Self::Scalar) {
        *self *= *by;
    }
}

impl ff::Field for Fq {
    fn random(mut rng: impl RngCore) -> Self {
        Self::from_u512([
            rng.next_u64(),
            rng.next_u64(),
            rng.next_u64(),
            rng.next_u64(),
            rng.next_u64(),
            rng.next_u64(),
            rng.next_u64(),
            rng.next_u64(),
        ])
    }

    fn zero() -> Self {
        Self::zero()
    }

    fn one() -> Self {
        Self::one()
    }

    fn double(&self) -> Self {
        self.double()
    }

    #[inline(always)]
    fn square(&self) -> Self {
        self.square()
    }

    /// Computes the square root of this element, if it exists.
    fn sqrt(&self) -> CtOption<Self> {
        #[cfg(feature = "std")]
        {
            let (is_square, res) = FQ_TABLES.sqrt_alt(self);
            CtOption::new(res, is_square)
        }

        #[cfg(not(feature = "std"))]
        crate::arithmetic::sqrt_tonelli_shanks(self, &T_MINUS1_OVER2)
    }

    /// Computes the multiplicative inverse of this element,
    /// failing if the element is zero.
    fn invert(&self) -> CtOption<Self> {
        let tmp = self.pow_vartime(&[
            0xbfd25e8cd036413f,
            0xbaaedce6af48a03b,
            0xfffffffffffffffe,
            0xffffffffffffffff,
        ]);

        CtOption::new(tmp, !self.ct_eq(&Self::zero()))
    }

    fn pow_vartime<S: AsRef<[u64]>>(&self, exp: S) -> Self {
        let mut res = Self::one();
        let mut found_one = false;
        for e in exp.as_ref().iter().rev() {
            for i in (0..64).rev() {
                if found_one {
                    res = res.square();
                }

                if ((*e >> i) & 1) == 1 {
                    found_one = true;
                    res *= self;
                }
            }
        }
        res
    }
}

impl ff::PrimeField for Fq {
    type Repr = [u8; 32];

    const NUM_BITS: u32 = 256;
    const CAPACITY: u32 = 255;
    const S: u32 = 6;

    fn from_repr(repr: Self::Repr) -> CtOption<Self> {
        let mut tmp = Fq([0, 0, 0, 0]);

        tmp.0[0] = u64::from_le_bytes(repr[0..8].try_into().unwrap());
        tmp.0[1] = u64::from_le_bytes(repr[8..16].try_into().unwrap());
        tmp.0[2] = u64::from_le_bytes(repr[16..24].try_into().unwrap());
        tmp.0[3] = u64::from_le_bytes(repr[24..32].try_into().unwrap());

        // Try to subtract the modulus
        let (_, borrow) = sbb(tmp.0[0], MODULUS.0[0], 0);
        let (_, borrow) = sbb(tmp.0[1], MODULUS.0[1], borrow);
        let (_, borrow) = sbb(tmp.0[2], MODULUS.0[2], borrow);
        let (_, borrow) = sbb(tmp.0[3], MODULUS.0[3], borrow);

        // If the element is smaller than MODULUS then the
        // subtraction will underflow, producing a borrow value
        // of 0xffff...ffff. Otherwise, it'll be zero.
        let is_some = (borrow as u8) & 1;

        // Convert to Montgomery form by computing
        // (a.R^0 * R^2) / R = a.R
        tmp *= &R2;

        CtOption::new(tmp, Choice::from(is_some))
    }

    fn to_repr(&self) -> Self::Repr {
        // Turn into canonical form by computing
        // (a.R) / R = a
        let tmp = Fq::montgomery_reduce(self.0[0], self.0[1], self.0[2], self.0[3], 0, 0, 0, 0);

        let mut res = [0; 32];
        res[0..8].copy_from_slice(&tmp.0[0].to_le_bytes());
        res[8..16].copy_from_slice(&tmp.0[1].to_le_bytes());
        res[16..24].copy_from_slice(&tmp.0[2].to_le_bytes());
        res[24..32].copy_from_slice(&tmp.0[3].to_le_bytes());

        res
    }

    fn is_odd(&self) -> Choice {
        Choice::from(self.to_repr()[0] & 1)
    }

    fn multiplicative_generator() -> Self {
        unimplemented!();
    }

    fn root_of_unity() -> Self {
        unimplemented!();
    }
}

#[cfg(all(feature = "bits", not(target_pointer_width = "64")))]
type ReprBits = [u32; 8];

#[cfg(all(feature = "bits", target_pointer_width = "64"))]
type ReprBits = [u64; 4];

#[cfg(feature = "bits")]
impl PrimeFieldBits for Fq {
    type ReprBits = ReprBits;

    fn to_le_bits(&self) -> FieldBits<Self::ReprBits> {
        let bytes = self.to_repr();

        #[cfg(not(target_pointer_width = "64"))]
        let limbs = [
            u32::from_le_bytes(bytes[0..4].try_into().unwrap()),
            u32::from_le_bytes(bytes[4..8].try_into().unwrap()),
            u32::from_le_bytes(bytes[8..12].try_into().unwrap()),
            u32::from_le_bytes(bytes[12..16].try_into().unwrap()),
            u32::from_le_bytes(bytes[16..20].try_into().unwrap()),
            u32::from_le_bytes(bytes[20..24].try_into().unwrap()),
            u32::from_le_bytes(bytes[24..28].try_into().unwrap()),
            u32::from_le_bytes(bytes[28..32].try_into().unwrap()),
        ];

        #[cfg(target_pointer_width = "64")]
        let limbs = [
            u64::from_le_bytes(bytes[0..8].try_into().unwrap()),
            u64::from_le_bytes(bytes[8..16].try_into().unwrap()),
            u64::from_le_bytes(bytes[16..24].try_into().unwrap()),
            u64::from_le_bytes(bytes[24..32].try_into().unwrap()),
        ];

        FieldBits::new(limbs)
    }

    fn char_le_bits() -> FieldBits<Self::ReprBits> {
        #[cfg(not(target_pointer_width = "64"))]
        {
            FieldBits::new(MODULUS_LIMBS_32)
        }

        #[cfg(target_pointer_width = "64")]
        FieldBits::new(MODULUS.0)
    }
}

#[cfg(feature = "std")]
lazy_static! {
    // The perfect hash parameters are found by `squareroottab.sage` in zcash/pasta.
    static ref FQ_TABLES: SqrtTables<Fq> = SqrtTables::new(0x116A9E, 1206);
}

#[cfg(feature = "std")]
impl SqrtRatio for Fq {
    const T_MINUS1_OVER2: [u64; 4] = [0, 0, 0, 0];

    fn pow_by_t_minus1_over2(&self) -> Self {
        unimplemented!()
    }

    fn get_lower_32(&self) -> u32 {
        // TODO: don't reduce, just hash the Montgomery form. (Requires rebuilding perfect hash table.)
        let tmp = Fq::montgomery_reduce(self.0[0], self.0[1], self.0[2], self.0[3], 0, 0, 0, 0);

        tmp.0[0] as u32
    }

    fn sqrt_ratio(num: &Self, div: &Self) -> (Choice, Self) {
        FQ_TABLES.sqrt_ratio(num, div)
    }

    fn sqrt_alt(&self) -> (Choice, Self) {
        FQ_TABLES.sqrt_alt(self)
    }
}

#[cfg(feature = "std")]
impl FieldExt for Fq {
    const MODULUS: &'static str =
        "0xfffffffffffffffffffffffffffffffebaaedce6af48a03bbfd25e8cd0364141";
    const ROOT_OF_UNITY_INV: Self = Self::zero();
    const DELTA: Self = Self::zero();
    const TWO_INV: Self = Self::from_raw([
        0xdfe92f46681b20a1,
        0x5d576e7357a4501d,
        0xffffffffffffffff,
        0x7fffffffffffffff,
    ]);

    const ZETA: Self = Self::zero();

    fn from_u128(v: u128) -> Self {
        Fq::from_raw([v as u64, (v >> 64) as u64, 0, 0])
    }

    /// Converts a 512-bit little endian integer into
    /// a `Fq` by reducing by the modulus.
    fn from_bytes_wide(bytes: &[u8; 64]) -> Fq {
        Fq::from_u512([
            u64::from_le_bytes(bytes[0..8].try_into().unwrap()),
            u64::from_le_bytes(bytes[8..16].try_into().unwrap()),
            u64::from_le_bytes(bytes[16..24].try_into().unwrap()),
            u64::from_le_bytes(bytes[24..32].try_into().unwrap()),
            u64::from_le_bytes(bytes[32..40].try_into().unwrap()),
            u64::from_le_bytes(bytes[40..48].try_into().unwrap()),
            u64::from_le_bytes(bytes[48..56].try_into().unwrap()),
            u64::from_le_bytes(bytes[56..64].try_into().unwrap()),
        ])
    }

    fn get_lower_128(&self) -> u128 {
        let tmp = Fq::montgomery_reduce(self.0[0], self.0[1], self.0[2], self.0[3], 0, 0, 0, 0);

        u128::from(tmp.0[0]) | (u128::from(tmp.0[1]) << 64)
    }
}

#[cfg(all(test, feature = "std"))]
use ff::Field;
#[cfg(all(test, feature = "std"))]
use num_bigint::BigUint;
#[cfg(all(test, feature = "std"))]
use num_traits::Num;

#[test]
fn test_inv() {
    // Compute -(r^{-1} mod 2^64) mod 2^64 by exponentiating
    // by totient(2**64) - 1

    let mut inv = 1u64;
    for _ in 0..63 {
        inv = inv.wrapping_mul(inv);
        inv = inv.wrapping_mul(MODULUS.0[0]);
    }
    inv = inv.wrapping_neg();

    assert_eq!(inv, INV);
}

#[cfg(feature = "std")]
#[test]
fn test_inv_2() {
    assert_eq!(Fq::TWO_INV, Fq::from(2).invert().unwrap());
}

#[cfg(test)]
fn fp_to_big(fe: Fq) -> BigUint {
    let u: [u8; 32] = fe.to_repr().into();
    BigUint::from_bytes_le(&u[..])
}

#[cfg(test)]
fn big_modulus() -> BigUint {
    let modulus_big = BigUint::from_str_radix(
        "fffffffffffffffffffffffffffffffebaaedce6af48a03bbfd25e8cd0364141",
        16,
    )
    .unwrap();
    modulus_big
}

#[cfg(feature = "std")]
#[test]
fn test_add_against_big() {
    let modulus = &big_modulus();
    for _ in 0..1000 {
        let a = Fq::rand();
        let b = Fq::rand();
        let c = a + b;

        let c_big_0 = fp_to_big(c);
        let c_big_1 = (fp_to_big(a) + fp_to_big(b)) % modulus;

        assert_eq!(c_big_0, c_big_1);
    }
}

#[cfg(feature = "std")]
#[test]
fn test_sub_against_big() {
    let modulus = &big_modulus();
    for _ in 0..1000 {
        let a = Fq::rand();
        let b = Fq::rand();
        let c = a - b;

        let c_big_0 = fp_to_big(c);
        let c_big_1 = fp_to_big(a) + modulus;
        let c_big_1 = (c_big_1 - fp_to_big(b)) % modulus;

        assert_eq!(c_big_0, c_big_1);
    }
}

#[cfg(feature = "std")]
#[test]
fn test_mul_against_big() {
    let modulus = &big_modulus();
    for _ in 0..1000 {
        let a = Fq::rand();
        let b = Fq::rand();
        let c = a * b;
        let c_big_0 = fp_to_big(c);
        let c_big_1 = (fp_to_big(a) * fp_to_big(b)) % modulus;
        assert_eq!(c_big_0, c_big_1);
    }
}

#[cfg(feature = "std")]
#[test]
fn test_square_against_big() {
    let modulus = &big_modulus();
    for _ in 0..1000 {
        let a = Fq::rand();
        let c = a.square();
        let c_big_0 = fp_to_big(c);
        let c_big_1 = (fp_to_big(a) * fp_to_big(a)) % modulus;
        assert_eq!(c_big_0, c_big_1);
    }
}
