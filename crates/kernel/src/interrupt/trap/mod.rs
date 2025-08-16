use riscv::{
    interrupt::{Exception, Interrupt, Trap},
    register::{
        scause, sepc,
        sstatus::{self, SPP},
        stval,
    },
};

mod imp;

pub fn apply() {
    imp::apply();
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
