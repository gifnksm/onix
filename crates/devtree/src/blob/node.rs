use super::UNIT_ADDRESS_SEPARATOR;
use crate::{
    de::{DeserializeNode, NodeDeserializer, error::DeserializeError},
    polyfill,
    types::ByteStr,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node<'blob> {
    full_name: &'blob ByteStr,
}

impl<'blob> Node<'blob> {
    pub(crate) fn new<N>(full_name: &'blob N) -> Self
    where
        N: AsRef<ByteStr> + ?Sized,
    {
        let full_name = full_name.as_ref();
        Self { full_name }
    }

    #[must_use]
    pub fn full_name(&self) -> &'blob ByteStr {
        self.full_name
    }

    #[must_use]
    pub fn split_name(&self) -> (&'blob ByteStr, Option<&'blob ByteStr>) {
        match polyfill::slice_split_once(self.full_name, |&b| b == UNIT_ADDRESS_SEPARATOR) {
            Some((name, unit_address)) => (ByteStr::new(name), Some(ByteStr::new(unit_address))),
            None => (ByteStr::new(self.full_name), None),
        }
    }

    #[must_use]
    pub fn name(&self) -> &'blob ByteStr {
        self.split_name().0
    }

    #[must_use]
    pub fn unit_address(&self) -> Option<&'blob ByteStr> {
        self.split_name().1
    }

    #[must_use]
    pub fn is_root(&self) -> bool {
        self.full_name().is_empty()
    }
}

impl<'blob> DeserializeNode<'blob> for Node<'blob> {
    fn deserialize_node<'de, D>(de: &mut D) -> Result<Self, DeserializeError>
    where
        D: NodeDeserializer<'de, 'blob> + ?Sized,
    {
        Ok(de.node().clone())
    }
}

#[cfg_attr(coverage_nightly, coverage(off))]
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_full_name() {
        let node = Node::new("node@123");
        assert_eq!(node.full_name(), "node@123");
    }

    #[test]
    fn test_split_name_with_unit_address() {
        let node = Node::new("node@123");
        let (base, unit) = node.split_name();
        assert_eq!(base, ByteStr::new(b"node"));
        assert_eq!(unit, Some(ByteStr::new(b"123")));
    }

    #[test]
    fn test_split_name_without_unit_address() {
        let node = Node::new("node");
        let (base, unit) = node.split_name();
        assert_eq!(base, "node");
        assert_eq!(unit, None);
    }

    #[test]
    fn test_name_method() {
        let node = Node::new("abc@456");
        assert_eq!(node.name(), b"abc");
    }

    #[test]
    fn test_unit_address_method_some() {
        let node = Node::new("abc@456");
        assert_eq!(node.unit_address(), Some(ByteStr::new(b"456")));
    }

    #[test]
    fn test_unit_address_method_none() {
        let node = Node::new("abc");
        assert_eq!(node.unit_address(), None);
    }

    #[test]
    fn test_is_root_true() {
        let node = Node::new("");
        assert!(node.is_root());
    }

    #[test]
    fn test_is_root_false() {
        let node = Node::new("notroot");
        assert!(!node.is_root());
    }
}
