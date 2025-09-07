use alloc::{
    collections::{BTreeMap, BTreeSet},
    vec::Vec,
};

use crate::{
    de::{
        DeserializeError, DeserializeNode, DeserializeProperty, NodeCollection, NodeContext,
        PropertyCollection, PropertyContext,
    },
    types::ByteStr,
};

impl<'blob, T> PropertyCollection<'blob> for Vec<T>
where
    T: DeserializeProperty<'blob>,
{
    fn insert_property(
        &mut self,
        pctx: &mut PropertyContext<'_, 'blob>,
    ) -> Result<(), DeserializeError> {
        let item = pctx.deserialize_property()?;
        self.push(item);
        Ok(())
    }
}

impl<'blob, T> PropertyCollection<'blob> for BTreeSet<T>
where
    T: DeserializeProperty<'blob> + Ord,
{
    fn insert_property(
        &mut self,
        pctx: &mut PropertyContext<'_, 'blob>,
    ) -> Result<(), DeserializeError> {
        let item = pctx.deserialize_property()?;
        self.insert(item);
        Ok(())
    }
}

impl<'blob, K, V> PropertyCollection<'blob> for BTreeMap<K, V>
where
    K: From<&'blob ByteStr> + Ord,
    V: DeserializeProperty<'blob>,
{
    fn insert_property(
        &mut self,
        pctx: &mut PropertyContext<'_, 'blob>,
    ) -> Result<(), DeserializeError> {
        let key = K::from(pctx.property().name());
        let value = pctx.deserialize_property()?;
        self.insert(key, value);
        Ok(())
    }
}

impl<'blob, T> NodeCollection<'blob> for Vec<T>
where
    T: DeserializeNode<'blob>,
{
    fn insert_node(&mut self, nctx: &mut NodeContext<'_, 'blob>) -> Result<(), DeserializeError> {
        let item = nctx.deserialize_node()?;
        self.push(item);
        Ok(())
    }
}

impl<'blob, T> NodeCollection<'blob> for BTreeSet<T>
where
    T: DeserializeNode<'blob> + Ord,
{
    fn insert_node(&mut self, nctx: &mut NodeContext<'_, 'blob>) -> Result<(), DeserializeError> {
        let item = nctx.deserialize_node()?;
        self.insert(item);
        Ok(())
    }
}

impl<'blob, K, V> NodeCollection<'blob> for BTreeMap<K, V>
where
    K: DeserializeNode<'blob> + Ord,
    V: DeserializeNode<'blob>,
{
    fn insert_node(&mut self, nctx: &mut NodeContext<'_, 'blob>) -> Result<(), DeserializeError> {
        let key = nctx.deserialize_node()?;
        let value = nctx.deserialize_node()?;
        self.insert(key, value);
        Ok(())
    }
}
