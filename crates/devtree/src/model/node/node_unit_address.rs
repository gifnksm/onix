use crate::{
    de::{DeserializeNode, NodeDeserializer, error::DeserializeError},
    types::ByteStr,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeUnitAddress<'blob>(Option<&'blob ByteStr>);

impl<'blob> NodeUnitAddress<'blob> {
    #[must_use]
    pub fn new(value: Option<&'blob ByteStr>) -> Self {
        Self(value)
    }

    #[must_use]
    pub fn value(&self) -> Option<&'blob ByteStr> {
        self.0
    }
}

impl<'blob> DeserializeNode<'blob> for NodeUnitAddress<'blob> {
    fn deserialize_node<'de, D>(de: &mut D) -> Result<Self, DeserializeError>
    where
        D: NodeDeserializer<'de, 'blob> + ?Sized,
    {
        Ok(Self::new(de.node().unit_address()))
    }
}
