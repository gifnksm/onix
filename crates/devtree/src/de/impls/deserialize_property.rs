use crate::{
    blob::Property,
    de::{
        DeserializeProperty, PropertyDeserializer,
        error::{DeserializeError, DeserializePropertyError},
    },
    polyfill,
    types::ByteStr,
};

fn deserialize_byte_str_until_first_nul<'blob>(
    property: &Property<'blob>,
) -> Result<&'blob [u8], DeserializeError> {
    let (bytes, _) = polyfill::slice_split_once(property.value(), |&c| c == 0)
        .ok_or_else(|| DeserializePropertyError::missing_nul_in_string_value(property))?;
    Ok(bytes)
}

impl<'blob, T> DeserializeProperty<'blob> for Option<T>
where
    T: DeserializeProperty<'blob>,
{
    fn deserialize_property<'de, D>(de: &mut D) -> Result<Self, DeserializeError>
    where
        D: PropertyDeserializer<'de, 'blob> + ?Sized,
    {
        T::deserialize_property(de).map(Some)
    }
}

impl<'blob, const N: usize> DeserializeProperty<'blob> for [u8; N] {
    fn deserialize_property<'de, D>(de: &mut D) -> Result<Self, DeserializeError>
    where
        D: PropertyDeserializer<'de, 'blob> + ?Sized,
    {
        let property = de.property();
        let value = property.value();
        let Ok(value) = value.try_into() else {
            bail!(DeserializePropertyError::value_length_mismatch(property, N));
        };
        Ok(value)
    }
}

impl<'blob> DeserializeProperty<'blob> for &'blob [u8] {
    fn deserialize_property<'de, D>(de: &mut D) -> Result<Self, DeserializeError>
    where
        D: PropertyDeserializer<'de, 'blob> + ?Sized,
    {
        Ok(de.property().value())
    }
}

impl<'blob, const N: usize> DeserializeProperty<'blob> for &'blob [u8; N] {
    fn deserialize_property<'de, D>(de: &mut D) -> Result<Self, DeserializeError>
    where
        D: PropertyDeserializer<'de, 'blob> + ?Sized,
    {
        let property = de.property();
        let value = property.value();
        let Ok(value) = value.try_into() else {
            bail!(DeserializePropertyError::value_length_mismatch(property, N));
        };
        Ok(value)
    }
}

impl<'blob, const N: usize> DeserializeProperty<'blob> for &'blob [[u8; N]] {
    fn deserialize_property<'de, D>(de: &mut D) -> Result<Self, DeserializeError>
    where
        D: PropertyDeserializer<'de, 'blob> + ?Sized,
    {
        let property = de.property();
        let value = property.value();
        let (value, rest) = value.as_chunks();
        ensure!(
            rest.is_empty(),
            DeserializePropertyError::value_length_is_not_multiple_of(property, N)
        );
        Ok(value)
    }
}

impl<'blob> DeserializeProperty<'blob> for () {
    fn deserialize_property<'de, D>(de: &mut D) -> Result<Self, DeserializeError>
    where
        D: PropertyDeserializer<'de, 'blob> + ?Sized,
    {
        let [] = <[u8; 0]>::deserialize_property(de)?;
        Ok(())
    }
}

impl<'blob> DeserializeProperty<'blob> for bool {
    fn deserialize_property<'de, D>(de: &mut D) -> Result<Self, DeserializeError>
    where
        D: PropertyDeserializer<'de, 'blob> + ?Sized,
    {
        <()>::deserialize_property(de)?;
        Ok(true)
    }
}

impl<'blob> DeserializeProperty<'blob> for u32 {
    fn deserialize_property<'de, D>(de: &mut D) -> Result<Self, DeserializeError>
    where
        D: PropertyDeserializer<'de, 'blob> + ?Sized,
    {
        <_>::deserialize_property(de).map(Self::from_be_bytes)
    }
}

impl<'blob> DeserializeProperty<'blob> for u64 {
    fn deserialize_property<'de, D>(de: &mut D) -> Result<Self, DeserializeError>
    where
        D: PropertyDeserializer<'de, 'blob> + ?Sized,
    {
        <_>::deserialize_property(de).map(Self::from_be_bytes)
    }
}

impl<'blob> DeserializeProperty<'blob> for &'blob str {
    fn deserialize_property<'de, D>(de: &mut D) -> Result<Self, DeserializeError>
    where
        D: PropertyDeserializer<'de, 'blob> + ?Sized,
    {
        let property = de.property();
        let bytes = deserialize_byte_str_until_first_nul(property)?;
        str::from_utf8(bytes).map_err(|source| {
            DeserializePropertyError::invalid_string_value(property, source).into()
        })
    }
}

impl<'blob> DeserializeProperty<'blob> for &'blob ByteStr {
    fn deserialize_property<'de, D>(de: &mut D) -> Result<Self, DeserializeError>
    where
        D: PropertyDeserializer<'de, 'blob> + ?Sized,
    {
        let property = de.property();
        let bytes = deserialize_byte_str_until_first_nul(property)?;
        Ok(ByteStr::new(bytes))
    }
}
