use platform_cast::CastFrom as _;

use crate::de::{DeserializeProperty, PropertyDeserializer, error::DeserializeError};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InterruptCells(u32);

impl InterruptCells {
    #[must_use]
    pub fn new(value: u32) -> Self {
        Self(value)
    }

    #[must_use]
    pub fn value(self) -> usize {
        usize::cast_from(self.0)
    }
}

forward_numeric_fmt_impls!(InterruptCells);

impl<'blob> DeserializeProperty<'blob> for InterruptCells {
    fn deserialize_property<'de, D>(de: &mut D) -> Result<Self, DeserializeError>
    where
        D: PropertyDeserializer<'de, 'blob> + ?Sized,
    {
        <_>::deserialize_property(de).map(Self::new)
    }
}
