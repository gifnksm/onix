use alloc::{borrow::ToOwned, boxed::Box, string::String, sync::Arc, vec::Vec};
use core::fmt;

use devicetree::parsed::{
    Devicetree,
    node::{Interrupt, Node, PropertyError},
};
use snafu::{OptionExt as _, ResultExt as _, Snafu, ensure};
use snafu_utils::Location;

use super::SerialDevice;
use crate::{
    drivers::{
        irq::plic::{self, Plic, PlicSource},
        serial::ns16550a,
    },
    sync::spinlock::SpinMutex,
};

struct NodeNameFormat<'a> {
    name: &'a String,
    address: &'a Option<String>,
}

impl fmt::Display for NodeNameFormat<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(address) = &self.address {
            write!(f, "{}@{}", self.name, address)
        } else {
            write!(f, "{}", self.name)
        }
    }
}

#[derive(Debug, Snafu)]
pub enum ParseDevicetreeError {
    #[snafu(display("missing `soc` node in devicetree"))]
    MissingSocNode {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("failed to parse `{name}` node: {source}", name = NodeNameFormat { name, address }))]
    ParseSerialNode {
        name: String,
        address: Option<String>,
        #[snafu(source)]
        source: ParseSerialNodeError,
        #[snafu(implicit)]
        location: Location,
    },
}

#[derive(Debug, Snafu)]
pub enum ParseSerialNodeError {
    #[snafu(display("failed to get property in `serial` node: {source}"))]
    Property {
        #[snafu(source)]
        source: PropertyError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("`reg` property contains no addresses"))]
    NoAddressInReg {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("`reg` property contains too many addresses"))]
    TooManyAddressesReg {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("unsupporeted serial device: {path}"))]
    UnsupportedDevice {
        path: String,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("no PLIC found for any interrupt"))]
    NoPlicFound {
        #[snafu(implicit)]
        location: Location,
    },
}

pub(super) fn parse(
    dtree: &Devicetree,
) -> Result<Vec<Arc<SpinMutex<SerialDevice>>>, Box<ParseDevicetreeError>> {
    let root = dtree.root_node();
    let soc_node = root
        .children()
        .find(|node| node.name() == "soc")
        .context(MissingSocNodeSnafu)?;
    let serial_drivers = soc_node
        .children()
        .filter(|node| node.name() == "serial")
        .map(|serial_node| {
            SerialDevice::parse(&serial_node)
                .with_context(|_e| ParseSerialNodeSnafu {
                    name: serial_node.name().to_owned(),
                    address: serial_node.address().map(ToOwned::to_owned),
                })
                .map(SpinMutex::new)
                .map(Arc::new)
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(serial_drivers)
}

fn find_plic_source(
    interrupts: &[Interrupt],
) -> Result<(Arc<Plic>, PlicSource), ParseSerialNodeError> {
    for interrupt in interrupts {
        let Some(plic) = plic::find_plic_by_dtree_path(&interrupt.interrupt_parent.path()) else {
            continue;
        };
        let source = plic.translate_interrupt_specifier(&interrupt.specifier);
        return Ok((plic, source));
    }
    Err(NoPlicFoundSnafu.build())
}

impl SerialDevice {
    fn parse(serial_node: &Node) -> Result<Self, ParseSerialNodeError> {
        let interrupts = serial_node.interrupts().context(PropertySnafu)?;
        let (plic, source) = find_plic_source(&interrupts)?;
        let mut reg_iter = serial_node.reg().context(PropertySnafu)?;
        let reg = reg_iter.next().context(NoAddressInRegSnafu)?;
        ensure!(reg_iter.next().is_none(), TooManyAddressesRegSnafu);
        let base_addr = reg.address;
        let size = reg.size;
        let uart_clock_frequency = serial_node
            .fetch_property_as::<u32>("clock-frequency")
            .context(PropertySnafu)?;

        let driver = if serial_node.is_compatible_to("ns16550a") {
            Box::new(unsafe { ns16550a::Driver::new(base_addr, size, uart_clock_frequency) })
        } else {
            return Err(UnsupportedDeviceSnafu {
                path: serial_node.path(),
            }
            .build());
        };
        Ok(Self {
            plic,
            source,
            driver,
        })
    }
}
