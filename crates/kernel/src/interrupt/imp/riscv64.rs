use core::arch::asm;

use riscv::register::sstatus;

const SSTATUS_SIE: usize = 0b10;

#[derive(Debug, Clone, Copy)]
pub struct State {
    enabled: bool,
}

impl State {
    pub const fn new() -> Self {
        Self { enabled: false }
    }
}

pub fn read_and_disable() -> State {
    let sstatus: usize;
    unsafe {
        asm!(
            "csrrci {rd}, sstatus, {sstatus_sie}",
            rd = out(reg) sstatus,
            sstatus_sie = const SSTATUS_SIE,
            options(preserves_flags, nostack)
        );
    }
    State {
        enabled: (sstatus & SSTATUS_SIE) != 0,
    }
}

pub fn is_enabled() -> bool {
    sstatus::read().sie()
}

pub fn restore(state: State) {
    assert!(!is_enabled());
    if state.enabled {
        unsafe {
            sstatus::set_sie();
        }
    }
}

pub fn enable() {
    unsafe {
        riscv::interrupt::enable();
    }
}

pub fn disable() {
    riscv::interrupt::disable();
}

pub fn wait() {
    riscv::asm::wfi();
}
