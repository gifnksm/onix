use crate::{
    blob::Node,
    de::{DeserializeNode, NodeDeserializer, error::DeserializeError},
    types::node::{NodeFullName, NodeName, NodeUnitAddress},
};

#[cfg(feature = "alloc")]
mod alloc;

impl<'blob> DeserializeNode<'blob> for Node<'blob> {
    fn deserialize_node<'de, D>(de: &mut D) -> Result<Self, DeserializeError>
    where
        D: NodeDeserializer<'de, 'blob> + ?Sized,
    {
        Ok(de.node().clone())
    }
}

impl<'blob, T> DeserializeNode<'blob> for Option<T>
where
    T: DeserializeNode<'blob>,
{
    fn deserialize_node<'de, D>(de: &mut D) -> Result<Self, DeserializeError>
    where
        D: NodeDeserializer<'de, 'blob> + ?Sized,
    {
        T::deserialize_node(de).map(Some)
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

impl<'blob> DeserializeNode<'blob> for NodeName<'blob> {
    fn deserialize_node<'de, D>(de: &mut D) -> Result<Self, DeserializeError>
    where
        D: NodeDeserializer<'de, 'blob> + ?Sized,
    {
        Ok(Self::new(de.node().name()))
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
