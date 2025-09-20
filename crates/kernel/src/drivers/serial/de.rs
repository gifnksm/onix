use alloc::{boxed::Box, sync::Arc, vec::Vec};

use devtree::{
    DeserializeNode, Devicetree,
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

    let root_node = dt
        .read_root_node()
        .whatever_context("failed to devicetree root node")?;
    root_node
        .try_visit_deserialize_all_nodes_by_query("/soc/serial", |serial_node| {
            let device = SerialDevice::from_node(serial_node)?;
            serial_devices.push(Arc::new(device));
            Ok(())
        })
        .whatever_context("failed to deserialize devicetree serial node")?
        .map_or(Ok(()), Err)?;
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
