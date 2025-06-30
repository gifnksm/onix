//! Devicetree node representation and traversal.
//!
//! This module provides functionality to represent and navigate Devicetree
//! nodes, which form the hierarchical structure of the device tree. Each node
//! can contain properties and child nodes, creating a tree-like representation
//! of the system's hardware.
//!
//! # Node Structure
//!
//! Devicetree nodes have the following characteristics:
//!
//! - **Name**: A string identifier (e.g., "cpu", "memory")
//! - **Unit Address**: Optional address part after '@' (e.g., "cpu@0")
//! - **Properties**: Key-value pairs describing the node
//! - **Children**: Nested nodes forming the hierarchy
//!
//! # Navigation
//!
//! Nodes can be traversed using various methods:
//!
//! - Properties can be iterated over
//! - Child nodes can be enumerated
//! - Sibling nodes can be accessed
//!
//! # Usage Example
//!
//! ```rust,ignore
//! let root = devicetree.root_node()?;
//! for child in root.children() {
//!     let child = child?;
//!     println!("Node: {}", child.name());
//!
//!     for prop in child.properties() {
//!         let prop = prop?;
//!         println!("  {}: {:?}", prop.name(), prop.value()?);
//!     }
//! }
//! ```

use core::{iter::FusedIterator, ops::Range};

use platform_cast::CastFrom as _;
use snafu::{IntoError as _, OptionExt as _, ResultExt as _, Snafu, ensure};
use snafu_utils::Location;

use crate::{
    property::{ParsePropertyValueError, Property},
    struct_lexer::{StructLexer, StructLexerError, StructTokenWithData},
};

#[derive(Debug, Snafu)]
pub enum ParseStructError {
    #[snafu(display("invalid struct token: {source}"))]
    Lexer {
        #[snafu(implicit)]
        source: StructLexerError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("missing begin node token"))]
    MissingBeginNodeToken {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("missing end node token"))]
    MissingEndNodeToken {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("unexpected property token"))]
    UnexpectedPropToken {
        #[snafu(implicit)]
        location: Location,
    },
}

/// A Devicetree node with properties and potential child nodes.
///
/// Nodes represent hardware components and their relationships in the system.
/// Each node has a name, optional unit address, properties describing its
/// characteristics, and may contain child nodes.
pub struct Node<'fdt, 'tree> {
    name: &'fdt str,
    address: Option<&'fdt str>,
    properties_tokens: StructLexer<'fdt, 'tree>,
    children_tokens: StructLexer<'fdt, 'tree>,
}

impl<'fdt, 'tree> Node<'fdt, 'tree> {
    pub(crate) fn new(node_tokens: &StructLexer<'fdt, 'tree>) -> Result<Self, ParseStructError> {
        let Some((StructTokenWithData::BeginNode { name, address }, properties_tokens)) =
            node_tokens.split_token().context(LexerSnafu)?
        else {
            return Err(MissingBeginNodeTokenSnafu.build());
        };

        let mut children_tokens = properties_tokens.clone();
        while let Some((StructTokenWithData::Nop | StructTokenWithData::Prop { .. }, next_tokens)) =
            children_tokens.split_token().context(LexerSnafu)?
        {
            children_tokens = next_tokens;
        }

        Ok(Self {
            name,
            address,
            properties_tokens,
            children_tokens,
        })
    }

