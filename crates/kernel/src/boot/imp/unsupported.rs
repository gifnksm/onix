#[unsafe(link_section = ".text.entry")]
#[unsafe(export_name = "entry")]
unsafe extern "C" fn entry() {
    // to suppress warnings
    let _ = super::super::boot_hart_start;
    let _ = unsafe { super::super::BOOT_STACK_TOP };
    unimplemented!("unsupported architecture");
}
