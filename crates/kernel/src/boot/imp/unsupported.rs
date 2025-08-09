use crate::cpu::Cpuid;

#[unsafe(link_section = ".text.entry")]
#[unsafe(export_name = "entry")]
unsafe extern "C" fn entry() {
    // to suppress warnings
    let _ = super::super::primary_cpu_entry;
    let _ = super::super::primary_cpu_reentry;
    let _ = unsafe { super::super::BOOT_STACK_TOP };
    unimplemented!("unsupported architecture");
}

pub unsafe fn start_secondary_cpu(_cpuid: Cpuid) {
    let _ = super::super::secondary_cpu_entry;
    let _ = super::super::secondary_cpu_reentry;
    let _ = unsafe { super::super::BOOT_STACK_TOP };
    unimplemented!("unsupported architecture");
}
