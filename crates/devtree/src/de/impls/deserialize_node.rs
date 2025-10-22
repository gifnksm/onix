use crate::de::{DeserializeNode, NodeDeserializer, error::DeserializeError};

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
