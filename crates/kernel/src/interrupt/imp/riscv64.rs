use core::arch::asm;

use riscv::register::sstatus;

const SSTATUS_SIE: usize = 0b10;

#[derive(Debug, Clone, Copy)]
pub struct State(bool);

pub fn read_and_disable() -> State {
    let sstatus: usize;
    unsafe {
        asm!(
            "csrrci {rd}, sstatus, {sstatus_sie}",
            rd = out(reg) sstatus,
            sstatus_sie = const SSTATUS_SIE,
            options(preserves_flags, nostack)
        );
        State((sstatus & SSTATUS_SIE) != 0)
    }
}

pub fn is_enabled() -> bool {
    sstatus::read().sie()
}

pub fn restore(state: State) {
    if state.0 {
        unsafe {
            sstatus::set_sie();
        }
    } else {
        unsafe {
            sstatus::clear_sie();
        }
    }
}
