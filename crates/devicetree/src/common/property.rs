//! Devicetree property handling and value parsing.
//!
//! This module provides functionality to parse and interpret Devicetree
//! properties according to their expected types and formats. Properties
//! in Device Trees can have various data types including strings, integers,
//! arrays, and complex encoded data.
//!
//! # Property Types
//!
//! The module handles several standard property types:
//!
//! - **Empty properties**: Properties that indicate presence without a value
//! - **Integer properties**: 32-bit and 64-bit big-endian values
//! - **String properties**: Null-terminated UTF-8 strings
//! - **String lists**: Multiple null-terminated strings concatenated
//! - **Property-encoded arrays**: Complex data structures for `reg`,
//!   `interrupts`, etc.
//! - **Phandles**: References to other nodes in the device tree
//!
//! # Usage
//!
//! ```rust,ignore
//! let property = Property::new("compatible", b"vendor,device\0");
//! match property.value()? {
//!     PropertyValue::String(s) => println!("Compatible: {}", s),
//!     PropertyValue::StringList(list) => {
//!         for item in list {
//!             println!("Compatible: {}", item);
//!         }
//!     }
//!     _ => {}
//! }
//! ```

use alloc::boxed::Box;
use core::{array, fmt, iter::FusedIterator, ops::Range, str::Utf8Error};

use either::Either;
use platform_cast::CastFrom as _;
use snafu::{OptionExt as _, ResultExt as _, Snafu, ensure};
use snafu_utils::Location;

use super::Phandle;

#[derive(Debug, Snafu)]
pub enum ParsePropertyValueError {
    #[snafu(display("missing nul in <string>"))]
    MissingNulInString {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("invalid <string>: {source}"))]
    Utf8 {
        #[snafu(implicit)]
        location: Location,
        #[snafu(source)]
        source: Utf8Error,
    },
    #[snafu(display("invalid value length. expected: {expected}, actual: {actual}"))]
    InvalidValueLength {
        expected: usize,
        actual: usize,
        #[snafu(implicit)]
        location: Location,
        #[snafu(source)]
        source: array::TryFromSliceError,
    },
    #[snafu(display("value length is not multiple of `{unit}`: length: {len}"))]
    ValueLengthIsNotMultipleOf {
        unit: usize,
        len: usize,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("value length is too small"))]
    ValueLengthIsTooSmall {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("value length is too large"))]
    ValueLengthIsTooLarge {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("invalid `#address-cells`: {address_cells}"))]
    InvalidAddressCells {
        address_cells: usize,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("invalid `#size-cells`: {size_cells}"))]
    InvalidSizeCells {
        size_cells: usize,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("value cannot be parsed: {left}, {right}"))]
    Either {
        left: Box<ParsePropertyValueError>,
        right: Box<ParsePropertyValueError>,
        #[snafu(implicit)]
        location: Location,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Property<'a> {
    name: &'a str,
    value: &'a [u8],
}

impl<'a> Property<'a> {
    #[must_use]
    pub fn new(name: &'a str, value: &'a [u8]) -> Self {
        Self { name, value }
    }

