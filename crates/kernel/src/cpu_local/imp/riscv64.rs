use core::arch::asm;

pub fn get_thread_pointer() -> *mut u8 {
    let tp: *mut u8;
    unsafe {
        asm!("mv {0}, tp", out(reg) tp);
    }
    tp
}

pub fn set_thread_pointer(tp: *mut u8) {
    unsafe {
        asm!("mv tp, {0}", in(reg) tp);
    }
}
