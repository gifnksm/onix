use crate::{
    de::{
        DeserializeProperty, PropertyDeserializer,
        error::{DeserializeError, DeserializePropertyError},
    },
    types::ByteStr,
};

#[derive(
    Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, derive_more::IsVariant,
)]
pub enum Status {
    #[default]
    Okay,
    Disabled,
    Reserved,
    Fail,
    FailSss,
}

impl<'blob> DeserializeProperty<'blob> for Status {
    fn deserialize_property<'de, D>(de: &mut D) -> Result<Self, DeserializeError>
    where
        D: PropertyDeserializer<'de, 'blob> + ?Sized,
    {
        let s = <&ByteStr>::deserialize_property(de)?;
        match &**s {
            b"okay" => Ok(Self::Okay),
            b"disabled" => Ok(Self::Disabled),
            b"reserved" => Ok(Self::Reserved),
            b"fail" => Ok(Self::Fail),
            b"fail-sss" => Ok(Self::FailSss),
            _ => Err(DeserializePropertyError::custom(de.property(), "invalid status").into()),
        }
    }
}
