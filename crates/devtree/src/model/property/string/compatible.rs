use core::fmt;

use super::ByteStrList;
use crate::{
    de::{DeserializeProperty, PropertyDeserializer, error::DeserializeError},
    types::ByteStr,
};

#[derive(Clone, Copy)]
pub struct Compatible<'blob> {
    value: ByteStrList<'blob>,
}

impl fmt::Debug for Compatible<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.value, f)
    }
}

impl<'blob> Compatible<'blob> {
    #[must_use]
    pub fn new(value: ByteStrList<'blob>) -> Self {
        Self { value }
    }

    pub fn is_compatible_to<B>(&self, model: B) -> bool
    where
        B: AsRef<ByteStr>,
    {
        let model = model.as_ref();
        self.value.iter().any(|c| c == model)
    }
}

impl<'blob> DeserializeProperty<'blob> for Compatible<'blob> {
    fn deserialize_property<'de, D>(de: &mut D) -> Result<Self, DeserializeError>
    where
        D: PropertyDeserializer<'de, 'blob> + ?Sized,
    {
        <_>::deserialize_property(de).map(Self::new)
    }
}
