use crate::de::{DeserializeNode, NodeDeserializer, error::DeserializeResult};

impl<'blob, T> DeserializeNode<'blob> for Option<T>
where
    T: DeserializeNode<'blob>,
{
    fn deserialize_node<'de, D>(de: &mut D) -> DeserializeResult<Self>
    where
        D: NodeDeserializer<'de, 'blob> + ?Sized,
    {
        T::deserialize_node(de).map(Some)
    }
}
