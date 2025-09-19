use alloc::boxed::Box;
use core::{error::Error, ops::Range, ptr};

use bitflags::bitflags;
use sv39::MapPageFlags;

use super::SerialDriver;
use crate::memory::{self, kernel_space};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Register {
    offset: usize,
}

// the UART control registers.
// some have different meanings for
// read vs write.
// see <http://byterunner.com/16550.html>

impl Register {
    /// Receive Holding Register (readonly)
    const RX_HOLDING: Self = Self::new(0);
    /// Transmit Holding Register (writeonly)
    const TX_HOLDING: Self = Self::new(0);
    /// Interrupt Enable Register (writeonly)
    const INTERRUPT_ENABLE: Self = Self::new(1);
    /// Interrupt Status Register (readonly)
    const INTERRUPT_STATUS: Self = Self::new(2);
    /// FIFO Control Register (writeonly)
    const FIFO_CONTROL: Self = Self::new(2);
    /// Line Control Register (writeonly)
    const LINE_CONTROL: Self = Self::new(3);
    #[expect(dead_code)]
    /// Modem Control Register (writeonly)
    const MODEM_CONTROL: Self = Self::new(4);
    /// Line Status Register (readonly)
    const LINE_STATUS: Self = Self::new(5);
    #[expect(dead_code)]
    /// Modem Status Register (readonly)
    const MODEM_STATUS: Self = Self::new(6);
    #[expect(dead_code)]
    /// Scratchpad Register (read/write)
    const SCRATCHPAD: Self = Self::new(7);

    /// LSB of Divisor Latch
    const DIVISOR_LATCH_LSB: Self = Self::new(0);
    /// MSB of Divisor Latch
    const DIVISOR_LATCH_MSB: Self = Self::new(1);

    const fn new(offset: usize) -> Self {
        Self { offset }
    }
}

bitflags! {
    #[repr(transparent)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct InterruptEnable : u8 {
        const RX_READY = 1 << 0;
        const TX_EMPTY = 1 << 1;
        const RX_LINE_STATUS = 1 << 2;
        const MODEM_STATUS = 1 << 3;
    }

    struct FifoControl : u8 {
        const FIFO_ENABLE = 1 << 0;
        const RX_FIFO_RESET = 1 << 1;
        const TX_FIFO_RESET = 1 << 2;
        const FIFO_RESET = Self::RX_FIFO_RESET.bits() | Self::TX_FIFO_RESET.bits();
    }

    struct LineControl : u8 {
        const EIGHT_BITS = 0b11;
        const BAUD_LATCH = 1 << 7;
    }

    struct LineStatus : u8 {
        const RX_READY = 1 << 0;
        const TX_IDLE = 1 << 5;
    }
}

#[derive(custom_debug_derive::Debug)]
pub(super) struct Driver {
    #[debug(format = "{:#x}")]
    base_addr: usize,
    #[debug(format = "{:#x}")]
    size: usize,
    uart_clock_frequency: u32,
}

impl Driver {
    pub(super) unsafe fn new(base_addr: usize, size: usize, uart_clock_frequency: u32) -> Self {
        Self {
            base_addr,
            size,
            uart_clock_frequency,
        }
    }

    fn range(&self) -> Range<usize> {
        self.base_addr..self.base_addr + self.size
    }

    fn register_addr(&self, reg: Register) -> usize {
        assert!(reg.offset < self.size);
        self.base_addr + reg.offset
    }

    unsafe fn write_register(&mut self, reg: Register, value: u8) {
        let addr = self.register_addr(reg);
        unsafe {
            ptr::with_exposed_provenance_mut::<u8>(addr).write_volatile(value);
        }
    }

    unsafe fn read_register(&mut self, reg: Register) -> u8 {
        let addr = self.register_addr(reg);
        unsafe { ptr::with_exposed_provenance::<u8>(addr).read_volatile() }
    }
}

impl SerialDriver for Driver {
    fn init(&mut self) -> Result<(), Box<dyn Error>> {
        kernel_space::identity_map_range(
            memory::expand_to_page_boundaries(self.range()),
            MapPageFlags::RW,
        )?;

        unsafe {
            // disable interrupts
            self.write_register(Register::INTERRUPT_ENABLE, 0x00);

            // special mode to set baud rate
            self.write_register(Register::LINE_CONTROL, LineControl::BAUD_LATCH.bits());
            let baud_rate = 38400; // 38.4k
            let divisor = u16::try_from(self.uart_clock_frequency / baud_rate).unwrap();
            let [divisor_msb, divisor_lsb] = divisor.to_be_bytes();
            self.write_register(Register::DIVISOR_LATCH_LSB, divisor_lsb);
            self.write_register(Register::DIVISOR_LATCH_MSB, divisor_msb);

            // leave set-baud mode and set word length to 8 bits, no parity
            self.write_register(Register::LINE_CONTROL, LineControl::EIGHT_BITS.bits());
            // reset and enable FIFOs
            self.write_register(
                Register::FIFO_CONTROL,
                (FifoControl::FIFO_ENABLE | FifoControl::FIFO_RESET).bits(),
            );

            // enable transmit and receive interrupts
            self.write_register(Register::INTERRUPT_ENABLE, InterruptEnable::empty().bits());
        }

        Ok(())
    }

    fn is_tx_idle(&mut self) -> bool {
        unsafe { self.read_register(Register::LINE_STATUS) & LineStatus::TX_IDLE.bits() != 0 }
    }

    fn is_rx_ready(&mut self) -> bool {
        unsafe { self.read_register(Register::LINE_STATUS) & LineStatus::RX_READY.bits() != 0 }
    }

    fn set_tx_idle_interrupt(&mut self, enable: bool) {
        unsafe {
            let mut ier =
                InterruptEnable::from_bits_retain(self.read_register(Register::INTERRUPT_ENABLE));
            ier.set(InterruptEnable::TX_EMPTY, enable);
            self.write_register(Register::INTERRUPT_ENABLE, ier.bits());
        }
    }

    fn set_rx_ready_interrupt(&mut self, enable: bool) {
        unsafe {
            let mut ier =
                InterruptEnable::from_bits_retain(self.read_register(Register::INTERRUPT_ENABLE));
            ier.set(InterruptEnable::RX_READY, enable);
            self.write_register(Register::INTERRUPT_ENABLE, ier.bits());
        }
    }

    fn write(&mut self, bytes: &[u8]) -> usize {
        let mut count = 0;
        for byte in bytes {
            if !self.is_tx_idle() {
                break;
            }
            unsafe {
                self.write_register(Register::TX_HOLDING, *byte);
            };
            count += 1;
        }
        count
    }

    fn read(&mut self, bytes: &mut [u8]) -> usize {
        let mut count = 0;
        for byte in bytes {
            if !self.is_rx_ready() {
                break;
            }
            unsafe {
                *byte = self.read_register(Register::RX_HOLDING);
            };
            count += 1;
        }
        count
    }

    fn complete(&mut self) {
        // read the interrupt status register to complete the interrupt
        unsafe {
            self.read_register(Register::INTERRUPT_STATUS);
        }
    }
}
