use core::{arch::naked_asm, ffi::c_void, mem::offset_of};

use dataview::Pod;

use crate::memory::kernel_space::KernelStack;

#[derive(Debug, Clone, Copy, Pod)]
#[repr(C)]
pub struct Context {
    ra: usize,
    // callee-saved registers
    sp: usize,
    s0: usize,
    s1: usize,
    s2: usize,
    s3: usize,
    s4: usize,
    s5: usize,
    s6: usize,
    s7: usize,
    s8: usize,
    s9: usize,
    s10: usize,
    s11: usize,
}

impl Context {
    pub(crate) fn new(
        stack: &KernelStack,
        entry: extern "C" fn(*mut c_void) -> !,
        arg: *mut c_void,
    ) -> Self {
        let mut context = Self::zeroed();
        context.ra = task_entry as usize;
        context.sp = stack.top();
        context.s1 = entry as usize;
        context.s2 = arg as usize;
        context
    }

    pub const fn zeroed() -> Self {
        Self {
            ra: 0,
            sp: 0,
            s0: 0,
            s1: 0,
            s2: 0,
            s3: 0,
            s4: 0,
            s5: 0,
            s6: 0,
            s7: 0,
            s8: 0,
            s9: 0,
            s10: 0,
            s11: 0,
        }
    }
}

/// Saves current registers in `old`, loads from `new`.
#[unsafe(naked)]
pub(super) unsafe extern "C" fn switch(old: *mut Context, new: *const Context) {
    naked_asm!(
        "sd ra, {c_ra}(a0)",
        "sd sp, {c_sp}(a0)",
        "sd s0, {c_s0}(a0)",
        "sd s1, {c_s1}(a0)",
        "sd s2, {c_s2}(a0)",
        "sd s3, {c_s3}(a0)",
        "sd s4, {c_s4}(a0)",
        "sd s5, {c_s5}(a0)",
        "sd s6, {c_s6}(a0)",
        "sd s7, {c_s7}(a0)",
        "sd s8, {c_s8}(a0)",
        "sd s9, {c_s9}(a0)",
        "sd s10, {c_s10}(a0)",
        "sd s11, {c_s11}(a0)",
        "ld ra, {c_ra}(a1)",
        "ld sp, {c_sp}(a1)",
        "ld s0, {c_s0}(a1)",
        "ld s1, {c_s1}(a1)",
        "ld s2, {c_s2}(a1)",
        "ld s3, {c_s3}(a1)",
        "ld s4, {c_s4}(a1)",
        "ld s5, {c_s5}(a1)",
        "ld s6, {c_s6}(a1)",
        "ld s7, {c_s7}(a1)",
        "ld s8, {c_s8}(a1)",
        "ld s9, {c_s9}(a1)",
        "ld s10, {c_s10}(a1)",
        "ld s11, {c_s11}(a1)",
        "ret",
        c_ra = const offset_of!(Context, ra),
        c_sp = const offset_of!(Context, sp),
        c_s0 = const offset_of!(Context, s0),
        c_s1 = const offset_of!(Context, s1),
        c_s2 = const offset_of!(Context, s2),
        c_s3 = const offset_of!(Context, s3),
        c_s4 = const offset_of!(Context, s4),
        c_s5 = const offset_of!(Context, s5),
        c_s6 = const offset_of!(Context, s6),
        c_s7 = const offset_of!(Context, s7),
        c_s8 = const offset_of!(Context, s8),
        c_s9 = const offset_of!(Context, s9),
        c_s10 = const offset_of!(Context, s10),
        c_s11 = const offset_of!(Context, s11),
    )
}

#[unsafe(naked)]
unsafe extern "C" fn task_entry() -> ! {
    naked_asm!(
        "mv a0, s1",
        "mv a1, s2",
        "j {task_entry_secondary}",
        task_entry_secondary = sym task_entry_secondary,
    );
}

extern "C" fn task_entry_secondary(entry: extern "C" fn(*mut c_void) -> !, arg: *mut c_void) -> ! {
    super::task_entry(entry, arg);
}
