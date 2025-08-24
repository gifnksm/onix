use alloc::{
    string::String,
    sync::{Arc, Weak},
    vec::Vec,
};
use core::{fmt, iter::FusedIterator, ops::Range, slice};

use platform_cast::CastInto as _;
use snafu::{OptionExt as _, ResultExt as _, Snafu};
use snafu_utils::Location;

use super::{Devicetree, DevicetreeInner};
use crate::common::{
    Phandle,
    property::{self, ParsePropertyValue, ParsePropertyValueError, Property, RegIter, StringList},
};

#[derive(Clone)]
pub struct Node {
    pub(crate) inner: Arc<NodeInner>,
    pub(crate) tree: Arc<DevicetreeInner>,
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
    pub fn path(&self) -> String {
        let Some(parent) = self.parent() else {
            return String::from("/");
        };
        let mut path = parent.path();
        if !path.ends_with('/') {
            path.push('/');
        }
        path.push_str(self.name());
        if let Some(address) = self.address() {
            path.push('@');
            path.push_str(address);
        }
        path
    }

    #[must_use]
    pub fn tree(&self) -> Devicetree {
        Devicetree {
            inner: Arc::clone(&self.tree),
        }
    }

    #[must_use]
    pub fn parent(&self) -> Option<Self> {
        Weak::upgrade(&self.inner.parent).map(|inner| Self {
            inner,
            tree: Arc::clone(&self.tree),
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

#[derive(Debug, Snafu)]
pub enum PropertyError {
    #[snafu(display("missing property `{name}`", name = name))]
    #[snafu(provide(ref, priority, Location => location))]
    MissingProperty {
        name: String,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("failed to parse property `{name}`", name = name))]
    #[snafu(provide(ref, priority, Location => location))]
    ParseProperty {
        name: String,
        #[snafu(source)]
        source: ParsePropertyValueError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("missing parent node"))]
    #[snafu(provide(ref, priority, Location => location))]
    MissingParentNode {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("missing interrupt parent node"))]
    #[snafu(provide(ref, priority, Location => location))]
    MissingInterruptParentNode {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("invalid phandle `{phandle}`"))]
    #[snafu(provide(ref, priority, Location => location))]
    InvalidPhandle {
        phandle: Phandle,
        #[snafu(implicit)]
        location: Location,
    },
}

impl Node {
    #[must_use]
    pub fn find_property(&self, name: &str) -> Option<Property<'_>> {
        self.properties().find(|prop| prop.name() == name)
    }

    pub fn find_property_as<'a, T>(&'a self, name: &str) -> Result<Option<T>, PropertyError>
    where
        T: ParsePropertyValue<'a>,
    {
        self.find_property(name)
            .map(|prop| prop.parse_value::<T>())
            .transpose()
            .context(ParsePropertySnafu { name })
    }

    pub fn find_common_property_as<T>(&self, name: &str) -> Result<Option<T>, PropertyError>
    where
        T: for<'a> ParsePropertyValue<'a>,
    {
        if let Some(value) = self.find_property_as(name)? {
            return Ok(Some(value));
        }
        if let Some(parent) = self.parent()
            && let Some(value) = parent.find_property_as(name)?
        {
            return Ok(Some(value));
        }
        Ok(None)
    }

    pub fn fetch_property(&self, name: &str) -> Result<Property<'_>, PropertyError> {
        self.find_property(name)
            .context(MissingPropertySnafu { name })
    }

    pub fn fetch_property_as<'a, T>(&'a self, name: &str) -> Result<T, PropertyError>
    where
        T: ParsePropertyValue<'a>,
    {
        let prop = self.fetch_property(name)?;
        prop.parse_value().context(ParsePropertySnafu { name })
    }

    pub fn fetch_common_property_as<T>(&self, name: &str) -> Result<T, PropertyError>
    where
        T: for<'a> ParsePropertyValue<'a>,
    {
        if let Some(value) = self.find_property_as(name)? {
            return Ok(value);
        }
        if let Some(parent) = self.parent()
            && let Some(value) = parent.find_property_as(name)?
        {
            return Ok(value);
        }
        MissingPropertySnafu { name }.fail()
    }

    pub fn phandle(&self) -> Result<Phandle, PropertyError> {
        self.fetch_property_as("phandle")
    }

    #[must_use]
    pub fn is_compatible_to(&self, model: &str) -> bool {
        self.fetch_property_as::<StringList>("compatible")
            .is_ok_and(|sl| sl.iter().any(|s| s == model))
    }

    #[must_use]
    pub fn is_interrupt_controller(&self) -> bool {
        self.find_property("interrupt-controller").is_some()
    }

    pub fn interrupt_parent(&self) -> Result<Option<Phandle>, PropertyError> {
        self.find_property_as("interrupt-parent")
    }

    pub fn interrupt_parent_node(&self) -> Result<Option<Self>, PropertyError> {
        if let Some(phandle) = self.interrupt_parent()? {
            let node = self
                .tree()
                .get_node_by_phandle(phandle)
                .context(InvalidPhandleSnafu { phandle })?;
            Ok(Some(node))
        } else {
            Ok(self.parent())
        }
    }

    pub fn address_cells(&self) -> Result<usize, PropertyError> {
        Ok(self.fetch_property_as::<u32>("#address-cells")?.cast_into())
    }

    pub fn size_cells(&self) -> Result<usize, PropertyError> {
        Ok(self.fetch_property_as::<u32>("#size-cells")?.cast_into())
    }

    pub fn interrupt_cells(&self) -> Result<usize, PropertyError> {
        Ok(self
            .fetch_property_as::<u32>("#interrupt-cells")?
            .cast_into())
    }

    pub fn reg(&self) -> Result<RegIter<'_>, PropertyError> {
        let name = "reg";
        let parent = self.parent().context(MissingParentNodeSnafu)?;
        let address_cells = parent.address_cells()?;
        let size_cells = parent.size_cells()?;
        let prop = self
            .find_property(name)
            .context(MissingPropertySnafu { name })?;
        prop.parse_value_as_reg(address_cells, size_cells)
            .context(ParsePropertySnafu { name })
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
        let name = str::from_utf8(&self.node.tree.string_block[prop.name_range.clone()]).unwrap();
        Some(Property::new(name, &prop.value))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl DoubleEndedIterator for Properties<'_> {
    fn next_back(&mut self) -> Option<Self::Item> {
        let prop = self.iter.next_back()?;
        let name = str::from_utf8(&self.node.tree.string_block[prop.name_range.clone()]).unwrap();
        Some(Property::new(name, &prop.value))
    }
}

