use alloc::{boxed::Box, sync::Arc, vec::Vec};

use devtree::{
    DeserializeNode, Devicetree,
    tree_cursor::{TreeCursor as _, TreeIterator as _},
    types::{
        node::{Interrupt, InterruptGeneratingDevice, NodePath},
        property::{Compatible, Reg},
    },
};
use snafu::{OptionExt as _, ResultExt as _, whatever};

use super::SerialDevice;
use crate::{
    drivers::{
        irq::plic::{self, Plic, PlicSource},
        serial::ns16550a,
    },
    error::GenericError,
    iter::IteratorExt as _,
};

#[derive(Debug, DeserializeNode)]
struct SerialNode<'blob> {
    #[devtree(node)]
    path: NodePath,
    #[devtree(node)]
    device: InterruptGeneratingDevice<'blob>,
    #[devtree(property(name = "clock-frequency"))]
    clock_frequency: u32,
    #[devtree(property)]
    reg: Reg<'blob>,
    #[devtree(property)]
    compatible: Compatible<'blob>,
}

pub fn deserialize(dt: &Devicetree) -> Result<Vec<Arc<SerialDevice>>, GenericError> {
    let mut serial_devices = Vec::new();

    let mut cursor = dt.tree_cursor();
    let iter = cursor
        .read_descendant_nodes_by_glob("/soc/serial")
        .deserialize_node::<SerialNode>();
    for serial_node in iter {
        let serial_node =
            serial_node.whatever_context("failed to deserialize serial node in devicetree")?;
        let device = SerialDevice::from_node(serial_node)?;
        serial_devices.push(Arc::new(device));
    }
    Ok(serial_devices)
}

fn find_plic_source(interrupts: &[Interrupt]) -> Result<(Arc<Plic>, PlicSource), GenericError> {
    for interrupt in interrupts {
        let Some(plic) = plic::find_plic_by_dtree_path(interrupt.parent_path()) else {
            continue;
        };
        let source = plic.translate_interrupt_specifier(interrupt.specifier());
        return Ok((plic, source));
    }
    whatever!("no plic device found")
}

impl SerialDevice {
    fn from_node(serial_node: SerialNode<'_>) -> Result<Self, GenericError> {
        let SerialNode {
            path,
            device,
            clock_frequency,
            reg,
            compatible,
        } = serial_node;
        let (plic, source) = find_plic_source(device.interrupts())?;
        let reg = reg
            .into_iter()
            .assume_one()
            .whatever_context("invalid 'reg' entries in serial node")?;
        let base_addr = reg.range().start;
        let size = reg.range().len();

        let driver = if compatible.is_compatible_to("ns16550a") {
            Box::new(unsafe { ns16550a::Driver::new(base_addr, size, clock_frequency) })
        } else {
            whatever!("unsupported serial device, compatible={compatible:?}");
        };
        Ok(Self::new(path.0, plic, source, driver))
    }
}
