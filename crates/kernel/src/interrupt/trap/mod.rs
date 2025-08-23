use alloc::sync::Arc;

use riscv::{
    interrupt::{Exception, Interrupt, Trap},
    register::{
        scause, sepc,
        sstatus::{self, SPP},
        stval,
    },
};
use spin::Once;

use crate::{
    cpu,
    drivers::irq::plic::{self, Plic, PlicContext},
};

cpu_local! {
    static PLIC_CONTEXT: Once<(Arc<Plic>, PlicContext)> = Once::new();
}

mod imp;

pub fn apply() {
    imp::apply();
    let cpu = cpu::current();
    let plic_ctx = plic::find_plic_context_for_cpu(cpu.id()).unwrap();
    PLIC_CONTEXT.get().call_once(|| plic_ctx);
}

pub(super) extern "C" fn trap_kernel() {
    super::cpu_state().increment_irq_depth();
    let sepc = sepc::read();
    let sstatus = sstatus::read();
    let stval = stval::read();
    let scause: Trap<Interrupt, Exception> = scause::read().cause().try_into().unwrap();

    assert_eq!(sstatus.spp(), SPP::Supervisor, "from supervisor mode");
    assert!(!super::is_enabled());

    match scause {
        Trap::Exception(e) => {
            panic!("unexpected kernel exception {e:#?}, sepc={sepc:#x}, stval={stval:#x}");
        }
        Trap::Interrupt(Interrupt::SupervisorTimer) => {
            super::timer::handle_interrupt();
        }
        Trap::Interrupt(Interrupt::SupervisorExternal) => {
            let (plic, plic_ctx) = PLIC_CONTEXT.get().get().unwrap();
            let _handled = plic.handle_interrupt(*plic_ctx);
        }
        Trap::Interrupt(int) => {
            panic!("unexpected kernel interrupt {int:#?}, sepc={sepc:#x}, stval={stval:#x}");
        }
    }

    // yield_execution (called in timer::handle_interrupt()) may transition the
    // current task to other CPUs, so restore trap registers.
    unsafe {
        sepc::write(sepc);
    }
    unsafe {
        sstatus::write(sstatus);
    }
    super::cpu_state().decrement_irq_depth();
}
