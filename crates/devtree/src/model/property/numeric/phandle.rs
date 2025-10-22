use crate::de::{DeserializeProperty, PropertyDeserializer, error::DeserializeError};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Phandle(u32);

impl Phandle {
    #[must_use]
    pub fn new(value: u32) -> Self {
        Self(value)
    }

    #[must_use]
    pub fn value(self) -> u32 {
        self.0
    }
}

forward_numeric_fmt_impls!(Phandle);

impl<'blob> DeserializeProperty<'blob> for Phandle {
    fn deserialize_property<'de, D>(de: &mut D) -> Result<Self, DeserializeError>
    where
        D: PropertyDeserializer<'de, 'blob> + ?Sized,
    {
        <_>::deserialize_property(de).map(Self::new)
    }
}
