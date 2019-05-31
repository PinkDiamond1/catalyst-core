use chain_core::mempack::{ReadBuf, ReadError, Readable};
use chain_core::property;
use std::ops;

/// Unspent transaction value.
#[cfg_attr(feature = "generic-serialization", derive(serde_derive::Serialize))]
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Value(pub u64);

impl Value {
    pub fn zero() -> Self {
        Value(0)
    }

    pub fn sum<I>(values: I) -> Result<Self, ValueError>
    where
        I: Iterator<Item = Self>,
    {
        values.fold(Ok(Value::zero()), |acc, v| acc? + v)
    }

    #[inline]
    pub fn checked_add(self, other: Self) -> Result<Self, ValueError> {
        self.0
            .checked_add(other.0)
            .map(Value)
            .ok_or(ValueError::Overflow)
    }

    #[inline]
    pub fn checked_sub(self, other: Value) -> Result<Value, ValueError> {
        self.0
            .checked_sub(other.0)
            .map(Value)
            .ok_or(ValueError::NegativeAmount)
    }
}

custom_error! {
    #[derive(Clone, PartialEq, Eq)]
    pub ValueError
        NegativeAmount = "Value cannot be negative",
        Overflow = "Value overflowed its maximum value",
}

impl ops::Add for Value {
    type Output = Result<Value, ValueError>;

    fn add(self, other: Value) -> Self::Output {
        self.checked_add(other)
    }
}

impl ops::Sub for Value {
    type Output = Result<Value, ValueError>;

    fn sub(self, other: Value) -> Self::Output {
        self.checked_sub(other)
    }
}

impl AsRef<u64> for Value {
    fn as_ref(&self) -> &u64 {
        &self.0
    }
}

impl property::Deserialize for Value {
    type Error = std::io::Error;

    fn deserialize<R: std::io::BufRead>(reader: R) -> Result<Self, Self::Error> {
        use chain_core::packer::*;
        let mut codec = Codec::new(reader);
        codec.get_u64().map(Value)
    }
}
impl Readable for Value {
    fn read<'a>(buf: &mut ReadBuf<'a>) -> Result<Self, ReadError> {
        buf.get_u64().map(Value)
    }
}

impl property::Serialize for Value {
    type Error = std::io::Error;
    fn serialize<W: std::io::Write>(&self, writer: W) -> Result<(), Self::Error> {
        use chain_core::packer::*;
        let mut codec = Codec::new(writer);
        codec.put_u64(self.0)
    }
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
