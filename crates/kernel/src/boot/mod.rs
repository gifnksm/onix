use core::{
    hint, ptr,
    sync::atomic::{AtomicBool, Ordering},
};

use crate::memory;

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
    crate::primary_cpu_entry(cpuid, dtb_pa)
}

unsafe extern "C" fn primary_cpu_reentry() -> ! {
    crate::main()
}

static CPU_STARTED: AtomicBool = AtomicBool::new(false);

pub fn start_secondary_cpu(cpuid: usize) {
    CPU_STARTED.store(false, Ordering::Release);
    imp::start_secondary_cpu(cpuid);
    while !CPU_STARTED.load(Ordering::Acquire) {
        // Wait for the secondary CPU to start
        hint::spin_loop();
    }
}

unsafe extern "C" fn secondary_cpu_entry(cpuid: usize) -> *mut u8 {
    crate::secondary_cpu_entry(cpuid)
}

unsafe extern "C" fn secondary_cpu_reentry() -> ! {
    CPU_STARTED.store(true, Ordering::Release);
    crate::main()
}
