use alloc::{boxed::Box, sync::Arc, vec::Vec};
use core::{error::Error, fmt};

use devtree::Devicetree;
use snafu::{ResultExt as _, Snafu};
use snafu_utils::Location;
use spin::Once;

use super::irq::plic::{Plic, PlicSource};
use crate::{cpu, sync::spinlock::SpinMutex};

mod de;
mod ns16550a;

trait SerialDriver: fmt::Debug + Send + Sync {
    fn init(&mut self) -> Result<(), Box<dyn Error>>;
    fn is_tx_idle(&mut self) -> bool;
    fn is_rx_ready(&mut self) -> bool;
    fn write(&mut self, bytes: &[u8]) -> usize;
    fn read(&mut self, bytes: &mut [u8]) -> usize;
    fn complete(&mut self);
}

#[derive(Debug)]
struct SerialDevice {
    plic: Arc<Plic>,
    source: PlicSource,
    driver: Box<dyn SerialDriver>,
}

#[derive(Debug, Snafu)]
#[snafu(module)]
pub enum SerialInitError {
    #[snafu(display("failed to deserialize devicetree"))]
    #[snafu(provide(ref, priority, Location => location))]
    DeserializeDevicetree {
        #[snafu(source)]
        source: de::DeserializeDevicetreeError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("failed to initialize driver"))]
    #[snafu(provide(ref, priority, Location => location))]
    DriverInit {
        #[snafu(source)]
        source: Box<dyn Error>,
        #[snafu(implicit)]
        location: Location,
    },
}

const SERIAL_PRIORITY: u32 = 1;
const SERIAL_THRESHOLD: u32 = 0;
static SERIAL_DRIVERS: Once<Vec<Arc<SpinMutex<SerialDevice>>>> = Once::new();

pub fn init(dt: &Devicetree) -> Result<(), Box<SerialInitError>> {
    #[cfg_attr(not(test), expect(clippy::wildcard_imports))]
    use self::serial_init_error::*;

    let mut drivers = de::deserialize(dt).context(DeserializeDevicetreeSnafu)?;
    for driver_ref in &mut drivers {
        let mut driver = driver_ref.lock();
        driver.driver.init().context(DriverInitSnafu)?;
        let plic = Arc::clone(&driver.plic);
        let source = driver.source;
        driver.unlock();

        let callback = Arc::new({
            let driver_ref = Arc::clone(driver_ref);
            move |_context| {
                let mut driver = driver_ref.lock();
                if driver.driver.is_rx_ready() {
                    let mut buf = [0_u8; 64];
                    let n = driver.driver.read(&mut buf);
                    if n > 0 {
                        // echo back
                        let _ = driver.driver.write(&buf[..n]);
                    }
                }
                driver.driver.complete();
            }
        });

        plic.set_priority(source, SERIAL_PRIORITY);
        plic.register_callback(source, callback);
    }
    SERIAL_DRIVERS.call_once(|| drivers);
    Ok(())
}

pub fn apply() {
    for driver in SERIAL_DRIVERS.get().unwrap() {
        let driver = driver.lock();
        let cpu = cpu::current();
        let Some(context) = driver.plic.find_context_for_cpu(cpu.id()) else {
            continue;
        };
        driver
            .plic
            .set_priority_threshold(context, SERIAL_THRESHOLD);
        driver.plic.enable_interrupt(driver.source, context);
    }
}
