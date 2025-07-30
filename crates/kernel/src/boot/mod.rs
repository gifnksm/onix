use core::ptr;

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

unsafe extern "C" fn boot_hart_start(hartid: usize, dtb_pa: usize) -> ! {
    unsafe {
        init_bss();
    }
    crate::boot_hart_start(hartid, dtb_pa);
}
