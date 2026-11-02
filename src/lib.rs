//! Unsigned 256-bit arithmetic. Ordinary arithmetic wraps, matching the pinned
//! Swift UInt256; use checked methods when overflow is a refusal condition.
#![forbid(unsafe_code)]
use num_bigint::BigUint;
use std::{fmt, ops::*, str::FromStr};

/// Network-order bytes; ordering is unsigned numeric ordering.
#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UInt256(pub [u8; 32]);
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    InvalidInteger,
    Overflow,
    DivisionByZero,
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for Error {}
impl UInt256 {
    pub const ZERO: Self = Self([0; 32]);
    pub const MAX: Self = Self([255; 32]);
    pub const BITS: u32 = 256;
    pub const fn from_be_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }
    pub const fn to_be_bytes(self) -> [u8; 32] {
        self.0
    }
    pub fn from_le_bytes(mut bytes: [u8; 32]) -> Self {
        bytes.reverse();
        Self(bytes)
    }
    pub fn to_le_bytes(self) -> [u8; 32] {
        let mut bytes = self.0;
        bytes.reverse();
        bytes
    }
    /// Swift's array initializer pads on the left, then retains the first four
    /// words when more than four are supplied. Words are most significant first.
    pub fn from_swift_words(words: &[u64]) -> Self {
        let mut out = [0; 32];
        let offset = 4usize.saturating_sub(words.len());
        for (i, word) in words.iter().take(4).enumerate() {
            out[(offset + i) * 8..(offset + i + 1) * 8].copy_from_slice(&word.to_be_bytes());
        }
        Self(out)
    }
    pub fn words_be(self) -> [u64; 4] {
        std::array::from_fn(|i| u64::from_be_bytes(self.0[i * 8..(i + 1) * 8].try_into().unwrap()))
    }
    /// Swift's concrete Int initializer sign-extends negative values.
    pub fn from_i64_wrapping(value: i64) -> Self {
        let mut out = [if value < 0 { 255 } else { 0 }; 32];
        out[24..].copy_from_slice(&value.to_be_bytes());
        Self(out)
    }
    fn big(self) -> BigUint {
        BigUint::from_bytes_be(&self.0)
    }
    fn modulus() -> BigUint {
        BigUint::from(1u8) << 256usize
    }
    fn low(value: &BigUint) -> Self {
        let bytes = value.to_bytes_be();
        let len = bytes.len().min(32);
        let mut out = [0; 32];
        out[32 - len..].copy_from_slice(&bytes[bytes.len() - len..]);
        Self(out)
    }
    pub fn from_str_radix(text: &str, radix: u32) -> Result<Self, Error> {
        if !(2..=36).contains(&radix) {
            return Err(Error::InvalidInteger);
        }
        let value = BigUint::parse_bytes(text.strip_prefix('+').unwrap_or(text).as_bytes(), radix)
            .ok_or(Error::InvalidInteger)?;
        if value.bits() > 256 {
            Err(Error::Overflow)
        } else {
            Ok(Self::low(&value))
        }
    }
    pub fn to_string_radix(self, radix: u32) -> Result<String, Error> {
        if !(2..=36).contains(&radix) {
            return Err(Error::InvalidInteger);
        }
        Ok(self.big().to_str_radix(radix))
    }
    pub fn to_hex_string(self) -> String {
        self.0.iter().map(|b| format!("{b:02x}")).collect()
    }
    pub fn count_ones(self) -> u32 {
        self.0.iter().map(|b| b.count_ones()).sum()
    }
    pub fn leading_zeros(self) -> u32 {
        let mut n = 0;
        for b in self.0 {
            n += b.leading_zeros();
            if b != 0 {
                break;
            }
        }
        n
    }
    pub fn trailing_zeros(self) -> u32 {
        let mut n = 0;
        for b in self.0.iter().rev() {
            n += b.trailing_zeros();
            if *b != 0 {
                break;
            }
        }
        n
    }
    pub fn swap_bytes(self) -> Self {
        Self(self.to_le_bytes())
    }
    pub fn overflowing_add(self, rhs: Self) -> (Self, bool) {
        let n = self.big() + rhs.big();
        (Self::low(&n), n.bits() > 256)
    }
    pub fn overflowing_sub(self, rhs: Self) -> (Self, bool) {
        let a = self.big();
        let b = rhs.big();
        let overflow = a < b;
        (
            Self::low(
                &(if overflow {
                    Self::modulus() + a - b
                } else {
                    a - b
                }),
            ),
            overflow,
        )
    }
    pub fn overflowing_mul(self, rhs: Self) -> (Self, bool) {
        let n = self.big() * rhs.big();
        (Self::low(&n), n.bits() > 256)
    }
    /// Division by zero reports the dividend unchanged, matching Swift.
    pub fn overflowing_div(self, rhs: Self) -> (Self, bool) {
        if rhs == Self::ZERO {
            (self, true)
        } else {
            (Self::low(&(self.big() / rhs.big())), false)
        }
    }
    pub fn overflowing_rem(self, rhs: Self) -> (Self, bool) {
        if rhs == Self::ZERO {
            (self, true)
        } else {
            (Self::low(&(self.big() % rhs.big())), false)
        }
    }
    pub fn wrapping_add(self, rhs: Self) -> Self {
        self.overflowing_add(rhs).0
    }
    pub fn wrapping_sub(self, rhs: Self) -> Self {
        self.overflowing_sub(rhs).0
    }
    pub fn wrapping_mul(self, rhs: Self) -> Self {
        self.overflowing_mul(rhs).0
    }
    pub fn saturating_add(self, rhs: Self) -> Self {
        self.checked_add(rhs).unwrap_or(Self::MAX)
    }
    pub fn saturating_sub(self, rhs: Self) -> Self {
        self.checked_sub(rhs).unwrap_or(Self::ZERO)
    }
    pub fn saturating_mul(self, rhs: Self) -> Self {
        self.checked_mul(rhs).unwrap_or(Self::MAX)
    }
    /// Ordinary Swift division returns (0,0) on a zero divisor. The reporting
    /// and checked methods above expose refusal instead of that sentinel.
    pub fn div_rem(self, rhs: Self) -> (Self, Self) {
        if rhs == Self::ZERO {
            (Self::ZERO, Self::ZERO)
        } else {
            (self.overflowing_div(rhs).0, self.overflowing_rem(rhs).0)
        }
    }
    pub fn checked_add(self, rhs: Self) -> Option<Self> {
        let (v, o) = self.overflowing_add(rhs);
        (!o).then_some(v)
    }
    pub fn checked_sub(self, rhs: Self) -> Option<Self> {
        let (v, o) = self.overflowing_sub(rhs);
        (!o).then_some(v)
    }
    pub fn checked_mul(self, rhs: Self) -> Option<Self> {
        let (v, o) = self.overflowing_mul(rhs);
        (!o).then_some(v)
    }
    pub fn checked_div(self, rhs: Self) -> Option<Self> {
        let (v, o) = self.overflowing_div(rhs);
        (!o).then_some(v)
    }
    pub fn checked_rem(self, rhs: Self) -> Option<Self> {
        let (v, o) = self.overflowing_rem(rhs);
        (!o).then_some(v)
    }
    pub fn full_mul(self, rhs: Self) -> (Self, Self) {
        let n = self.big() * rhs.big();
        (Self::low(&(&n >> 256usize)), Self::low(&n))
    }
    /// Divide a 512-bit dividend, refusing a quotient that cannot fit UInt256.
    pub fn full_div(self, high: Self, low: Self) -> Result<(Self, Self), Error> {
        if self == Self::ZERO {
            return Err(Error::DivisionByZero);
        }
        let n = (high.big() << 256usize) + low.big();
        let d = self.big();
        let q = &n / &d;
        if q.bits() > 256 {
            return Err(Error::Overflow);
        }
        Ok((Self::low(&q), Self::low(&(n % d))))
    }
    /// Swift's Int shifts return zero for negative or >=256 counts.
    pub fn shift_left(self, count: i64) -> Self {
        if !(0..256).contains(&count) {
            Self::ZERO
        } else {
            Self::low(&(self.big() << count as usize))
        }
    }
    pub fn shift_right(self, count: i64) -> Self {
        if !(0..256).contains(&count) {
            Self::ZERO
        } else {
            Self::low(&(self.big() >> count as usize))
        }
    }
}
impl From<u64> for UInt256 {
    fn from(v: u64) -> Self {
        Self::from_swift_words(&[v])
    }
}
impl FromStr for UInt256 {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Error> {
        Self::from_str_radix(s, 10)
    }
}
impl fmt::Display for UInt256 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_hex_string())
    }
}
impl fmt::Debug for UInt256 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "UInt256(0x{})", self.to_hex_string())
    }
}
macro_rules! arithmetic {
    ($trait:ident,$method:ident,$operation:ident) => {
        impl $trait for UInt256 {
            type Output = Self;
            fn $method(self, rhs: Self) -> Self {
                self.$operation(rhs).0
            }
        }
    };
}
arithmetic!(Add, add, overflowing_add);
arithmetic!(Sub, sub, overflowing_sub);
arithmetic!(Mul, mul, overflowing_mul);
impl Div for UInt256 {
    type Output = Self;
    fn div(self, rhs: Self) -> Self {
        self.checked_div(rhs).unwrap_or(Self::ZERO)
    }
}
impl Rem for UInt256 {
    type Output = Self;
    fn rem(self, rhs: Self) -> Self {
        self.checked_rem(rhs).unwrap_or(Self::ZERO)
    }
}
macro_rules! bitwise {($trait:ident,$method:ident,$op:tt)=>{impl $trait for UInt256{type Output=Self;fn $method(self,rhs:Self)->Self{Self(std::array::from_fn(|i| self.0[i] $op rhs.0[i]))}}};}
bitwise!(BitAnd,bitand,&);
bitwise!(BitOr,bitor,|);
bitwise!(BitXor,bitxor,^);
impl Not for UInt256 {
    type Output = Self;
    fn not(self) -> Self {
        Self(self.0.map(|b| !b))
    }
}
impl Shl<u32> for UInt256 {
    type Output = Self;
    fn shl(self, n: u32) -> Self {
        self.shift_left(n.into())
    }
}
impl Shr<u32> for UInt256 {
    type Output = Self;
    fn shr(self, n: u32) -> Self {
        self.shift_right(n.into())
    }
}
macro_rules! assign {($trait:ident,$method:ident,$op:tt)=>{impl $trait for UInt256{fn $method(&mut self,rhs:Self){*self=*self $op rhs;}}};}
assign!(AddAssign,add_assign,+);
assign!(SubAssign,sub_assign,-);
assign!(MulAssign,mul_assign,*);
assign!(DivAssign,div_assign,/);
assign!(RemAssign,rem_assign,%);
assign!(BitAndAssign,bitand_assign,&);
assign!(BitOrAssign,bitor_assign,|);
assign!(BitXorAssign,bitxor_assign,^);
