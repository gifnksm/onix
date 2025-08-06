use core::arch::naked_asm;

use super::super::BOOT_STACK_TOP;

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
        "la sp, {boot_stack_top}",
        "call {primary_cpu_entry}",
        "mv sp, a0",
        "j {primary_cpu_reentry}",
        boot_stack_top = sym BOOT_STACK_TOP,
        primary_cpu_entry = sym super::super::primary_cpu_entry,
        primary_cpu_reentry = sym super::super::primary_cpu_reentry,
    );
}