    /// Returns the name of the node.
    ///
    /// The name is the part before the '@' character in the unit name,
    /// or the entire unit name if no '@' is present.
    ///
    /// # Examples
    ///
    /// For a node with unit name "cpu@0", this returns "cpu".
    /// For a node with unit name "memory", this returns "memory".
    #[must_use]
    pub fn name(&self) -> &'fdt str {
        self.name
    }

    /// Returns the unit address of the node, if present.
    ///
    /// The unit address is the part after the '@' character in the unit name.
    /// It typically represents the address where the device is located.
    ///
    /// # Examples
    ///
    /// For a node with unit name "cpu@0", this returns `Some("0")`.
    /// For a node with unit name "memory", this returns `None`.
    #[must_use]
    pub fn address(&self) -> Option<&'fdt str> {
        self.address
    }

    /// Returns an iterator over the properties of this node.
    ///
    /// Properties contain the actual data describing the node's
    /// characteristics, such as compatible strings, register addresses,
    /// interrupt numbers, etc.
    #[must_use]
    pub fn properties(&self) -> Properties<'fdt, 'tree> {
        Properties {
            lexer: Some(self.properties_tokens.clone()),
        }
    }

    /// Returns an iterator over the child nodes of this node.
    ///
    /// Child nodes form the hierarchical structure of the devicetree,
    /// representing sub-components or related hardware.
    #[must_use]
    pub fn children(&self) -> Children<'fdt, 'tree> {
        Children {
            lexer: Some(self.children_tokens.clone()),
        }
    }

    fn next_tokens(&self) -> Result<StructLexer<'fdt, 'tree>, ParseStructError> {
        let mut child_tokens = self.children_tokens.clone();
        loop {
            match child_tokens.split_token().context(LexerSnafu)? {
                Some((StructTokenWithData::Nop, tks)) => {
                    child_tokens = tks;
                }
                Some((StructTokenWithData::BeginNode { .. }, _)) => {
                    let child = Node::new(&child_tokens)?;
                    child_tokens = child.next_tokens()?;
                }
                Some((StructTokenWithData::EndNode, tks)) => {
                    return Ok(tks);
                }
                Some((StructTokenWithData::Prop { .. }, _)) => {
                    return Err(UnexpectedPropTokenSnafu.build());
                }
                Some((StructTokenWithData::End, _)) | None => {
                    return Err(MissingEndNodeTokenSnafu.build());
                }
            }
        }
    }

    /// Returns the first child node, if any.
    ///
    /// This is more efficient than using the children iterator when you
    /// only need the first child.
    ///
    /// # Returns
    ///
    /// * `Ok(Some(node))` - The first child node
    /// * `Ok(None)` - If this node has no children
    /// * `Err(error)` - If the structure is malformed
    pub fn first_child(&self) -> Result<Option<Self>, ParseStructError> {
        let child_tokens = &self.children_tokens;
        match child_tokens.split_token().context(LexerSnafu)? {
            Some((StructTokenWithData::Nop, _)) => {
                unreachable!("Nop token should not be present as first children tokens");
            }
            Some((StructTokenWithData::BeginNode { .. }, _)) => Ok(Some(Node::new(child_tokens)?)),
            Some((StructTokenWithData::EndNode, _)) => Ok(None),
            Some(_) | None => Err(MissingEndNodeTokenSnafu.build()),
        }
    }

    /// Returns the next sibling node, if any.
    ///
    /// This allows traversing nodes at the same level in the hierarchy
    /// without going through the parent.
    ///
    /// # Returns
    ///
    /// * `Ok(Some(node))` - The next sibling node
    /// * `Ok(None)` - If this is the last sibling
    /// * `Err(error)` - If the structure is malformed
    pub fn next_sibling(&self) -> Result<Option<Self>, ParseStructError> {
        let mut next_tokens = self.next_tokens()?;
        loop {
            match next_tokens.split_token().context(LexerSnafu)? {
                Some((StructTokenWithData::Nop, tks)) => {
                    next_tokens = tks;
                }
                Some((StructTokenWithData::BeginNode { .. }, _)) => {
                    return Ok(Some(Node::new(&next_tokens)?));
                }
                Some((StructTokenWithData::EndNode, _)) => return Ok(None),
                Some((StructTokenWithData::Prop { .. }, _)) => {
                    return Err(UnexpectedPropTokenSnafu.build());
                }
                Some((StructTokenWithData::End, _)) | None => {
                    return Ok(None);
                }
            }
        }
    }
}

/// Iterator over the properties of a Devicetree node.
///
/// Each iteration yields a property with its name and value,
/// or an error if the property data is malformed.
#[derive(Debug, Clone)]
pub struct Properties<'fdt, 'tree> {
    lexer: Option<StructLexer<'fdt, 'tree>>,
}

