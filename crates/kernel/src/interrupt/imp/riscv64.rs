use core::arch::asm;

use riscv::register::sstatus;

#[derive(Debug, Clone, Copy)]
pub struct State(bool);

pub fn read_and_disable() -> State {
    let flags: u8;
    unsafe {
        asm!(
            "csrrci {rd}, sstatus, 0b10",
            rd = out(reg) flags,
            options(preserves_flags, nostack)
        );
        State(flags != 0)
    }
}

pub fn is_enable() -> bool {
    sstatus::read().sie()
}

pub fn restore(state: State) {
    let flags: u8 = if state.0 { 1 } else { 0 };
    unsafe {
        asm!(
            "csrs sstatus, {rsi}",
            rsi = in(reg) flags,
            options(preserves_flags, nostack)
        );
    }
}
