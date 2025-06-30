//! Endian-aware types and utilities.
//!
//! This crate provides wrapper types and traits for handling values stored in
//! big-endian or little-endian byte order, as well as conversions between
//! different byte orders. Useful for parsing binary formats and working with
//! hardware or network protocols.

#![cfg_attr(not(test), no_std)]

use core::fmt;

use dataview::Pod;

/// Trait for converting values between different byte orders.
pub trait ByteOrder {
    /// Converts a value from big-endian to native endianness.
    #[must_use]
    fn from_be(be: &Self) -> Self;

    /// Converts a value from little-endian to native endianness.
    #[must_use]
    fn from_le(le: &Self) -> Self;

    /// Converts a value from native endianness to big-endian.
    #[must_use]
    fn to_be(&self) -> Self;

    /// Converts a value from native endianness to little-endian.
    #[must_use]
    fn to_le(&self) -> Self;
}

macro_rules! impl_byte_order {
    ($($t:ty),+) => {
        $(
            impl ByteOrder for $t {
                fn from_be(be: &Self) -> Self {
                    Self::from_be(*be)
                }

                fn from_le(le: &Self) -> Self {
                    Self::from_le(*le)
                }

                fn to_be(&self) -> Self {
                    Self::to_be(*self)
                }

                fn to_le(&self) -> Self {
                    Self::to_le(*self)
                }

            }
        )+
    };
}

impl_byte_order!(u8, u16, u32, u64);
impl_byte_order!(i8, i16, i32, i64);

/// Wrapper type for values stored in big-endian byte order.
#[repr(transparent)]
#[derive(Copy, Clone, PartialEq, Eq)]
pub struct Be<T>(T);

impl<T> Be<T>
where
    T: ByteOrder,
{
    /// Reads the value, converting from big-endian to native endianness.
    pub fn read(&self) -> T {
        T::from_be(&self.0)
    }

    /// Writes a value, converting from native endianness to big-endian.
    pub fn write(&mut self, value: &T) {
        self.0 = T::to_be(value);
    }
}

/// Wrapper type for values stored in little-endian byte order.
#[repr(transparent)]
#[derive(Copy, Clone, PartialEq, Eq)]
pub struct Le<T>(T);

impl<T> Le<T>
where
    T: ByteOrder,
{
    /// Reads the value, converting from little-endian to native endianness.
    pub fn read(&self) -> T {
        T::from_le(&self.0)
    }

    /// Writes a value, converting from native endianness to little-endian.
    pub fn write(&mut self, value: &T) {
        self.0 = T::to_le(value);
    }
}

macro_rules! impl_fmt_traits {
    ($($trait:tt),+ for $ty:tt) => {
        $(
            impl<T> fmt::$trait for $ty<T>
            where
                T: ByteOrder + fmt::$trait
            {
                fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                    fmt::$trait::fmt(&self.read(), f)
                }
            }
        )+
    };
}

macro_rules! impl_common_traits {
    ($($ty:tt),+) => {
        $(
            unsafe impl<T> Pod for $ty<T> where T: Pod {}
            impl_fmt_traits!(Debug, Binary, Octal, Display, LowerHex, UpperHex for $ty);
        )+
    }
}

impl_common_traits!(Be, Le);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_byte_order_u16() {
        let n: u16 = 0x1234;
        // Check conversion from big-endian and little-endian to native
        assert_eq!(u16::from_be(0x3412_u16.to_be()), 0x3412_u16.to_be().to_be());
        assert_eq!(u16::from_le(0x3412_u16.to_le()), 0x3412_u16.to_le().to_le());
        assert_eq!(n.to_be(), n.to_be());
        assert_eq!(n.to_le(), n.to_le());
    }

    #[test]
    fn test_be_read_write() {
        let native: u32 = 0x1234_5678;
        let mut be = Be(native.to_be());
        assert_eq!(be.read(), native);
        be.write(&0xAABB_CCDD);
        assert_eq!(be.read(), 0xAABB_CCDD);
        // The internal value is always stored as BE
        assert_eq!(be.0, 0xAABB_CCDD_u32.to_be());
    }

    #[test]
    fn test_le_read_write() {
        let native: u32 = 0x1234_5678;
        let mut le = Le(native.to_le());
        assert_eq!(le.read(), native);
        le.write(&0xAABB_CCDD);
        assert_eq!(le.read(), 0xAABB_CCDD);
        // The internal value is always stored as LE
        assert_eq!(le.0, 0xAABB_CCDD_u32.to_le());
    }

    #[test]
    fn test_be_le_with_i16() {
        let n: i16 = -12345;
        let mut be = Be(n.to_be());
        let mut le = Le(n.to_le());
        assert_eq!(be.read(), n);
        assert_eq!(le.read(), n);
        be.write(&42);
        le.write(&-42);
        assert_eq!(be.read(), 42);
        assert_eq!(le.read(), -42);
    }

    #[test]
    fn test_fmt_traits() {
        let be = Be(0xABCD_u16.to_be());
        let le = Le(0x1234_u16.to_le());
        // Debug and LowerHex formatting
        assert_eq!(format!("{be:?}"), "43981");
        assert_eq!(format!("{le:x}"), "1234");
    }
}
