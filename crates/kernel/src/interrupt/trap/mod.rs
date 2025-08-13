use riscv::{
    interrupt::{Exception, Interrupt, Trap},
    register::{
        scause, sepc,
        sstatus::{self, SPP},
        stval,
    },
};

use crate::task::scheduler;

mod imp;

pub fn apply() {
    imp::apply();
}

pub(super) extern "C" fn trap_kernel() {
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
            if let Some(task) = scheduler::current_task() {
                let mut shared = task.shared.lock();
                scheduler::yield_execution(&mut shared);
            }
        }
        Trap::Interrupt(int) => {
            panic!("unexpected kernel interrupt {int:#?}, sepc={sepc:#x}, stval={stval:#x}");
        }
    }

    // yield_execution may transition the current task to other CPUs,
    // so restore trap registers.
    unsafe {
        sepc::write(sepc);
    }
    unsafe {
        sstatus::write(sstatus);
    }
}
