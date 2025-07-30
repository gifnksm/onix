use core::arch::naked_asm;

use super::super::{BOOT_STACK_TOP, boot_hart_start};

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
