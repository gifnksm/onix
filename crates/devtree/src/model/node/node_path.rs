use crate::{
    de::{DeserializeNode, NodeDeserializer, error::DeserializeResult},
    tree_cursor::TreeCursorAllocExt as _,
    types::{ByteStr, ByteString},
};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodePath(pub ByteString);

impl NodePath {
    #[must_use]
    pub fn new(value: ByteString) -> Self {
        Self(value)
    }

    #[must_use]
    pub fn value(&self) -> &ByteStr {
        ByteStr::new(&self.0)
    }
}

impl<'blob> DeserializeNode<'blob> for NodePath {
    fn deserialize_node<'de, D>(de: &mut D) -> DeserializeResult<Self>
    where
        D: NodeDeserializer<'de, 'blob> + ?Sized,
    {
        Ok(Self::new(de.tree_cursor().path()))
    }
}
