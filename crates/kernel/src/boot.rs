use core::{arch::naked_asm, ptr};

use crate::memory;

unsafe extern "C" {
    #[link_name = "__onix_boot_stack_top"]
    static mut BOOT_STACK_TOP: u8;
}

// OpenSBI passes the information via the following registers of RISC-V CPU:
//
// * hartid via `a0` register
// * device tree blob address in memory via `a1` register
//
// https://github.com/riscv-software-src/opensbi/blob/master/docs/firmware/fw.md#opensbi-platform-firmwares
#[unsafe(naked)]
#[unsafe(link_section = ".text.entry")]
#[unsafe(export_name = "entry")]
unsafe extern "C" fn entry(hartid: usize, dtb_pa: usize) {
    naked_asm!(
        "la sp, {stack_top}",
        "j {boot_hart_start}",
        stack_top = sym BOOT_STACK_TOP,
        boot_hart_start = sym boot_hart_start
    );
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
