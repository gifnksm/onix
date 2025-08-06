#[unsafe(link_section = ".text.entry")]
#[unsafe(export_name = "entry")]
unsafe extern "C" fn entry() {
    // to suppress warnings
    let _ = super::super::primary_cpu_entry;
    let _ = super::super::primary_cpu_reentry;
    let _ = unsafe { super::super::BOOT_STACK_TOP };
    unimplemented!("unsupported architecture");
}