impl<'fdt> Iterator for Properties<'fdt, '_> {
    type Item = Result<Property<'fdt>, ParseStructError>;

    fn next(&mut self) -> Option<Self::Item> {
        let lexer = self.lexer.as_mut()?;
        let token = lexer.next()?;

        loop {
            break match token {
                Ok(StructTokenWithData::Prop(p)) => Some(Ok(p)),
                Ok(StructTokenWithData::Nop) => continue,
                Err(source) => return Some(Err(LexerSnafu.into_error(source))),
                _ => {
                    self.lexer = None;
                    None
                }
            };
        }
    }
}

impl FusedIterator for Properties<'_, '_> {}

#[derive(Debug, Snafu)]
pub enum GetPropertyError {
    #[snafu(display("invalid struct: {source}"))]
    ParseStruct {
        #[snafu(implicit)]
        source: ParseStructError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("invalid property value: {source}"))]
    ParsePropertyValue {
        #[snafu(implicit)]
        source: ParsePropertyValueError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("missing property `{name}` in parent node"))]
    MissingParentProperty {
        name: &'static str,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("invalid property `{name}` in parent node"))]
    InvalidParentProperty {
        name: &'static str,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("invalid `{name}` value length: {len}"))]
    InvalidValueLength {
        name: &'static str,
        len: usize,
        #[snafu(implicit)]
        location: Location,
    },
}

const ADDRESS_CELLS: &str = "#address-cells";
const SIZE_CELLS: &str = "#size-cells";
const REG: &str = "reg";

impl<'fdt> Properties<'fdt, '_> {
    /// Finds a property by name.
    ///
    /// This searches through all properties of the node to find one
    /// with the specified name.
    ///
    /// # Arguments
    ///
    /// * `name` - The property name to search for
    ///
    /// # Returns
    ///
    /// * `Ok(Some(property))` - If the property is found
    /// * `Ok(None)` - If the property is not found
    /// * `Err(error)` - If there's an error parsing properties
    pub fn find(&self, name: &str) -> Result<Option<Property<'fdt>>, GetPropertyError> {
        for p in self.clone() {
            let p = p.context(ParseStructSnafu)?;
            if p.name() == name {
                return Ok(Some(p));
            }
        }
        Ok(None)
    }

    /// Finds a property and parses it as a 32-bit integer.
    ///
    /// This is a convenience method for properties that should contain
    /// a single 32-bit big-endian value.
    ///
    /// # Arguments
    ///
    /// * `name` - The property name to search for
    ///
    /// # Returns
    ///
    /// * `Ok(Some(value))` - The property value as u32
    /// * `Ok(None)` - If the property is not found
    /// * `Err(error)` - If the property exists but cannot be parsed as u32
    pub fn find_u32(&self, name: &str) -> Result<Option<u32>, GetPropertyError> {
        let Some(prop) = self.find(name)? else {
            return Ok(None);
        };
        let value = prop.value_as_u32().context(ParsePropertyValueSnafu)?;
        Ok(Some(value))
    }

    /// Returns the #address-cells value for this node.
    ///
    /// This property specifies how many 32-bit cells are needed to
    /// represent addresses in child nodes.
    ///
    /// # Returns
    ///
    /// * `Ok(Some(cells))` - The number of address cells
    /// * `Ok(None)` - If the property is not present
    /// * `Err(error)` - If the property is malformed
    pub fn address_cells(&self) -> Result<Option<usize>, GetPropertyError> {
        Ok(self.find_u32(ADDRESS_CELLS)?.map(usize::cast_from))
    }

    /// Returns the #size-cells value for this node.
    ///
    /// This property specifies how many 32-bit cells are needed to
    /// represent sizes in child nodes.
    ///
    /// # Returns
    ///
    /// * `Ok(Some(cells))` - The number of size cells
    /// * `Ok(None)` - If the property is not present
    /// * `Err(error)` - If the property is malformed
    pub fn size_cells(&self) -> Result<Option<usize>, GetPropertyError> {
        Ok(self.find_u32(SIZE_CELLS)?.map(usize::cast_from))
    }

    /// Parses the "reg" property with parent context.
    ///
    /// The "reg" property contains address and size pairs that describe
    /// the memory regions used by this node. The format depends on the
    /// parent node's #address-cells and #size-cells properties.
    ///
    /// # Arguments
    ///
    /// * `parent` - The parent node providing address/size cell information
    ///
    /// # Returns
    ///
    /// * `Ok(Some(iter))` - Iterator over address/size pairs
    /// * `Ok(None)` - If the "reg" property is not present
    /// * `Err(error)` - If the property or parent context is invalid
    pub fn reg(&self, parent: &Node<'_, '_>) -> Result<Option<RegIter<'fdt>>, GetPropertyError> {
        let Some(reg) = self.find(REG)? else {
            return Ok(None);
        };

        let parent_props = parent.properties();
        let address_cells: usize =
            parent_props
                .address_cells()?
                .context(MissingParentPropertySnafu {
                    name: ADDRESS_CELLS,
                })?;
        let size_cells: usize = parent_props
            .size_cells()?
            .context(MissingParentPropertySnafu { name: SIZE_CELLS })?;

        ensure!(
            (1..=2).contains(&address_cells),
            InvalidParentPropertySnafu {
                name: ADDRESS_CELLS,
            }
        );
        ensure!(
            (0..=2).contains(&size_cells),
            InvalidParentPropertySnafu { name: SIZE_CELLS }
        );

        let unit_len = size_of::<u32>() * (address_cells + size_cells);
        let value = reg.raw_value();
        ensure!(
            value.len().is_multiple_of(unit_len),
            InvalidValueLengthSnafu {
                name: REG,
                len: value.len(),
            }
        );

        Ok(Some(RegIter {
            address_cells,
            size_cells,
            bytes: value,
        }))
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
        fn split<const N: usize>(bytes: &mut &[u8]) -> [u8; N] {
            let chunk;
            (chunk, *bytes) = bytes.split_first_chunk().unwrap();
            *chunk
        }

        if self.bytes.is_empty() {
            return None;
        }

        let address = match self.address_cells {
            1 => usize::cast_from(u32::from_be_bytes(split(&mut self.bytes))),
            2 => usize::cast_from(u64::from_be_bytes(split(&mut self.bytes))),
            _ => unreachable!("address_cells must be 1 or 2"),
        };

        let size = match self.size_cells {
            0 => 0,
            1 => usize::cast_from(u32::from_be_bytes(split(&mut self.bytes))),
            2 => usize::cast_from(u64::from_be_bytes(split(&mut self.bytes))),
            _ => unreachable!("size_cells must be 0, 1, or 2"),
        };

        Some(Reg { address, size })
    }
}