    #[must_use]
    pub fn name(&self) -> &'a str {
        self.name
    }

    #[must_use]
    pub fn raw_value(&self) -> &'a [u8] {
        self.value
    }

    /// Parses the property value into a strongly-typed format.
    ///
    /// This method interprets the raw property value according to the property
    /// name and Devicetree conventions. Common properties like "compatible",
    /// "reg", "interrupts", etc. are parsed into appropriate types.
    ///
    /// # Returns
    ///
    /// * `Ok(PropertyValue)` - The parsed property value
    /// * `Err(ParsePropertyValueError)` - If the value format is invalid
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let prop = Property::new("compatible", b"vendor,device\0");
    /// match prop.value()? {
    ///     PropertyValue::String(s) => println!("Device: {}", s),
    ///     _ => {}
    /// }
    /// ```
    pub fn value(&self) -> Result<PropertyValue<'a>, ParsePropertyValueError> {
        match self.name {
            "interrupt-controller" => Ok(PropertyValue::Empty),
            "#address-cells" | "#size-cells" | "virtual-reg" | "#interrupt-cells" => {
                self.parse_value().map(PropertyValue::U32)
            }
            "compatible" => self.parse_value().map(PropertyValue::StringList),
            "model" | "status" | "name" | "device_type" => {
                self.parse_value().map(PropertyValue::String)
            }
            "phandle" | "interrupt-parent" => self.parse_value().map(PropertyValue::Phandle),
            "reg"
            | "interrupts"
            | "interrupts-extended"
            | "interrupt-map"
            | "interrupt-map-mask" => Ok(PropertyValue::PropEncodedArray(self.value)),
            "ranges" | "dma-ranges" => {
                let v = if self.value.is_empty() {
                    PropertyValue::Empty
                } else {
                    PropertyValue::PropEncodedArray(self.value)
                };
                Ok(v)
            }
            _ => Ok(PropertyValue::Unknown(self.value)),
        }
    }

    /// Parses the property value as a specified type `T`.
    pub fn parse_value<T>(&self) -> Result<T, ParsePropertyValueError>
    where
        T: ParsePropertyValue<'a>,
    {
        T::parse(self)
    }

    pub fn parse_value_as_reg(
        &self,
        address_cells: usize,
        size_cells: usize,
    ) -> Result<RegIter<'a>, ParsePropertyValueError> {
        ensure!(
            (1..=2).contains(&address_cells),
            InvalidAddressCellsSnafu { address_cells }
        );
        ensure!(
            (0..=2).contains(&size_cells),
            InvalidSizeCellsSnafu { size_cells }
        );
        ensure!(
            self.value
                .len()
                .is_multiple_of((address_cells + size_cells) * size_of::<u32>()),
            ValueLengthIsNotMultipleOfSnafu {
                unit: address_cells + size_cells,
                len: self.value.len(),
            }
        );
        Ok(RegIter {
            address_cells,
            size_cells,
            bytes: self.value,
        })
    }
}

pub trait ParsePropertyValue<'a>: Sized {
    fn parse(prop: &Property<'a>) -> Result<Self, ParsePropertyValueError>;
}

impl<const N: usize> ParsePropertyValue<'_> for [u8; N] {
    fn parse(prop: &Property<'_>) -> Result<Self, ParsePropertyValueError> {
        prop.value.try_into().context(InvalidValueLengthSnafu {
            expected: N,
            actual: prop.value.len(),
        })
    }
}

impl ParsePropertyValue<'_> for u32 {
    fn parse(prop: &Property<'_>) -> Result<Self, ParsePropertyValueError> {
        Ok(Self::from_be_bytes(prop.parse_value()?))
    }
}

impl ParsePropertyValue<'_> for u64 {
    fn parse(prop: &Property<'_>) -> Result<Self, ParsePropertyValueError> {
        Ok(Self::from_be_bytes(prop.parse_value()?))
    }
}

impl ParsePropertyValue<'_> for Phandle {
    fn parse(prop: &Property<'_>) -> Result<Self, ParsePropertyValueError> {
        Ok(Self(prop.parse_value()?))
    }
}

impl<'a> ParsePropertyValue<'a> for &'a str {
    fn parse(prop: &Property<'a>) -> Result<Self, ParsePropertyValueError> {
        let end = prop
            .value
            .iter()
            .position(|b| *b == b'\0')
            .context(MissingNulInStringSnafu)?;
        let bytes = &prop.value[..end];
        let s = str::from_utf8(bytes).context(Utf8Snafu)?;
        Ok(s)
    }
}

