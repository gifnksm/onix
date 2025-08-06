use core::arch::naked_asm;

use sbi::hart_state_management;

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
unsafe extern "C" fn entry(hartid: usize, dtb_pa: usize) -> ! {
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

pub fn start_secondary_cpu(hartid: usize) {
    unsafe {
        hart_state_management::start(hartid, secondary_cpu_entry as usize, 0).unwrap();
    }
}

#[unsafe(naked)]
unsafe extern "C" fn secondary_cpu_entry(hartid: usize, opaque: usize) -> ! {
    naked_asm!(
        "la sp, {boot_stack_top}",
        "call {secondary_cpu_entry}",
        "mv sp, a0",
        "j {secondary_cpu_reentry}",
        boot_stack_top = sym BOOT_STACK_TOP,
        secondary_cpu_entry = sym super::super::secondary_cpu_entry,
        secondary_cpu_reentry = sym super::super::secondary_cpu_reentry,
    )
}
