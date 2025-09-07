use crate::{
    blob::Property,
    de::{DeserializeError, DeserializeProperty, PropertyContext},
    types::{
        ByteStr,
        node::AddressSpace,
        property::{
            AddressCells, ByteStrList, Compatible, InterruptCells, Model, Phandle, Ranges, Reg,
            SizeCells, Status, StrList, U32Array,
        },
    },
    utils,
};

fn deserialize_byte_str_until_first_nul<'blob>(
    pctx: &PropertyContext<'_, 'blob>,
) -> Result<&'blob [u8], DeserializeError> {
    let (bytes, _) = utils::slice_split_once(pctx.property().value(), |&c| c == 0)
        .ok_or_else(|| pctx.error_missing_nul_in_string())?;
    Ok(bytes)
}

fn deserialize_byte_str_until_last_nul<'blob>(
    pctx: &PropertyContext<'_, 'blob>,
) -> Result<&'blob [u8], DeserializeError> {
    let (bytes, _) = utils::slice_rsplit_once(pctx.property().value(), |&c| c == 0)
        .ok_or_else(|| pctx.error_missing_nul_in_string())?;
    Ok(bytes)
}

impl<'blob> DeserializeProperty<'blob> for Property<'blob> {
    fn deserialize_property(
        pctx: &mut PropertyContext<'_, 'blob>,
    ) -> Result<Self, DeserializeError> {
        Ok(pctx.property().clone())
    }
}

impl<'blob, T> DeserializeProperty<'blob> for Option<T>
where
    T: DeserializeProperty<'blob>,
{
    fn deserialize_property(
        pctx: &mut PropertyContext<'_, 'blob>,
    ) -> Result<Self, DeserializeError> {
        T::deserialize_property(pctx).map(Some)
    }
}

impl<'blob> DeserializeProperty<'blob> for &'blob [u8] {
    fn deserialize_property(
        pctx: &mut PropertyContext<'_, 'blob>,
    ) -> Result<Self, DeserializeError> {
        Ok(pctx.property().value())
    }
}

impl<'blob, const N: usize> DeserializeProperty<'blob> for [u8; N] {
    fn deserialize_property(
        pctx: &mut PropertyContext<'_, 'blob>,
    ) -> Result<Self, DeserializeError> {
        let value = pctx.property().value();
        let Ok(bytes) = value.try_into() else {
            return Err(pctx.error_invalid_value_length(N));
        };
        Ok(bytes)
    }
}

impl<'blob, const N: usize> DeserializeProperty<'blob> for &'blob [[u8; N]] {
    fn deserialize_property(
        pctx: &mut PropertyContext<'_, 'blob>,
    ) -> Result<Self, DeserializeError> {
        let value = pctx.property().value();
        let (bytes, rest) = value.as_chunks();
        if !rest.is_empty() {
            return Err(pctx.error_invalid_value_length(value.len().next_multiple_of(N)));
        }
        Ok(bytes)
    }
}

impl<'blob> DeserializeProperty<'blob> for &'blob U32Array {
    fn deserialize_property(
        pctx: &mut PropertyContext<'_, 'blob>,
    ) -> Result<Self, DeserializeError> {
        Ok(U32Array::new(pctx.deserialize_property()?))
    }
}

impl<'blob> DeserializeProperty<'blob> for () {
    fn deserialize_property(
        pctx: &mut PropertyContext<'_, 'blob>,
    ) -> Result<Self, DeserializeError> {
        let value = pctx.property().value();
        if !value.is_empty() {
            return Err(pctx.error_invalid_value_length(0));
        }
        Ok(())
    }
}

impl<'blob> DeserializeProperty<'blob> for bool {
    fn deserialize_property(
        pctx: &mut PropertyContext<'_, 'blob>,
    ) -> Result<Self, DeserializeError> {
        let value = pctx.property().value();
        if !value.is_empty() {
            return Err(pctx.error_invalid_value_length(0));
        }
        Ok(true)
    }
}

impl<'blob> DeserializeProperty<'blob> for u32 {
    fn deserialize_property(
        pctx: &mut PropertyContext<'_, 'blob>,
    ) -> Result<Self, DeserializeError> {
        Ok(Self::from_be_bytes(pctx.deserialize_property()?))
    }
}

impl<'blob> DeserializeProperty<'blob> for u64 {
    fn deserialize_property(
        pctx: &mut PropertyContext<'_, 'blob>,
    ) -> Result<Self, DeserializeError> {
        pctx.deserialize_property().map(Self::from_be_bytes)
    }
}

impl<'blob> DeserializeProperty<'blob> for &'blob str {
    fn deserialize_property(
        pctx: &mut PropertyContext<'_, 'blob>,
    ) -> Result<Self, DeserializeError> {
        let bytes = deserialize_byte_str_until_first_nul(pctx)?;
        str::from_utf8(bytes).map_err(|source| pctx.error_invalid_string_value(source))
    }
}

impl<'blob> DeserializeProperty<'blob> for &'blob ByteStr {
    fn deserialize_property(
        pctx: &mut PropertyContext<'_, 'blob>,
    ) -> Result<Self, DeserializeError> {
        let bytes = deserialize_byte_str_until_first_nul(pctx)?;
        Ok(ByteStr::new(bytes))
    }
}

