use core::fmt;

use crate::{
    de::{DeserializeProperty, PropertyDeserializer, error::DeserializeError},
    types::ByteStr,
};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Model<'blob> {
    value: &'blob ByteStr,
}

impl fmt::Debug for Model<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.value, f)
    }
}

impl<'blob> Model<'blob> {
    #[must_use]
    pub fn new(value: &'blob ByteStr) -> Self {
        Self { value }
    }
}

impl<'blob> DeserializeProperty<'blob> for Model<'blob> {
    fn deserialize_property<'de, D>(de: &mut D) -> Result<Self, DeserializeError>
    where
        D: PropertyDeserializer<'de, 'blob> + ?Sized,
    {
        <_>::deserialize_property(de).map(Self::new)
    }
}