impl FusedIterator for RegIter<'_> {}

/// Iterator over child nodes of a Devicetree node.
///
/// Each iteration yields a child node or an error if the
/// structure data is malformed.
pub struct Children<'fdt, 'tree> {
    lexer: Option<StructLexer<'fdt, 'tree>>,
}

impl<'fdt, 'tree> Children<'fdt, 'tree> {
    fn try_next(&mut self) -> Result<Option<Node<'fdt, 'tree>>, ParseStructError> {
        let Some(next_tokens) = self.lexer.as_mut() else {
            return Ok(None);
        };

        loop {
            match next_tokens.split_token().context(LexerSnafu)? {
                Some((StructTokenWithData::Nop, tks)) => {
                    *next_tokens = tks;
                }
                Some((StructTokenWithData::BeginNode { .. }, _)) => {
                    let node = Node::new(next_tokens)?;
                    *next_tokens = node.next_tokens()?;
                    return Ok(Some(node));
                }
                Some((StructTokenWithData::EndNode, _)) => {
                    self.lexer = None;
                    return Ok(None);
                }
                Some(_) | None => return Err(MissingEndNodeTokenSnafu.build()),
            }
        }
    }
}

impl<'fdt, 'tree> Iterator for Children<'fdt, 'tree> {
    type Item = Result<Node<'fdt, 'tree>, ParseStructError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.try_next().transpose()
    }
}

impl FusedIterator for Children<'_, '_> {}
