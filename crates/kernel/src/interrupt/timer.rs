use core::{arch::asm, time::Duration};

use riscv::register::scounteren;

const NANOS_PER_CLOCK: u64 = 100;
const NANOS_PER_SEC: u64 = 1_000_000_000;
const TICKS_PER_SEC: u64 = 10;
const NANOS_PER_TICK: u64 = NANOS_PER_SEC / TICKS_PER_SEC;
const CLOCKS_PER_TICK: u64 = NANOS_PER_TICK / NANOS_PER_CLOCK;

pub fn start() {
    // allow user to use time.
    unsafe {
        scounteren::set_tm();
    }

    // ask for the very first timer interrupt.
    unsafe {
        let time: u64;
        asm!("csrr {}, time", out(reg) time);
        asm!("csrw stimecmp, {}", in(reg) time);
    }
}

pub(super) fn handle_interrupt() {
    let time: u64;

    // ask for the next timer interrupt. this also clears
    // the interrupt request. 1_000_000 is about a tenth
    // of a second.
    unsafe {
        asm!("csrr {}, time", out(reg) time);
        asm!("csrw stimecmp, {}", in(reg) time + CLOCKS_PER_TICK);
    }
}

pub fn now() -> Duration {
    let time: u64;
    unsafe {
        asm!("csrr {}, time", out(reg) time);
    }
    Duration::from_nanos(time * NANOS_PER_CLOCK)
}
