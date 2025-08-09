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

use alloc::{
    sync::{Arc, Weak},
    vec::Vec,
};
use core::iter::FusedIterator;

use snafu::{IntoError as _, ResultExt as _, Snafu};
use snafu_utils::Location;

use super::struct_lexer::{StructLexer, StructLexerError, StructTokenWithData};
use crate::{common::property::Property, parsed};

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
#[derive(Debug)]
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

    pub(crate) fn parse(
        &self,
        parent: Weak<parsed::node::NodeInner>,
        string_block: &[u8],
    ) -> Result<Arc<parsed::node::NodeInner>, ParseStructError> {
        fn init_node(
            new_node: &mut parsed::node::NodeInner,
            new_node_ref: &Weak<parsed::node::NodeInner>,
            node: &Node<'_, '_>,
            string_block: &[u8],
        ) -> Result<(), ParseStructError> {
            for prop in node.properties() {
                let prop = prop?;
                new_node.properties.push(parsed::node::PropertyInner {
                    name_range: if prop.name().is_empty() {
                        0..0
                    } else {
                        string_block.subslice_range(prop.name().as_bytes()).unwrap()
                    },
                    value: prop.raw_value().into(),
                });
            }
            for child in node.children() {
                let child = child?;
                new_node
                    .children
                    .push(child.parse(Weak::clone(new_node_ref), string_block)?);
            }
            Ok(())
        }

        let mut result = Ok(());
        let node = Arc::new_cyclic(|node_ref| {
            let mut node = parsed::node::NodeInner {
                name: self.name.into(),
                address: self.address.map(Into::into),
                properties: Vec::new(),
                parent,
                children: Vec::new(),
            };
            if let Err(e) = init_node(&mut node, node_ref, self, string_block) {
                result = Err(e);
            }
            node
        });
        result?;
        Ok(node)
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
