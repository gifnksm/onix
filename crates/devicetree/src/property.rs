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

use core::{fmt, iter::FusedIterator, str::Utf8Error};

use snafu::{OptionExt as _, ResultExt as _, Snafu};
use snafu_utils::Location;

#[derive(Debug, Snafu)]
pub enum ParsePropertyValueError {
    #[snafu(display("missing nul in <string>"))]
    MissingNulInString {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("invalid <string>: {source}"))]
    Utf8 {
        #[snafu(source)]
        source: Utf8Error,
    },
    #[snafu(display("invalid value length. expected: {expected}, actual: {actual}"))]
    InvalidValueLength { expected: usize, actual: usize },
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
}

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
