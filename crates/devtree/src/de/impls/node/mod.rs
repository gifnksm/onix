use crate::{
    blob::Node,
    de::{DeserializeError, DeserializeNode, NodeContext},
    types::node::{NodeFullName, NodeName, NodeUnitAddress},
};

#[cfg(feature = "alloc")]
mod alloc;

impl<'blob> DeserializeNode<'blob> for Node<'blob> {
    fn deserialize_node(nctx: &mut NodeContext<'_, 'blob>) -> Result<Self, DeserializeError> {
        Ok(nctx.node().clone())
    }
}

impl<'blob, T> DeserializeNode<'blob> for Option<T>
where
    T: DeserializeNode<'blob>,
{
    fn deserialize_node(nctx: &mut NodeContext<'_, 'blob>) -> Result<Self, DeserializeError> {
        T::deserialize_node(nctx).map(Some)
    }
}

impl<'blob> DeserializeNode<'blob> for NodeFullName<'blob> {
    fn deserialize_node(nctx: &mut NodeContext<'_, 'blob>) -> Result<Self, DeserializeError> {
        Ok(Self::new(nctx.node().full_name()))
    }
}

impl<'blob> DeserializeNode<'blob> for NodeName<'blob> {
    fn deserialize_node(nctx: &mut NodeContext<'_, 'blob>) -> Result<Self, DeserializeError> {
        Ok(Self::new(nctx.node().name()))
    }
}

impl<'blob> DeserializeNode<'blob> for NodeUnitAddress<'blob> {
    fn deserialize_node(nctx: &mut NodeContext<'_, 'blob>) -> Result<Self, DeserializeError> {
        Ok(Self::new(nctx.node().unit_address()))
    }
}
