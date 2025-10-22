use crate::{
    de::{DeserializeNode, NodeDeserializer, error::DeserializeError},
    types::ByteStr,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeFullName<'blob>(&'blob ByteStr);

impl<'blob> NodeFullName<'blob> {
    #[must_use]
    pub fn new(value: &'blob ByteStr) -> Self {
        Self(value)
    }

    #[must_use]
    pub fn value(&self) -> &'blob ByteStr {
        self.0
    }
}

impl<'blob> DeserializeNode<'blob> for NodeFullName<'blob> {
    fn deserialize_node<'de, D>(de: &mut D) -> Result<Self, DeserializeError>
    where
        D: NodeDeserializer<'de, 'blob> + ?Sized,
    {
        Ok(Self::new(de.node().full_name()))
    }
}
