use crate::{
    de::{DeserializeProperty, PropertyDeserializer, error::DeserializeError},
    types::ByteStr,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PropertyName<'blob> {
    value: &'blob ByteStr,
}

impl<'blob> PropertyName<'blob> {
    #[must_use]
    pub fn new(value: &'blob ByteStr) -> Self {
        Self { value }
    }

    #[must_use]
    pub fn value(&self) -> &'blob ByteStr {
        self.value
    }
}

impl<'blob> DeserializeProperty<'blob> for PropertyName<'blob> {
    fn deserialize_property<'de, D>(de: &mut D) -> Result<Self, DeserializeError>
    where
        D: PropertyDeserializer<'de, 'blob> + ?Sized,
    {
        Ok(Self::new(de.property().name()))
    }
}