impl ExactSizeIterator for Properties<'_> {
    fn len(&self) -> usize {
        self.iter.len()
    }
}

impl FusedIterator for Properties<'_> {}

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
            tree: Arc::clone(&self.node.tree),
        })
    }
}

impl fmt::Debug for Children<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.clone()).finish()
    }
}

#[derive(Debug, Clone)]
pub struct Interrupt {
    pub interrupt_domain_root: Node,
    pub interrupt_parent: Node,
    pub specifier: Vec<u32>,
}

impl Interrupt {
    fn from_parts(
        property_name: &str,
        interrupt_parent: Node,
        source_bytes: &mut &[u8],
    ) -> Result<Self, PropertyError> {
        let mut interrupt_domain_root = interrupt_parent.clone();
        while !interrupt_domain_root.is_interrupt_controller() {
            interrupt_domain_root = interrupt_domain_root
                .interrupt_parent_node()?
                .context(MissingInterruptParentNodeSnafu)?;
        }
        let interrupt_cells = interrupt_domain_root.interrupt_cells()?;
        let specifier_bytes = property::split_first_bytes(source_bytes, interrupt_cells * 4)
            .context(ParsePropertySnafu {
                name: property_name,
            })?;
        let specifier = specifier_bytes
            .chunks_exact(4)
            .map(|chunk| u32::from_be_bytes(chunk.try_into().unwrap()))
            .collect();
        Ok(Self {
            interrupt_domain_root,
            interrupt_parent,
            specifier,
        })
    }
}

impl Node {
    pub fn interrupts(&self) -> Result<Vec<Interrupt>, PropertyError> {
        let name = "interrupts-extended";
        if let Some(prop) = self.find_property(name) {
            let mut value = prop.raw_value();

            let mut interrupts = Vec::new();
            while !value.is_empty() {
                let phandle_bytes = property::checked_split_first_chunk(&mut value)
                    .context(ParsePropertySnafu { name })?;
                let phandle = Phandle::new(u32::from_be_bytes(phandle_bytes));
                let interrupt_parent = self
                    .tree()
                    .get_node_by_phandle(phandle)
                    .context(InvalidPhandleSnafu { phandle })?;
                interrupts.push(Interrupt::from_parts(name, interrupt_parent, &mut value)?);
            }
            return Ok(interrupts);
        }

        let name = "interrupts";
        if let Some(prop) = self.find_property(name) {
            let mut value = prop.raw_value();

            let interrupt_parent = self
                .interrupt_parent_node()?
                .context(MissingInterruptParentNodeSnafu)?;

            let mut interrupts = Vec::new();
            while !value.is_empty() {
                interrupts.push(Interrupt::from_parts(
                    name,
                    interrupt_parent.clone(),
                    &mut value,
                )?);
            }
            return Ok(interrupts);
        }

        MissingPropertySnafu {
            name: "interrupts or interrupts-extended",
        }
        .fail()
    }
}
