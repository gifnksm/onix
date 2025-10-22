use crate::{
    de::{DeserializeNode, NodeDeserializer, error::DeserializeError},
    types::ByteStr,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeName<'blob>(&'blob ByteStr);

impl<'blob> NodeName<'blob> {
    #[must_use]
    pub fn new(value: &'blob ByteStr) -> Self {
        Self(value)
    }

    #[must_use]
    pub fn value(&self) -> &'blob ByteStr {
        self.0
    }
}

impl<'blob> DeserializeNode<'blob> for NodeName<'blob> {
    fn deserialize_node<'de, D>(de: &mut D) -> Result<Self, DeserializeError>
    where
        D: NodeDeserializer<'de, 'blob> + ?Sized,
    {
        Ok(Self::new(de.node().name()))
    }
}
