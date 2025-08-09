use core::{arch::asm, time::Duration};

use riscv::register::scounteren;

use super::super::cpu;

const NANOS_PER_SEC: u64 = 1_000_000_000;
const TICKS_PER_SEC: u64 = 10;

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
    let timer_frequency = cpu::current().timer_frequency();
    let timer_increment = timer_frequency / TICKS_PER_SEC;

    let time: u64;

    unsafe {
        asm!("csrr {}, time", out(reg) time);
        asm!("csrw stimecmp, {}", in(reg) time + timer_increment);
    }
}

pub fn try_now() -> Option<Duration> {
    let timer_frequency = cpu::try_current()?.timer_frequency();

    let time: u64;
    unsafe {
        asm!("csrr {}, time", out(reg) time);
    }

    let sec = time / timer_frequency;
    let subsec = time % timer_frequency;
    let subsec_nanos = (subsec * NANOS_PER_SEC / timer_frequency)
        .try_into()
        .unwrap();

    Some(Duration::new(sec, subsec_nanos))
}
