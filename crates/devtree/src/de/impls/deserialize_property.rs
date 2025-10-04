use devtree_derive::DeserializeNode;

use crate::{
    blob::Property,
    de::{
        DeserializeProperty, PropertyDeserializer,
        error::{DeserializeError, DeserializeNodeError, DeserializePropertyError},
    },
    polyfill,
    tree_cursor::TreeCursor as _,
    types::{
        ByteStr,
        property::{
            AddressCells, ByteStrList, Compatible, InterruptCells, Model, Phandle, PropertyName,
            Ranges, Reg, SizeCells, Status, StrList, U32Array,
        },
    },
};

fn deserialize_byte_str_until_first_nul<'blob>(
    property: &Property<'blob>,
) -> Result<&'blob [u8], DeserializeError> {
    let (bytes, _) = polyfill::slice_split_once(property.value(), |&c| c == 0)
        .ok_or_else(|| DeserializePropertyError::missing_nul_in_string_value(property))?;
    Ok(bytes)
}

fn deserialize_byte_str_until_last_nul<'blob>(
    property: &Property<'blob>,
) -> Result<&'blob [u8], DeserializeError> {
    let (bytes, _) = polyfill::slice_rsplit_once(property.value(), |&c| c == 0)
        .ok_or_else(|| DeserializePropertyError::missing_nul_in_string_value(property))?;
    Ok(bytes)
}

impl<'blob> DeserializeProperty<'blob> for PropertyName<'blob> {
    fn deserialize_property<'de, D>(de: &mut D) -> Result<Self, DeserializeError>
    where
        D: PropertyDeserializer<'de, 'blob> + ?Sized,
    {
        Ok(Self::new(de.property().name()))
    }
}

impl<'blob> DeserializeProperty<'blob> for Property<'blob> {
    fn deserialize_property<'de, D>(de: &mut D) -> Result<Self, DeserializeError>
    where
        D: PropertyDeserializer<'de, 'blob> + ?Sized,
    {
        Ok(de.property().clone())
    }
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

impl<'blob> DeserializeProperty<'blob> for &'blob U32Array {
    fn deserialize_property<'de, D>(de: &mut D) -> Result<Self, DeserializeError>
    where
        D: PropertyDeserializer<'de, 'blob> + ?Sized,
    {
        <_>::deserialize_property(de).map(U32Array::new)
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

impl<'blob> DeserializeProperty<'blob> for StrList<'blob> {
    fn deserialize_property<'de, D>(de: &mut D) -> Result<Self, DeserializeError>
    where
        D: PropertyDeserializer<'de, 'blob> + ?Sized,
    {
        let property = de.property();
        let bytes = deserialize_byte_str_until_last_nul(property)?;
        let s = str::from_utf8(bytes)
            .map_err(|source| DeserializePropertyError::invalid_string_value(property, source))?;
        Ok(Self::new(s))
    }
}

impl<'blob> DeserializeProperty<'blob> for ByteStrList<'blob> {
    fn deserialize_property<'de, D>(de: &mut D) -> Result<Self, DeserializeError>
    where
        D: PropertyDeserializer<'de, 'blob> + ?Sized,
    {
        let property = de.property();
        let bytes = deserialize_byte_str_until_last_nul(property)?;
        Ok(Self::new(ByteStr::new(bytes)))
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

impl<'blob> DeserializeProperty<'blob> for Model<'blob> {
    fn deserialize_property<'de, D>(de: &mut D) -> Result<Self, DeserializeError>
    where
        D: PropertyDeserializer<'de, 'blob> + ?Sized,
    {
        <_>::deserialize_property(de).map(Self::new)
    }
}

impl<'blob> DeserializeProperty<'blob> for Phandle {
    fn deserialize_property<'de, D>(de: &mut D) -> Result<Self, DeserializeError>
    where
        D: PropertyDeserializer<'de, 'blob> + ?Sized,
    {
        <_>::deserialize_property(de).map(Self::new)
    }
}

impl<'blob> DeserializeProperty<'blob> for AddressCells {
    fn deserialize_property<'de, D>(de: &mut D) -> Result<Self, DeserializeError>
    where
        D: PropertyDeserializer<'de, 'blob> + ?Sized,
    {
        <_>::deserialize_property(de).map(Self::new)
    }
}

impl<'blob> DeserializeProperty<'blob> for SizeCells {
    fn deserialize_property<'de, D>(de: &mut D) -> Result<Self, DeserializeError>
    where
        D: PropertyDeserializer<'de, 'blob> + ?Sized,
    {
        <_>::deserialize_property(de).map(Self::new)
    }
}

impl<'blob> DeserializeProperty<'blob> for InterruptCells {
    fn deserialize_property<'de, D>(de: &mut D) -> Result<Self, DeserializeError>
    where
        D: PropertyDeserializer<'de, 'blob> + ?Sized,
    {
        <_>::deserialize_property(de).map(Self::new)
    }
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

#[derive(DeserializeNode)]
#[devtree(crate = crate)]
struct RegParent {
    #[devtree(property(name = "#address-cells"))]
    address_cells: AddressCells,
    #[devtree(property(name = "#size-cells"))]
    size_cells: SizeCells,
}

impl<'blob> DeserializeProperty<'blob> for Reg<'blob> {
    fn deserialize_property<'de, D>(de: &mut D) -> Result<Self, DeserializeError>
    where
        D: PropertyDeserializer<'de, 'blob> + ?Sized,
    {
        let value = <&[[u8; 4]]>::deserialize_property(de)?;
        let RegParent {
            address_cells,
            size_cells,
        } = de
            .clone_tree_cursor()?
            .read_parent()
            .ok_or_else(|| DeserializeNodeError::missing_parent_node(de.node()))?
            .deserialize_node()?;

        let unit = address_cells.value() + size_cells.value();
        ensure!(
            value.len().is_multiple_of(unit),
            DeserializePropertyError::value_length_is_not_multiple_of(de.property(), unit)
        );
        Ok(Self::new(address_cells, size_cells, value))
    }
}

#[derive(DeserializeNode)]
#[devtree(crate = crate)]
struct RangesParent {
    #[devtree(property(name = "#address-cells"))]
    parent_address_cells: AddressCells,
}

#[derive(DeserializeNode)]
#[devtree(crate = crate)]
struct RangesNode {
    #[devtree(property(name = "#address-cells"))]
    child_address_cells: AddressCells,
    #[devtree(property(name = "#size-cells"))]
    child_size_cells: SizeCells,
}

impl<'blob> DeserializeProperty<'blob> for Ranges<'blob> {
    fn deserialize_property<'de, D>(de: &mut D) -> Result<Self, DeserializeError>
    where
        D: PropertyDeserializer<'de, 'blob> + ?Sized,
    {
        let RangesParent {
            parent_address_cells,
        } = de
            .clone_tree_cursor()?
            .read_parent()
            .ok_or_else(|| DeserializeNodeError::missing_parent_node(de.node()))?
            .deserialize_node()?;
        let RangesNode {
            child_address_cells,
            child_size_cells,
        } = de.clone_tree_cursor()?.read_node().deserialize_node()?;

        let value = <&[[u8; 4]]>::deserialize_property(de)?;

        let unit =
            parent_address_cells.value() + child_address_cells.value() + child_size_cells.value();
        ensure!(
            value.len().is_multiple_of(unit),
            DeserializePropertyError::value_length_is_not_multiple_of(de.property(), unit,)
        );
        Ok(Self::new(
            child_address_cells,
            child_size_cells,
            parent_address_cells,
            value,
        ))
    }
}
