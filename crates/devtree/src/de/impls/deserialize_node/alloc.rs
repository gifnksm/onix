extern crate alloc;

use alloc::vec::Vec;

use devtree_derive::DeserializeNode;

use crate::{
    de::{
        DeserializeNode, DeserializeProperty as _, NodeDeserializer, PropertyDeserializer as _,
        error::{DeserializeError, DeserializeNodeError, DeserializePropertyError},
    },
    tree_cursor::{TreeCursor as _, TreeCursorAllocExt as _},
    types::{
        node::{Interrupt, InterruptGeneratingDevice, NodePath},
        property::{InterruptCells, Phandle, U32Array},
    },
};

impl<'blob> DeserializeNode<'blob> for NodePath {
    fn deserialize_node<'de, D>(de: &mut D) -> Result<Self, DeserializeError>
    where
        D: NodeDeserializer<'de, 'blob> + ?Sized,
    {
        Ok(Self::new(de.tree_cursor().path()))
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
    fn deserialize_node<'de, D>(de: &mut D) -> Result<Self, DeserializeError>
    where
        D: NodeDeserializer<'de, 'blob> + ?Sized,
    {
        let node = de.node().clone();

        let mut interrupts = None;
        let mut interrupt_parent = None;
        let mut interrupts_extended = None;
        de.with_properties(|mut sub_de| {
            let property = sub_de.property().clone();
            match &**property.name() {
                b"interrupts" => {
                    interrupts = Some((<&[[u8; 4]]>::deserialize_property(&mut sub_de)?, property));
                }
                b"interrupt-parent" => {
                    interrupt_parent =
                        Some((Phandle::deserialize_property(&mut sub_de)?, property));
                }
                b"interrupts-extended" => {
                    interrupts_extended =
                        Some((<&[[u8; 4]]>::deserialize_property(&mut sub_de)?, property));
                }
                _ => {}
            }
            Ok(())
        })?;

        if let Some((mut chunks, property)) = interrupts_extended {
            let mut interrupts = Vec::new();

            while let Some((phandle, rest)) = chunks.split_first() {
                let phandle = Phandle::new(u32::from_be_bytes(*phandle));
                chunks = rest;

                let mut root_cursor = de.clone_tree_cursor()?;
                let _root = root_cursor.seek_root_start()?;
                let InterruptParentNode {
                    path,
                    interrupt_cells,
                } = root_cursor
                    .read_node_by_phandle(phandle)?
                    .ok_or_else(|| DeserializeError::missing_phandle_node(phandle))?
                    .deserialize_node()?;

                let (specifier, rest) = chunks
                    .split_at_checked(interrupt_cells.value())
                    .ok_or_else(|| {
                        DeserializePropertyError::custom(
                            &property,
                            "invalid property value length of `interrupts-extended`",
                        )
                    })?;
                chunks = rest;

                let specifier = U32Array::new(specifier);
                interrupts.push(Interrupt::new(path.0, specifier));
            }

            return Ok(Self::new(interrupts));
        }

        if let Some((chunks, interrupts_property)) = interrupts {
            let InterruptParentNode {
                path,
                interrupt_cells,
            } = match interrupt_parent {
                Some((phandle, _phandle_property)) => {
                    let mut root_cursor = de.clone_tree_cursor()?;
                    let _root = root_cursor.seek_root_start()?;
                    root_cursor
                        .read_node_by_phandle(phandle)?
                        .ok_or_else(|| DeserializeError::missing_phandle_node(phandle))?
                        .deserialize_node()?
                }
                None => de
                    .clone_tree_cursor()?
                    .read_parent()?
                    .ok_or_else(|| DeserializeNodeError::missing_parent_node(&node))?
                    .deserialize_node()?,
            };

            if !chunks.len().is_multiple_of(interrupt_cells.value()) {
                return Err(DeserializePropertyError::custom(
                    &interrupts_property,
                    "invalid property value length of `interrupts`",
                )
                .into());
            }

            return Ok(Self::new(
                chunks
                    .chunks_exact(interrupt_cells.value())
                    .map(|specifier| Interrupt::new(path.0.clone(), U32Array::new(specifier)))
                    .collect(),
            ));
        }

        Err(DeserializeNodeError::custom(
            de.node(),
            "`interrupts` and `inteterrupts-extended` property is missing",
        )
        .into())
    }
}
