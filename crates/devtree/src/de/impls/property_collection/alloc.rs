extern crate alloc;

use alloc::{
    collections::{BTreeMap, BTreeSet},
    vec::Vec,
};

use crate::de::{
    DeserializeProperty, PropertyCollection, PropertyDeserializer, error::DeserializeError,
};

impl<'blob, T> PropertyCollection<'blob> for Vec<T>
where
    T: DeserializeProperty<'blob>,
{
    fn insert_property<'de, D>(&mut self, de: &mut D) -> Result<(), DeserializeError>
    where
        D: PropertyDeserializer<'de, 'blob> + ?Sized,
    {
        let item = T::deserialize_property(de)?;
        self.push(item);
        Ok(())
    }
}

impl<'blob, T> PropertyCollection<'blob> for BTreeSet<T>
where
    T: DeserializeProperty<'blob> + Ord,
{
    fn insert_property<'de, D>(&mut self, de: &mut D) -> Result<(), DeserializeError>
    where
        D: PropertyDeserializer<'de, 'blob> + ?Sized,
    {
        let item = T::deserialize_property(de)?;
        self.insert(item);
        Ok(())
    }
}

impl<'blob, K, V> PropertyCollection<'blob> for BTreeMap<K, V>
where
    K: DeserializeProperty<'blob> + Ord,
    V: DeserializeProperty<'blob>,
{
    fn insert_property<'de, D>(&mut self, de: &mut D) -> Result<(), DeserializeError>
    where
        D: PropertyDeserializer<'de, 'blob> + ?Sized,
    {
        let key = K::deserialize_property(de)?;
        let value = V::deserialize_property(de)?;
        self.insert(key, value);
        Ok(())
    }
}
