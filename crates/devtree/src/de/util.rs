use platform_cast::CastFrom as _;

use super::{DeserializeError, NodeContext, PropertyCollection, PropertyContext};

pub fn deserialize_usize_property_from_u32(
    pctx: &mut PropertyContext<'_, '_>,
) -> Result<usize, DeserializeError> {
    let value = pctx.deserialize_property::<u32>()?;
    Ok(usize::cast_from(value))
}

pub fn deserialize_u64_or_u32_property(
    pctx: &mut PropertyContext<'_, '_>,
) -> Result<u64, DeserializeError> {
    let value = pctx.property().value();
    let value = match value.len() {
        4 => u64::from(pctx.deserialize_property::<u32>()?),
        8 => pctx.deserialize_property::<u64>()?,
        _ => return Err(pctx.error_custom("u32 or u64 expected")),
    };
    Ok(value)
}

pub fn deserialize_node_as_property_collection<'blob, T>(
    nctx: &mut NodeContext<'_, 'blob>,
) -> Result<T, DeserializeError>
where
    T: PropertyCollection<'blob>,
{
    let mut collection = T::default();
    while let Some(item) = nctx.read_item()? {
        if let Some(mut pctx) = item.into_property() {
            collection.insert_property(&mut pctx)?;
        }
    }
    Ok(collection)
}
