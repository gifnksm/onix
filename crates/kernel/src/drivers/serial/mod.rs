use alloc::{boxed::Box, format, sync::Arc, vec::Vec};
use core::{error::Error, fmt};

use devtree::{
    Devicetree,
    types::{ByteStr, ByteString},
};
use snafu::ResultExt as _;
use spin::Once;

use super::irq::plic::{Plic, PlicSource};
use crate::{
    cpu,
    error::GenericError,
    sync::spinlock::{SpinMutex, SpinMutexCondVar},
};

mod de;
mod ns16550a;

trait SerialDriver: fmt::Debug + Send + Sync {
    fn init(&mut self) -> Result<(), Box<dyn Error>>;
    fn is_tx_idle(&mut self) -> bool;
    fn is_rx_ready(&mut self) -> bool;
    fn set_tx_idle_interrupt(&mut self, enable: bool);
    fn set_rx_ready_interrupt(&mut self, enable: bool);
    fn write(&mut self, bytes: &[u8]) -> usize;
    fn read(&mut self, bytes: &mut [u8]) -> usize;
    fn complete(&mut self);
}

const SERIAL_PRIORITY: u32 = 1;
const SERIAL_THRESHOLD: u32 = 0;
static SERIAL_DRIVERS: Once<Vec<Arc<SerialDevice>>> = Once::new();

pub fn init(dt: &Devicetree) -> Result<(), GenericError> {
    let drivers = de::deserialize(dt).whatever_context("failed to deserialize devicetree")?;
    for driver in &drivers {
        driver.init()?;
        let plic = Arc::clone(&driver.plic);
        let source = driver.source;

        let callback = Arc::new({
            let driver = Arc::clone(driver);
            move |_context| {
                driver.handle_interrupt();
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

pub fn find_serial_by_dtree_path<P>(path: P) -> Option<Arc<SerialDevice>>
where
    P: AsRef<ByteStr>,
{
    let path = path.as_ref();
    SERIAL_DRIVERS
        .get()?
        .iter()
        .find(|device| device.path == path)
        .cloned()
}

#[derive(Debug)]
pub struct SerialDevice {
    path: ByteString,
    plic: Arc<Plic>,
    source: PlicSource,
    driver: SpinMutex<Box<dyn SerialDriver>>,
    rx_ready: SpinMutexCondVar,
    tx_idle: SpinMutexCondVar,
}

impl SerialDevice {
    fn new(
        path: ByteString,
        plic: Arc<Plic>,
        source: PlicSource,
        driver: Box<dyn SerialDriver>,
    ) -> Self {
        Self {
            path,
            plic,
            source,
            driver: SpinMutex::new(driver),
            rx_ready: SpinMutexCondVar::new(),
            tx_idle: SpinMutexCondVar::new(),
        }
    }

    fn init(&self) -> Result<(), GenericError> {
        let mut driver = self.driver.lock();
        driver.init().with_whatever_context(|_| {
            format!(
                "failed to initialize serial device driver, path={}",
                self.path,
            )
        })?;
        driver.set_rx_ready_interrupt(true);
        Ok(())
    }

    fn handle_interrupt(&self) {
        let mut driver = self.driver.lock();

        if driver.is_rx_ready() {
            self.rx_ready.notify_all();
            driver.set_rx_ready_interrupt(false);
        }
        if driver.is_tx_idle() {
            self.tx_idle.notify_all();
            driver.set_tx_idle_interrupt(false);
        }
        driver.complete();
    }

    pub fn read(&self, bytes: &mut [u8]) -> usize {
        if bytes.is_empty() {
            return 0;
        }

        let mut driver = self.driver.lock();

        loop {
            let nread = driver.read(bytes);
            if nread > 0 {
                return nread;
            }

            driver.set_rx_ready_interrupt(true);
            driver = self.rx_ready.wait(driver);
        }
    }

    pub fn write(&self, bytes: &[u8]) -> usize {
        if bytes.is_empty() {
            return 0;
        }

        let mut driver = self.driver.lock();

        loop {
            let nwritten = driver.write(bytes);
            if nwritten > 0 {
                return nwritten;
            }

            driver.set_tx_idle_interrupt(true);
            driver = self.tx_idle.wait(driver);
        }
    }
}
