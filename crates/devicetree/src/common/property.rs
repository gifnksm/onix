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

use core::{fmt, iter::FusedIterator, ops::Range, str::Utf8Error};

use platform_cast::CastFrom as _;
use snafu::{OptionExt as _, ResultExt as _, Snafu, ensure};
use snafu_utils::Location;

#[derive(Debug)]
pub enum ExpectedValues<T>
where
    T: 'static,
{
    Single(T),
    Multiple(&'static [T]),
}

impl<T> From<T> for ExpectedValues<T> {
    fn from(value: T) -> Self {
        Self::Single(value)
    }
}

impl<T> From<&'static [T]> for ExpectedValues<T> {
    fn from(value: &'static [T]) -> Self {
        Self::Multiple(value)
    }
}

impl<T> fmt::Display for ExpectedValues<T>
where
    T: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Single(value) => write!(f, "{value}"),
            Self::Multiple(values) => {
                let mut iter = values.iter();
                if let Some(first) = iter.next() {
                    write!(f, "{first}")?;
                }
                for value in iter {
                    write!(f, ", {value}")?;
                }
                Ok(())
            }
        }
    }
}

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
        expected: ExpectedValues<usize>,
        actual: usize,
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
                self.value_as_u32().map(PropertyValue::U32)
            }
            "compatible" => self.value_as_string_list().map(PropertyValue::StringList),
            "model" | "status" | "name" | "device_type" => {
                self.value_as_string().map(PropertyValue::String)
            }
            "phandle" | "interrupt-parent" => self.value_as_phandle().map(PropertyValue::Phandle),
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

    /// Parses the property value as a fixed-size byte array.
    ///
    /// This is useful for properties that should contain exactly N bytes.
    ///
    /// # Type Parameters
    ///
    /// * `N` - The expected number of bytes
    ///
    /// # Returns
    ///
    /// * `Ok([u8; N])` - The property value as a byte array
    /// * `Err(ParsePropertyValueError)` - If the length doesn't match
    pub fn value_as_array<const N: usize>(&self) -> Result<[u8; N], ParsePropertyValueError> {
        self.value.try_into().map_or_else(
            |_| {
                Err(InvalidValueLengthSnafu {
                    expected: N,
                    actual: self.value.len(),
                }
                .build())
            },
            Ok,
        )
    }

    /// Parses the property value as a 32-bit big-endian integer.
    ///
    /// # Returns
    ///
    /// * `Ok(u32)` - The parsed integer value
    /// * `Err(ParsePropertyValueError)` - If the value is not exactly 4 bytes
    pub fn value_as_u32(&self) -> Result<u32, ParsePropertyValueError> {
        Ok(u32::from_be_bytes(self.value_as_array()?))
    }

    /// Parses the property value as a 64-bit big-endian integer.
    ///
    /// # Returns
    ///
    /// * `Ok(u64)` - The parsed integer value
    /// * `Err(ParsePropertyValueError)` - If the value is not exactly 8 bytes
    pub fn value_as_u64(&self) -> Result<u64, ParsePropertyValueError> {
        Ok(u64::from_be_bytes(self.value_as_array()?))
    }

    pub fn value_as_u32_or_u64(&self) -> Result<u64, ParsePropertyValueError> {
        #[expect(clippy::map_err_ignore)]
        self.value_as_u32()
            .map(u64::from)
            .or_else(|_| self.value_as_u64())
            .map_err(|_| {
                InvalidValueLengthSnafu {
                    expected: [4, 8].as_slice(),
                    actual: self.value.len(),
                }
                .build()
            })
    }

    /// Parses the property value as a phandle (reference to another node).
    ///
    /// Phandles are 32-bit values that reference other nodes in the device
    /// tree.
    ///
    /// # Returns
    ///
    /// * `Ok(u32)` - The phandle value
    /// * `Err(ParsePropertyValueError)` - If the value is not exactly 4 bytes
    pub fn value_as_phandle(&self) -> Result<u32, ParsePropertyValueError> {
        Ok(u32::from_be_bytes(self.value_as_array()?))
    }

    /// Parses the property value as a null-terminated string.
    ///
    /// # Returns
    ///
    /// * `Ok(&str)` - The parsed string (without the null terminator)
    /// * `Err(ParsePropertyValueError)` - If no null terminator is found or
    ///   invalid UTF-8
    pub fn value_as_string(&self) -> Result<&'a str, ParsePropertyValueError> {
        let end = self
            .value
            .iter()
            .position(|b| *b == b'\0')
            .context(MissingNulInStringSnafu)?;
        let bytes = &self.value[..end];
        let s = str::from_utf8(bytes).context(Utf8Snafu)?;
        Ok(s)
    }

    /// Parses the property value as a list of null-terminated strings.
    ///
    /// String lists are used for properties like "compatible" that can contain
    /// multiple string values.
    ///
    /// # Returns
    ///
    /// * `Ok(StringList)` - An iterator over the individual strings
    /// * `Err(ParsePropertyValueError)` - If no null terminators found or
    ///   invalid UTF-8
    pub fn value_as_string_list(&self) -> Result<StringList<'a>, ParsePropertyValueError> {
        let end = self
            .value
            .iter()
            .rposition(|b| *b == b'\0')
            .context(MissingNulInStringSnafu)?;
        let bytes = &self.value[..=end];
        let s = str::from_utf8(bytes).context(Utf8Snafu)?;
        Ok(StringList { value: s })
    }

    pub fn value_as_reg(
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
            InvalidValueLengthSnafu {
                expected: address_cells + size_cells,
                actual: self.value.len(),
            }
        );
        Ok(RegIter {
            address_cells,
            size_cells,
            bytes: self.value,
        })
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