impl<'a> ParsePropertyValue<'a> for StringList<'a> {
    fn parse(prop: &Property<'a>) -> Result<Self, ParsePropertyValueError> {
        let end = prop
            .value
            .iter()
            .rposition(|b| *b == b'\0')
            .context(MissingNulInStringSnafu)?;
        let bytes = &prop.value[..=end];
        let s = str::from_utf8(bytes).context(Utf8Snafu)?;
        Ok(StringList { value: s })
    }
}

impl<'a, L, R> ParsePropertyValue<'a> for Either<L, R>
where
    L: ParsePropertyValue<'a>,
    R: ParsePropertyValue<'a>,
{
    fn parse(prop: &Property<'a>) -> Result<Self, ParsePropertyValueError> {
        let left = match L::parse(prop) {
            Ok(value) => return Ok(Self::Left(value)),
            Err(e) => e,
        };
        let right = match R::parse(prop) {
            Ok(value) => return Ok(Self::Right(value)),
            Err(e) => e,
        };
        Err(EitherSnafu {
            left: Box::new(left),
            right: Box::new(right),
        }
        .build())
    }
}

/// A register entry from a "reg" property.
///
/// Contains an address and size pair describing a memory region
/// used by a device.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Reg {
    /// The starting address of the memory region
    pub address: usize,
    /// The size of the memory region in bytes
    pub size: usize,
}

impl Reg {
    /// Returns the memory range as a Rust Range.
    ///
    /// # Returns
    ///
    /// A range from `address` to `address + size`, capped to prevent overflow.
    #[must_use]
    pub fn range(&self) -> Range<usize> {
        self.address..self.address.saturating_add(self.size)
    }
}

pub(crate) fn checked_split_first_chunk<const N: usize>(
    bytes: &mut &[u8],
) -> Result<[u8; N], ParsePropertyValueError> {
    let chunk;
    (chunk, *bytes) = bytes
        .split_first_chunk()
        .context(ValueLengthIsTooSmallSnafu)?;
    Ok(*chunk)
}

pub(crate) fn split_first_bytes<'a>(
    bytes: &mut &'a [u8],
    len: usize,
) -> Result<&'a [u8], ParsePropertyValueError> {
    ensure!(bytes.len() >= len, ValueLengthIsTooSmallSnafu);
    let (first, rest) = bytes.split_at(len);
    *bytes = rest;
    Ok(first)
}

fn split_first_chunk<const N: usize>(bytes: &mut &[u8]) -> [u8; N] {
    let chunk;
    (chunk, *bytes) = bytes.split_first_chunk().unwrap();
    *chunk
}

fn split_last_chunk<const N: usize>(bytes: &mut &[u8]) -> [u8; N] {
    let chunk;
    (*bytes, chunk) = bytes.split_last_chunk().unwrap();
    *chunk
}

/// Iterator over register entries in a "reg" property.
///
/// Parses the binary data in a "reg" property according to the
/// parent node's #address-cells and #size-cells values.
#[derive(Debug, Clone)]
pub struct RegIter<'fdt> {
    address_cells: usize,
    size_cells: usize,
    bytes: &'fdt [u8],
}

impl RegIter<'_> {
    #[must_use]
    pub fn assume_one(&self) -> Option<Reg> {
        let mut this = self.clone();
        let reg = this.next()?;
        if this.next().is_some() {
            return None;
        }
        Some(reg)
    }
}

impl Iterator for RegIter<'_> {
    type Item = Reg;

    fn next(&mut self) -> Option<Self::Item> {
        if self.bytes.is_empty() {
            return None;
        }

        let address = match self.address_cells {
            1 => usize::cast_from(u32::from_be_bytes(split_first_chunk(&mut self.bytes))),
            2 => usize::cast_from(u64::from_be_bytes(split_first_chunk(&mut self.bytes))),
            _ => unreachable!("address_cells must be 1 or 2"),
        };

        let size = match self.size_cells {
            0 => 0,
            1 => usize::cast_from(u32::from_be_bytes(split_first_chunk(&mut self.bytes))),
            2 => usize::cast_from(u64::from_be_bytes(split_first_chunk(&mut self.bytes))),
            _ => unreachable!("size_cells must be 0, 1, or 2"),
        };

        Some(Reg { address, size })
    }
}

