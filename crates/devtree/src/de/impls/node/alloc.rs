use alloc::vec::Vec;

use devtree_derive::DeserializeNode;

use crate::{
    de::{DeserializeError, DeserializeNode, NodeContext},
    types::{
        ByteString,
        node::{Interrupt, InterruptGeneratingDevice, NodePath},
        property::{InterruptCells, Phandle, U32Array},
    },
};

impl<'blob> DeserializeNode<'blob> for NodePath {
    fn deserialize_node(nctx: &mut NodeContext<'_, 'blob>) -> Result<Self, DeserializeError> {
        let mut path = ByteString::default();
        dump_path(&mut path, nctx);
        Ok(Self::new(path))
    }
}

fn dump_path(out: &mut ByteString, nctx: &NodeContext<'_, '_>) {
    match nctx.parent() {
        Some(parent) => {
            dump_path(out, &parent);
            if !out.ends_with(b"/") {
                out.push(b'/');
            }
            out.extend_from_slice(nctx.node().full_name());
        }
        None => {
            out.push(b'/');
        }
    }
}

#[derive(DeserializeNode)]
#[devtree(crate = crate)]
struct InterruptParentNode {
    #[devtree(node)]
    path: NodePath,
    #[devtree(property = "#interrupt-cells")]
    interrupt_cells: InterruptCells,
}

impl<'blob> DeserializeNode<'blob> for InterruptGeneratingDevice<'blob> {
    fn deserialize_node(nctx: &mut NodeContext<'_, 'blob>) -> Result<Self, DeserializeError> {
        let mut interrupts = None;
        let mut interrupt_parent = None;
        let mut interrupt_extended = None;
        while let Some(item) = nctx.read_item()? {
            let Some(pctx) = item.into_property() else {
                break;
            };
            let property = pctx.property();
            match &**property.name() {
                b"interrupts" => interrupts = Some(property.value()),
                b"interrupt-parent" => {
                    interrupt_parent = Some(pctx.deserialize_property::<Phandle>()?);
                }
                b"interrupts-extended" => interrupt_extended = Some(property.value()),
                _ => {}
            }
        }

        if let Some(interrupts_extended) = interrupt_extended {
            let (mut chunks, rest) = interrupts_extended.as_chunks::<{ size_of::<u32>() }>();
            if !rest.is_empty() {
                return Err(
                    nctx.error_custom("invalid property value length of `interrupts-extended`")
                );
            }

            let mut interrupts = Vec::new();

            while let Some((phandle, rest)) = chunks.split_first() {
                let phandle = Phandle::new(u32::from_be_bytes(*phandle));
                chunks = rest;

                let InterruptParentNode {
                    path,
                    interrupt_cells,
                } = nctx
                    .root()
                    .deserialize_node_by_phandle(phandle)?
                    .ok_or_else(|| nctx.root().error_missing_phandle_node(phandle))?;

                let (specifier, rest) = chunks
                    .split_at_checked(interrupt_cells.value())
                    .ok_or_else(|| {
                        nctx.error_custom("invalid property value length of `interrupts-extended`")
                    })?;
                chunks = rest;

                let specifier = U32Array::new(specifier);
                interrupts.push(Interrupt::new(path.0, specifier));
            }

            return Ok(Self::new(interrupts));
        }

        if let Some(interrupts) = interrupts {
            let (chunks, rest) = interrupts.as_chunks::<{ size_of::<u32>() }>();
            if !rest.is_empty() {
                return Err(nctx.error_custom("invalid property value length of `interrupts`"));
            }

            let InterruptParentNode {
                path,
                interrupt_cells,
            } = match interrupt_parent {
                None => nctx
                    .deserialize_parent()?
                    .ok_or_else(|| nctx.error_missing_parent_node())?,
                Some(phandle) => nctx
                    .root()
                    .deserialize_node_by_phandle(phandle)?
                    .ok_or_else(|| nctx.root().error_missing_phandle_node(phandle))?,
            };

            if !chunks.len().is_multiple_of(interrupt_cells.value()) {
                return Err(nctx.error_custom("invalid property value length of `interrupts`"));
            }

            return Ok(Self::new(
                chunks
                    .chunks_exact(interrupt_cells.value())
                    .map(|specifier| Interrupt::new(path.0.clone(), U32Array::new(specifier)))
                    .collect(),
            ));
        }

        Err(nctx.error_missing_properties(&["interrupts", "interrupts-extended"]))
    }
}
