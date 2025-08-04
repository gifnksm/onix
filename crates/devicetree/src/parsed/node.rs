use alloc::{
    string::String,
    sync::{Arc, Weak},
    vec::Vec,
};
use core::{fmt, ops::Range, slice};

use crate::common::property::Property;

pub struct Node {
    pub(crate) inner: Arc<NodeInner>,
    pub(crate) string_block: Arc<[u8]>,
}

#[derive(custom_debug_derive::Debug)]
pub(crate) struct NodeInner {
    pub(crate) name: String,
    pub(crate) address: Option<String>,
    pub(crate) properties: Vec<PropertyInner>,
    #[debug(skip)]
    pub(crate) parent: Weak<Self>,
    pub(crate) children: Vec<Arc<Self>>,
}

#[derive(Debug)]
pub struct PropertyInner {
    pub(crate) name_range: Range<usize>,
    pub(crate) value: Vec<u8>,
}

impl fmt::Debug for Node {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Node")
            .field("name", &self.name())
            .field("address", &self.address())
            .field("properties", &self.properties())
            .field("children", &self.children())
            .finish_non_exhaustive()
    }
}

impl Node {
    #[must_use]
    pub fn name(&self) -> &str {
        &self.inner.name
    }

    #[must_use]
    pub fn address(&self) -> Option<&str> {
        self.inner.address.as_deref()
    }

    #[must_use]
    pub fn parent(&self) -> Option<Self> {
        Weak::upgrade(&self.inner.parent).map(|inner| Self {
            inner,
            string_block: Arc::clone(&self.string_block),
        })
    }

    #[must_use]
    pub fn properties(&self) -> Properties<'_> {
        Properties {
            iter: self.inner.properties.iter(),
            node: self,
        }
    }

    #[must_use]
    pub fn children(&self) -> Children<'_> {
        Children {
            iter: self.inner.children.iter(),
            node: self,
        }
    }
}

impl fmt::Display for Node {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fn fmt(nest: usize, node: &Node, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            let name = if nest == 0 && node.name().is_empty() {
                "/"
            } else {
                node.name()
            };
            writeln!(f)?;
            if let Some(address) = node.address() {
                writeln!(f, "{:indent$}{name}@{address} {{", "", indent = nest * 4)?;
            } else {
                writeln!(f, "{:indent$}{name} {{", "", indent = nest * 4)?;
            }
            for prop in node.properties() {
                let name = prop.name();
                let Ok(value) = prop.value() else {
                    return Err(fmt::Error);
                };
                writeln!(
                    f,
                    "{:indent$}{name} = {value};",
                    "",
                    indent = (nest + 1) * 4
                )?;
            }
            for child in node.children() {
                fmt(nest + 1, &child, f)?;
            }
            writeln!(f, "{:indent$}}};", "", indent = nest * 4)?;
            Ok(())
        }

        fmt(0, self, f)
    }
}

#[derive(Clone)]
pub struct Properties<'node> {
    iter: slice::Iter<'node, PropertyInner>,
    node: &'node Node,
}

impl fmt::Debug for Properties<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.clone()).finish()
    }
}

impl<'node> Iterator for Properties<'node> {
    type Item = Property<'node>;

    fn next(&mut self) -> Option<Self::Item> {
        let prop = self.iter.next()?;
        let name = str::from_utf8(&self.node.string_block[prop.name_range.clone()]).unwrap();
        Some(Property::new(name, &prop.value))
    }
}

#[derive(Clone)]
pub struct Children<'node> {
    iter: slice::Iter<'node, Arc<NodeInner>>,
    node: &'node Node,
}

impl Iterator for Children<'_> {
    type Item = Node;

    fn next(&mut self) -> Option<Self::Item> {
        let node = self.iter.next()?;
        Some(Node {
            inner: Arc::clone(node),
            string_block: Arc::clone(&self.node.string_block),
        })
    }
}

impl fmt::Debug for Children<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.clone()).finish()
    }
}