impl Iterator for RegIter<'_> {
    type Item = Reg;

    fn next(&mut self) -> Option<Self::Item> {
        fn split_first<const N: usize>(bytes: &mut &[u8]) -> [u8; N] {
            let chunk;
            (chunk, *bytes) = bytes.split_first_chunk().unwrap();
            *chunk
        }

        if self.bytes.is_empty() {
            return None;
        }

        let address = match self.address_cells {
            1 => usize::cast_from(u32::from_be_bytes(split_first(&mut self.bytes))),
            2 => usize::cast_from(u64::from_be_bytes(split_first(&mut self.bytes))),
            _ => unreachable!("address_cells must be 1 or 2"),
        };

        let size = match self.size_cells {
            0 => 0,
            1 => usize::cast_from(u32::from_be_bytes(split_first(&mut self.bytes))),
            2 => usize::cast_from(u64::from_be_bytes(split_first(&mut self.bytes))),
            _ => unreachable!("size_cells must be 0, 1, or 2"),
        };

        Some(Reg { address, size })
    }
}

impl DoubleEndedIterator for RegIter<'_> {
    fn next_back(&mut self) -> Option<Self::Item> {
        fn split_last<const N: usize>(bytes: &mut &[u8]) -> [u8; N] {
            let chunk;
            (*bytes, chunk) = bytes.split_last_chunk().unwrap();
            *chunk
        }

        if self.bytes.is_empty() {
            return None;
        }

        let size = match self.size_cells {
            0 => 0,
            1 => usize::cast_from(u32::from_be_bytes(split_last(&mut self.bytes))),
            2 => usize::cast_from(u64::from_be_bytes(split_last(&mut self.bytes))),
            _ => unreachable!("size_cells must be 0, 1, or 2"),
        };

        let address = match self.address_cells {
            1 => usize::cast_from(u32::from_be_bytes(split_last(&mut self.bytes))),
            2 => usize::cast_from(u64::from_be_bytes(split_last(&mut self.bytes))),
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
    Phandle(u32),
    StringList(StringList<'a>),
    Unknown(&'a [u8]),
}

impl fmt::Display for PropertyValue<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(f, "<empty>"),
            Self::U32(n) | Self::Phandle(n) => write!(f, "<{n:#04x}>"),
            Self::U64(n) => write!(f, "<{n:#04x}>"),
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
