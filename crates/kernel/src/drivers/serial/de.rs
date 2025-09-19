use alloc::{boxed::Box, sync::Arc, vec::Vec};

use devtree::{
    DeserializeNode, Devicetree,
    cursor::ReadNodeError,
    de::DeserializeError,
    types::{
        ByteString,
        node::{Interrupt, InterruptGeneratingDevice, NodePath},
        property::{Compatible, Reg},
    },
};
use snafu::{OptionExt as _, ResultExt as _, Snafu};
use snafu_utils::Location;

use super::SerialDevice;
use crate::{
    drivers::{
        irq::plic::{self, Plic, PlicSource},
        serial::ns16550a,
    },
    iter::IteratorExt as _,
};

#[derive(Debug, Snafu)]
#[snafu(module)]
pub enum DeserializeDevicetreeError {
    #[snafu(display("failed to read devicetree node"))]
    #[snafu(provide(ref, priority, Location => location))]
    Read {
        #[snafu(implicit)]
        location: Location,
        #[snafu(source)]
        source: ReadNodeError,
    },
    #[snafu(display("failed to deserialize serial node"))]
    #[snafu(provide(ref, priority, Location => location))]
    DeserializeSerialNode {
        #[snafu(implicit)]
        location: Location,
        #[snafu(source)]
        source: DeserializeError,
    },
    #[snafu(display("invalid 'reg' entries in serial node"))]
    #[snafu(provide(ref, priority, Location => location))]
    InvalidRegSerial {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("unsupporeted serial device: {path}"))]
    #[snafu(provide(ref, priority, Location => location))]
    UnsupportedDevice {
        path: ByteString,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("no PLIC found for any interrupt"))]
    #[snafu(provide(ref, priority, Location => location))]
    NoPlicFound {
        #[snafu(implicit)]
        location: Location,
    },
}

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

pub fn deserialize(dt: &Devicetree) -> Result<Vec<Arc<SerialDevice>>, DeserializeDevicetreeError> {
    #[cfg_attr(not(test), expect(clippy::wildcard_imports))]
    use self::deserialize_devicetree_error::*;

    let mut serial_devices = Vec::new();

    let root_node = dt.read_root_node().context(ReadSnafu)?;
    root_node
        .try_visit_deserialize_all_nodes_by_query("/soc/serial", |serial_node| {
            let device = SerialDevice::from_node(serial_node)?;
            serial_devices.push(Arc::new(device));
            Ok(())
        })
        .context(DeserializeSerialNodeSnafu)?
        .map_or(Ok(()), Err)?;
    Ok(serial_devices)
}

fn find_plic_source(
    interrupts: &[Interrupt],
) -> Result<(Arc<Plic>, PlicSource), DeserializeDevicetreeError> {
    #[cfg_attr(not(test), expect(clippy::wildcard_imports))]
    use self::deserialize_devicetree_error::*;

    for interrupt in interrupts {
        let Some(plic) = plic::find_plic_by_dtree_path(interrupt.parent_path()) else {
            continue;
        };
        let source = plic.translate_interrupt_specifier(interrupt.specifier());
        return Ok((plic, source));
    }
    NoPlicFoundSnafu.fail()
}

impl SerialDevice {
    fn from_node(serial_node: SerialNode<'_>) -> Result<Self, DeserializeDevicetreeError> {
        #[cfg_attr(not(test), expect(clippy::wildcard_imports))]
        use self::deserialize_devicetree_error::*;

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
            .context(InvalidRegSerialSnafu)?;
        let base_addr = reg.range().start;
        let size = reg.range().len();

        let driver = if compatible.is_compatible_to("ns16550a") {
            Box::new(unsafe { ns16550a::Driver::new(base_addr, size, clock_frequency) })
        } else {
            return UnsupportedDeviceSnafu { path: path.0 }.fail();
        };
        Ok(Self::new(path.0, plic, source, driver))
    }
}
