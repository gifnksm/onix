pub fn sfence_vma(vaddr: usize, asid: usize) {
    cfg_if::cfg_if! {
        if #[cfg(any(target_arch = "riscv32", target_arch = "riscv64"))] {
            unsafe {
                core::arch::asm!("sfence.vma {}, {}", in(reg) vaddr, in(reg) asid);
            }
        } else {
            let _ = vaddr;
            let _ = asid;
            unimplemented!("unsupported architecture")
        }
    }
}

pub fn sfence_vma_asid_all(asid: usize) {
    cfg_if::cfg_if! {
        if #[cfg(any(target_arch = "riscv32", target_arch = "riscv64"))] {
            unsafe {
                core::arch::asm!("sfence.vma zero, {}", in(reg) asid);
            }
        } else {
            let _ = asid;
            unimplemented!("unsupported architecture")
        }
    }
}

pub fn sfence_vma_all() {
    cfg_if::cfg_if! {
        if #[cfg(any(target_arch = "riscv32", target_arch = "riscv64"))] {
            unsafe {
                core::arch::asm!("sfence.vma zero, zero");
            }
        } else {
            unimplemented!("unsupported architecture")
        }
    }
}
