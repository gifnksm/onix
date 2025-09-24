extern crate alloc;

use alloc::{
    collections::{BTreeMap, BTreeSet},
    vec::Vec,
};

use crate::{
    de::{DeserializeNode, NodeCollection, NodeDeserializer, error::DeserializeError},
    tree_cursor::TreeCursor as _,
};

impl<'blob, T> NodeCollection<'blob> for Vec<T>
where
    T: DeserializeNode<'blob>,
{
    fn insert_node<'de, D>(&mut self, de: &mut D) -> Result<(), DeserializeError>
    where
        D: NodeDeserializer<'de, 'blob> + ?Sized,
    {
        let item = T::deserialize_node(de)?;
        self.push(item);
        Ok(())
    }
}

impl<'blob, T> NodeCollection<'blob> for BTreeSet<T>
where
    T: DeserializeNode<'blob> + Ord,
{
    fn insert_node<'de, D>(&mut self, de: &mut D) -> Result<(), DeserializeError>
    where
        D: NodeDeserializer<'de, 'blob> + ?Sized,
    {
        let item = T::deserialize_node(de)?;
        self.insert(item);
        Ok(())
    }
}

impl<'blob, K, V> NodeCollection<'blob> for BTreeMap<K, V>
where
    K: DeserializeNode<'blob> + Ord,
    V: DeserializeNode<'blob>,
{
    fn insert_node<'de, D>(&mut self, de: &mut D) -> Result<(), DeserializeError>
    where
        D: NodeDeserializer<'de, 'blob> + ?Sized,
    {
        let key = de.clone_tree_cursor()?.deserialize_node()?;
        let value = V::deserialize_node(de)?;
        self.insert(key, value);
        Ok(())
    }
}
