use core::arch::naked_asm;

use riscv::register::{
    sie,
    stvec::{self, Stvec, TrapMode},
};

pub fn apply() {
    unsafe {
        let mut sie = sie::read();
        sie.set_sext(true);
        sie.set_stimer(true);
        sie.set_ssoft(true);
        sie::write(sie);
    }

    let mut stvec = Stvec::from_bits(0);
    stvec.set_address(kernel_vec as usize);
    stvec.set_trap_mode(TrapMode::Direct);
    unsafe {
        stvec::write(stvec);
    }
}

#[unsafe(naked)]
extern "C" fn kernel_vec() {
    naked_asm!(
        // make room to save registers.
        "addi sp, sp, -8 * 20",

        // save caller-saved registers.
        "sd ra, 8 * 0(sp)",
        "sd sp, 8 * 1(sp)",
        "sd gp, 8 * 2(sp)",
        "sd tp, 8 * 3(sp)",
        "sd t0, 8 * 4(sp)",
        "sd t1, 8 * 5(sp)",
        "sd t2, 8 * 6(sp)",
        "sd a0, 8 * 7(sp)",
        "sd a1, 8 * 8(sp)",
        "sd a2, 8 * 9(sp)",
        "sd a3, 8 * 10(sp)",
        "sd a4, 8 * 11(sp)",
        "sd a5, 8 * 12(sp)",
        "sd a6, 8 * 13(sp)",
        "sd a7, 8 * 14(sp)",
        "sd t3, 8 * 15(sp)",
        "sd t4, 8 * 16(sp)",
        "sd t5, 8 * 17(sp)",
        "sd t6, 8 * 18(sp)",

        // call the Rust trap handler in trap.rs
        "call {trap_kernel}",

        // restore registers.
        "ld ra, 8 * 0(sp)",
        "ld sp, 8 * 1(sp)",
        "ld gp, 8 * 2(sp)",
        // not tp (contains hartid), in case we moved CPUs
        // "ld tp, 8 * 3(sp)",
        "ld t0, 8 * 4(sp)",
        "ld t1, 8 * 5(sp)",
        "ld t2, 8 * 6(sp)",
        "ld a0, 8 * 7(sp)",
        "ld a1, 8 * 8(sp)",
        "ld a2, 8 * 9(sp)",
        "ld a3, 8 * 10(sp)",
        "ld a4, 8 * 11(sp)",
        "ld a5, 8 * 12(sp)",
        "ld a6, 8 * 13(sp)",
        "ld a7, 8 * 14(sp)",
        "ld t3, 8 * 15(sp)",
        "ld t4, 8 * 16(sp)",
        "ld t5, 8 * 17(sp)",
        "ld t6, 8 * 18(sp)",

        "addi sp, sp, 8 * 20",

        // return to whatever we were doing in the kernel.
        "sret",
        trap_kernel = sym super::super::trap_kernel
    )
}