impl<'blob> DeserializeProperty<'blob> for StrList<'blob> {
    fn deserialize_property(
        pctx: &mut PropertyContext<'_, 'blob>,
    ) -> Result<Self, DeserializeError> {
        let bytes = deserialize_byte_str_until_last_nul(pctx)?;
        let s = str::from_utf8(bytes).map_err(|source| pctx.error_invalid_string_value(source))?;
        Ok(Self::new(s))
    }
}

impl<'blob> DeserializeProperty<'blob> for ByteStrList<'blob> {
    fn deserialize_property(
        pctx: &mut PropertyContext<'_, 'blob>,
    ) -> Result<Self, DeserializeError> {
        let bytes = deserialize_byte_str_until_last_nul(pctx)?;
        Ok(Self::new(ByteStr::new(bytes)))
    }
}

impl<'blob> DeserializeProperty<'blob> for Compatible<'blob> {
    fn deserialize_property(
        pctx: &mut PropertyContext<'_, 'blob>,
    ) -> Result<Self, DeserializeError> {
        pctx.deserialize_property().map(Self::new)
    }
}

impl<'blob> DeserializeProperty<'blob> for Model<'blob> {
    fn deserialize_property(
        pctx: &mut PropertyContext<'_, 'blob>,
    ) -> Result<Self, DeserializeError> {
        pctx.deserialize_property().map(Self::new)
    }
}

impl<'blob> DeserializeProperty<'blob> for Phandle {
    fn deserialize_property(
        pctx: &mut PropertyContext<'_, 'blob>,
    ) -> Result<Self, DeserializeError> {
        pctx.deserialize_property().map(Self::new)
    }
}

impl<'blob> DeserializeProperty<'blob> for AddressCells {
    fn deserialize_property(
        pctx: &mut PropertyContext<'_, 'blob>,
    ) -> Result<Self, DeserializeError> {
        pctx.deserialize_property().map(Self::new)
    }
}

impl<'blob> DeserializeProperty<'blob> for SizeCells {
    fn deserialize_property(
        pctx: &mut PropertyContext<'_, 'blob>,
    ) -> Result<Self, DeserializeError> {
        pctx.deserialize_property().map(Self::new)
    }
}

impl<'blob> DeserializeProperty<'blob> for InterruptCells {
    fn deserialize_property(
        pctx: &mut PropertyContext<'_, 'blob>,
    ) -> Result<Self, DeserializeError> {
        pctx.deserialize_property().map(Self::new)
    }
}

impl<'blob> DeserializeProperty<'blob> for Status {
    fn deserialize_property(
        pctx: &mut PropertyContext<'_, 'blob>,
    ) -> Result<Self, DeserializeError> {
        match &**pctx.deserialize_property::<&ByteStr>()? {
            b"okay" => Ok(Self::Okay),
            b"disabled" => Ok(Self::Disabled),
            b"reserved" => Ok(Self::Reserved),
            b"fail" => Ok(Self::Fail),
            b"fail-sss" => Ok(Self::FailSss),
            _ => Err(pctx.error_custom("invalid status")),
        }
    }
}

impl<'blob> DeserializeProperty<'blob> for Reg<'blob> {
    fn deserialize_property(
        pctx: &mut PropertyContext<'_, 'blob>,
    ) -> Result<Self, DeserializeError> {
        let AddressSpace {
            address_cells,
            size_cells,
        } = pctx
            .node()
            .deserialize_parent()?
            .ok_or_else(|| pctx.node().error_missing_parent_node())?;

        let unit = (address_cells.value() + size_cells.value()) * size_of::<u32>();
        let value = pctx.property().value();
        let len = value.len();
        if !len.is_multiple_of(unit) {
            return Err(pctx.error_invalid_value_length(len.next_multiple_of(unit)));
        }
        let (value, rest) = value.as_chunks();
        assert!(rest.is_empty());

        Ok(Self::new(address_cells, size_cells, value))
    }
}

impl<'blob> DeserializeProperty<'blob> for Ranges<'blob> {
    fn deserialize_property(
        pctx: &mut PropertyContext<'_, 'blob>,
    ) -> Result<Self, DeserializeError> {
        let AddressSpace {
            address_cells: child_address_cells,
            size_cells: child_size_cells,
        } = pctx.node().deserialize_node()?;

        let AddressSpace {
            address_cells: parent_address_cells,
            ..
        } = pctx
            .node()
            .deserialize_parent()?
            .ok_or_else(|| pctx.node().error_missing_parent_node())?;

        let unit =
            (child_address_cells.value() + child_size_cells.value() + parent_address_cells.value())
                * size_of::<u32>();
        let value = pctx.property().value();
        let len = value.len();
        if !len.is_multiple_of(unit) {
            return Err(pctx.error_invalid_value_length(len.next_multiple_of(unit)));
        }
        let (value, rest) = value.as_chunks();
        assert!(rest.is_empty());

        Ok(Self::new(
            child_address_cells,
            child_size_cells,
            parent_address_cells,
            value,
        ))
    }
}
