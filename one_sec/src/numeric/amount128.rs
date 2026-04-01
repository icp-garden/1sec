use ethnum::u256;
use num_traits::ToPrimitive;
use rlp::RlpStream;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::cmp::Ordering;
use std::fmt;
use std::marker::PhantomData;
use std::ops::Rem;

/// A 128-bit number with safe arithmetic.
pub struct Amount128<Unit>(u128, PhantomData<Unit>);

impl<Unit> Amount128<Unit> {
    pub const ZERO: Self = Self(0, PhantomData);
    pub const ONE: Self = Self(1, PhantomData);
    pub const TWO: Self = Self(2, PhantomData);
    pub const MAX: Self = Self(u128::MAX, PhantomData);

    /// `new` is a synonym for `from` that can be evaluated in
    /// compile time. The main use-case of this functions is defining
    /// constants.
    #[inline]
    pub const fn new(value: u128) -> Amount128<Unit> {
        Self(value, PhantomData)
    }

    #[inline]
    const fn from_inner(value: u128) -> Self {
        Self(value, PhantomData)
    }

    pub const fn into_inner(self) -> u128 {
        self.0
    }

    pub fn from_be_bytes(bytes: [u8; 16]) -> Self {
        Self::from_inner(u128::from_be_bytes(bytes))
    }

    pub fn to_be_bytes(self) -> [u8; 16] {
        self.0.to_be_bytes()
    }

    pub fn checked_add(self, other: Self) -> Option<Self> {
        self.0.checked_add(other.0).map(Self::from_inner)
    }

    #[inline]
    pub fn add(self, other: Self, err: &str) -> Self {
        match self.checked_add(other) {
            Some(result) => result,
            None => {
                panic!("{}: {} + {}", err, self.0, other.0);
            }
        }
    }

    pub fn checked_increment(&self) -> Option<Self> {
        self.checked_add(Self::ONE)
    }

    #[inline]
    pub fn increment(self, err: &str) -> Self {
        match self.checked_increment() {
            Some(result) => result,
            None => {
                panic!("{}: {}++", err, self.0);
            }
        }
    }

    pub fn checked_decrement(&self) -> Option<Self> {
        self.checked_sub(Self::ONE)
    }

    #[inline]
    pub fn decrement(self, err: &str) -> Self {
        match self.checked_decrement() {
            Some(result) => result,
            None => {
                panic!("{}: {}--", err, self.0);
            }
        }
    }
    pub fn checked_sub(self, other: Self) -> Option<Self> {
        self.0.checked_sub(other.0).map(Self::from_inner)
    }

    #[inline]
    pub fn sub(self, other: Self, err: &str) -> Self {
        match self.checked_sub(other) {
            Some(result) => result,
            None => {
                panic!("{}: {} - {}", err, self.0, other.0);
            }
        }
    }

    pub fn change_units<NewUnits>(self) -> Amount128<NewUnits> {
        Amount128::<NewUnits>::from_inner(self.0)
    }

    pub fn checked_mul<T: Into<u128>>(self, factor: T) -> Option<Self> {
        self.0.checked_mul(factor.into()).map(Self::from_inner)
    }

    #[inline]
    pub fn mul<T: Into<u128>>(self, factor: T, err: &str) -> Self {
        let factor: u128 = factor.into();
        match self.checked_mul(factor) {
            Some(result) => result,
            None => {
                panic!("{}: {} * {}", err, self.0, factor);
            }
        }
    }

    pub fn checked_div_ceil<T: Into<u128>>(self, rhs: T) -> Option<Self> {
        let rhs = rhs.into();
        if rhs == 0 {
            return None;
        }
        let (quotient, remainder) = (self.0.div_euclid(rhs), self.0.rem(&rhs));
        if remainder == 0 {
            Some(Self::from_inner(quotient))
        } else {
            Self::from_inner(quotient).checked_increment()
        }
    }

    #[inline]
    pub fn div_ceil<T: Into<u128>>(self, rhs: T, err: &str) -> Self {
        let rhs: u128 = rhs.into();
        match self.checked_div_ceil(rhs) {
            Some(result) => result,
            None => {
                panic!("{}: {} / {}", err, self.0, rhs);
            }
        }
    }

    pub fn checked_div_floor<T: Into<u128>>(self, rhs: T) -> Option<Self> {
        let rhs = rhs.into();
        if rhs == 0 {
            return None;
        }
        let quotient = self.0.div_euclid(rhs);
        Some(Self::from_inner(quotient))
    }