impl DoubleEndedIterator for RegIter<'_> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.bytes.is_empty() {
            return None;
        }

        let size = match self.size_cells {
            0 => 0,
            1 => usize::cast_from(u32::from_be_bytes(split_last_chunk(&mut self.bytes))),
            2 => usize::cast_from(u64::from_be_bytes(split_last_chunk(&mut self.bytes))),
            _ => unreachable!("size_cells must be 0, 1, or 2"),
        };

        let address = match self.address_cells {
            1 => usize::cast_from(u32::from_be_bytes(split_last_chunk(&mut self.bytes))),
            2 => usize::cast_from(u64::from_be_bytes(split_last_chunk(&mut self.bytes))),
            _ => unreachable!("address_cells must be 1 or 2"),
        };

        Some(Reg { address, size })
    }
}

impl ExactSizeIterator for RegIter<'_> {
    fn len(&self) -> usize {
        self.bytes.len() / (self.address_cells + self.size_cells) / size_of::<u32>()
    }
}

impl FusedIterator for RegIter<'_> {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropertyValue<'a> {
    Empty,
    U32(u32),
    U64(u64),
    String(&'a str),
    PropEncodedArray(&'a [u8]),
    Phandle(Phandle),
    StringList(StringList<'a>),
    Unknown(&'a [u8]),
}

impl fmt::Display for PropertyValue<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(f, "<empty>"),
            Self::U32(n) => write!(f, "<{n:#04x}>"),
            Self::U64(n) => write!(f, "<{n:#04x}>"),
            Self::Phandle(n) => write!(f, "<{n:#04x}>"),
            Self::String(s) => write!(f, "{s:?}"),
            Self::StringList(s) => write!(f, "{s}"),
            Self::PropEncodedArray(items) | Self::Unknown(items) => {
                fmt::Display::fmt(&Bytes(items), f)
            }
        }
    }
}

struct Bytes<'a>(&'a [u8]);
impl fmt::Display for Bytes<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut iter = self.0.iter();
        write!(f, "<")?;
        if let Some(n) = iter.next() {
            write!(f, "{n:#04x}")?;
        }
        for n in iter {
            write!(f, " {n:#04x}")?;
        }
        write!(f, ">")?;
        Ok(())
    }
}

/// A list of null-terminated strings from a Device Tree property.
///
/// This type provides iteration over individual strings in properties
/// that contain multiple string values, such as "compatible" properties.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StringList<'a> {
    value: &'a str,
}

impl<'a> StringList<'a> {
    /// Returns an iterator over the individual strings in the list.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// for string in string_list.iter() {
    ///     println!("String: {}", string);
    /// }
    /// ```
    #[must_use]
    pub fn iter(&self) -> StringListIter<'a> {
        StringListIter { value: self.value }
    }
}

impl<'a> IntoIterator for StringList<'a> {
    type Item = &'a str;

    type IntoIter = StringListIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a> IntoIterator for &StringList<'a> {
    type Item = &'a str;

    type IntoIter = StringListIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl fmt::Display for StringList<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut iter = self.iter();
        if let Some(s) = iter.next() {
            write!(f, "{s:?}")?;
        }
        for s in iter {
            write!(f, ", {s:?}")?;
        }
        Ok(())
    }
}

/// Iterator over individual strings in a [`StringList`].
///
/// Each call to `next()` returns the next null-terminated string
/// from the concatenated string data.
#[derive(Debug, Clone)]
pub struct StringListIter<'a> {
    value: &'a str,
}

impl<'a> Iterator for StringListIter<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        if self.value.is_empty() {
            return None;
        }

        let end = self.value.find('\0').unwrap();
        let s = &self.value[..end];
        self.value = &self.value[end + 1..];
        Some(s)
    }
}

impl FusedIterator for StringListIter<'_> {}
