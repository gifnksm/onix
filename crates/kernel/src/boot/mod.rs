use alloc::format;
use core::{
    hint, mem, ptr,
    sync::atomic::{AtomicBool, Ordering},
};

use snafu::ResultExt as _;

use crate::{
    cpu::Cpuid,
    error::{self, GenericError},
    memory,
};

mod imp;

unsafe extern "C" {
    #[link_name = "__onix_boot_stack_top"]
    static mut BOOT_STACK_TOP: u8;
}

unsafe fn init_bss() {
    let bss_range = memory::layout::bss_addr_range();
    let bss_size = bss_range.end - bss_range.start;
    unsafe {
        ptr::write_bytes(
            ptr::with_exposed_provenance_mut::<u8>(bss_range.start),
            0,
            bss_size,
        );
    }
}

unsafe extern "C" fn primary_cpu_entry(cpuid: usize, dtb_pa: usize) -> *mut u8 {
    unsafe {
        init_bss();
    }
    let cpuid = Cpuid::from_raw(cpuid);
    let stack = crate::primary_cpu_entry(cpuid, dtb_pa)
        .with_whatever_context(|_| format!("failed to initialize primary CPU#{cpuid}"))
        .unwrap_or_else(|e: GenericError| error::report(e));
    let stack_top = stack.top();
    mem::forget(stack);
    ptr::with_exposed_provenance_mut(stack_top)
}

unsafe extern "C" fn primary_cpu_reentry() -> ! {
    crate::main(true)
        .whatever_context("kernel main thread panicked")
        .unwrap_or_else(|e: GenericError| error::report(e));
    unreachable!();
}

static CPU_STARTED: AtomicBool = AtomicBool::new(false);

pub unsafe fn start_secondary_cpu(cpuid: Cpuid) {
    CPU_STARTED.store(false, Ordering::Release);
    unsafe {
        imp::start_secondary_cpu(cpuid);
    }
    while !CPU_STARTED.load(Ordering::Acquire) {
        // Wait for the secondary CPU to start
        hint::spin_loop();
    }
}

unsafe extern "C" fn secondary_cpu_entry(cpuid: usize) -> *mut u8 {
    let cpuid = Cpuid::from_raw(cpuid);
    let stack = crate::secondary_cpu_entry(cpuid)
        .with_whatever_context(|_| format!("failed to initialize secondary CPU#{cpuid}"))
        .unwrap_or_else(|e: GenericError| error::report(e));
    let stack_top = stack.top();
    mem::forget(stack);
    ptr::with_exposed_provenance_mut(stack_top)
}

unsafe extern "C" fn secondary_cpu_reentry() -> ! {
    CPU_STARTED.store(true, Ordering::Release);

    crate::main(false)
        .whatever_context("kernel main thread panicked")
        .unwrap_or_else(|e: GenericError| error::report(e));
    unreachable!();
}