    #[inline]
    pub fn div_floor<T: Into<u128>>(self, rhs: T, err: &str) -> Self {
        let rhs: u128 = rhs.into();
        match self.checked_div_floor(rhs) {
            Some(result) => result,
            None => {
                panic!("{}: {} / {}", err, self.0, rhs);
            }
        }
    }

    pub fn div_by_two(self) -> Self {
        Self::from_inner(self.0 >> 1)
    }

    pub fn as_f64(&self) -> f64 {
        self.0 as f64
    }
}

impl<Unit> Default for Amount128<Unit> {
    fn default() -> Self {
        Self::ZERO
    }
}

macro_rules! impl_from {
    ($($t:ty),* $(,)?) => {$(
        impl<Unit> From<$t> for Amount128<Unit> {
            #[inline]
            fn from(value: $t) -> Self {
                Self(u128::from(value), PhantomData)
            }
        }
    )*};
}

impl_from! { u8, u16, u32, u64, u128 }

impl<Unit> TryFrom<candid::Nat> for Amount128<Unit> {
    type Error = String;

    fn try_from(value: candid::Nat) -> Result<Self, Self::Error> {
        let v128 = value
            .0
            .to_u128()
            .ok_or_else(|| format!("Nat does not fit in a u128: {}", value))?;
        Ok(Self::new(v128))
    }
}

impl<Unit> From<Amount128<Unit>> for candid::Nat {
    fn from(value: Amount128<Unit>) -> Self {
        use num_bigint::BigUint;
        candid::Nat::from(BigUint::from_bytes_be(&value.0.to_be_bytes()))
    }
}

impl<Unit> fmt::Debug for Amount128<Unit> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use thousands::Separable;
        write!(f, "{}", self.0.separate_with_underscores())
    }
}

impl<Unit> fmt::Display for Amount128<Unit> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use thousands::Separable;
        write!(f, "{}", self.0.separate_with_underscores())
    }
}

impl<Unit> fmt::LowerHex for Amount128<Unit> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:x}", self.0)
    }
}

impl<Unit> fmt::UpperHex for Amount128<Unit> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:X}", self.0)
    }
}

impl<Unit> Clone for Amount128<Unit> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<Unit> Copy for Amount128<Unit> {}

impl<Unit> PartialEq for Amount128<Unit> {
    fn eq(&self, rhs: &Self) -> bool {
        self.0.eq(&rhs.0)
    }
}

impl<Unit> Eq for Amount128<Unit> {}

impl<Unit> PartialOrd for Amount128<Unit> {
    fn partial_cmp(&self, rhs: &Self) -> Option<Ordering> {
        Some(self.cmp(rhs))
    }
}

impl<Unit> Ord for Amount128<Unit> {
    fn cmp(&self, rhs: &Self) -> Ordering {
        self.0.cmp(&rhs.0)
    }
}

// Derived serde `impl Serialize` produces an extra `unit` value for
// phantom data, e.g. `AmountOf::<Meters>::from(10)` is serialized
// into json as `[10, null]` by default.
//
// We want serialization format of `Repr` and the `AmountOf` to match
// exactly, that's why we have to provide custom instances.
impl<Unit> Serialize for Amount128<Unit> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(serializer)
    }
}

impl<'de, Unit> Deserialize<'de> for Amount128<Unit> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        u128::deserialize(deserializer).map(Self::from_inner)
    }
}

impl<C, Unit> minicbor::Encode<C> for Amount128<Unit> {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        icrc_cbor::u256::encode(&u256::new(self.0), e, ctx)
    }
}

impl<'b, C, Unit> minicbor::Decode<'b, C> for Amount128<Unit> {
    fn decode(d: &mut minicbor::Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        let v256 = icrc_cbor::u256::decode(d, ctx)?;
        let v128 = u128::try_from(v256)
            .map_err(|err| minicbor::decode::Error::message(err.to_string()))?;
        Ok(Self::new(v128))
    }
}

impl<Unit> rlp::Encodable for Amount128<Unit> {
    fn rlp_append(&self, s: &mut RlpStream) {
        let leading_empty_bytes: usize = self.0.leading_zeros() as usize / 8;
        s.append(&self.0.to_be_bytes()[leading_empty_bytes..].as_ref());
    }
}
