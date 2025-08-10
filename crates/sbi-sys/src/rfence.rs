//! SBI RFENCE Extension interface.
//!
//! This module provides functions to interact with the SBI RFENCE Extension,
//! defining remote fence functions.

use crate::SbiRet;

pub const EXTENSION_ID: usize = 0x52_46_4E_43; // 'RFNC' in ASCII

/// Instructs remote harts to execute `FENCE.I` instruction.
pub fn remote_fence_i(hart_mask: usize, hart_mask_base: usize) -> SbiRet {
    const FUNCTION_ID: usize = 0x0;
    unsafe { crate::ecall2(hart_mask, hart_mask_base, EXTENSION_ID, FUNCTION_ID) }
}

/// Instructs remote harts to execute one or more `FENCE.VMA` instructions.
///
/// This covers the range of virtual addresses between `start_addr` and
/// `start_addr + size`.
pub fn remote_sfence_vma(
    hart_mask: usize,
    hart_mask_base: usize,
    start_addr: usize,
    size: usize,
) -> SbiRet {
    const FUNCTION_ID: usize = 0x1;
    unsafe {
        crate::ecall4(
            hart_mask,
            hart_mask_base,
            start_addr,
            size,
            EXTENSION_ID,
            FUNCTION_ID,
        )
    }
}

/// Instructs remote harts to execute one or more `FENCE.VMA` instructions.
///
/// This covers the range of virtual addresses between `start_addr` and
/// `start_addr + size`. This covers only the given `ASID`.
pub fn remote_sfence_vma_asid(
    hart_mask: usize,
    hart_mask_base: usize,
    start_addr: usize,
    size: usize,
    asid: usize,
) -> SbiRet {
    const FUNCTION_ID: usize = 0x2;
    unsafe {
        crate::ecall5(
            hart_mask,
            hart_mask_base,
            start_addr,
            size,
            asid,
            EXTENSION_ID,
            FUNCTION_ID,
        )
    }
}

/// Instruct the remote harts to execute one or more `HFNECE.GVMA` instructions.
///
/// This coverts the range of guest physical addresses between `start_addr` and
/// `start_addr + size` only for the given `VMID`. This function call is only
/// valid for harts implementing hypervisor extension.
pub fn remote_hfence_gvma_vmid(
    hart_mask: usize,
    hart_mask_base: usize,
    start_addr: usize,
    size: usize,
    vmid: usize,
) -> SbiRet {
    const FUNCTION_ID: usize = 0x3;
    unsafe {
        crate::ecall5(
            hart_mask,
            hart_mask_base,
            start_addr,
            size,
            vmid,
            EXTENSION_ID,
            FUNCTION_ID,
        )
    }
}

/// Instruct the remote harts to execute one or more `HFENCE.GVMA` instructions.
///
/// This coverts the range of guest physical addresses between `start_addr` and
/// `start_addr + size` for all the guets. This function call is only valid for
/// harts implementing hypervisor extension.
pub fn sbi_remote_hfence_gvma(
    hart_mask: usize,
    hart_mask_base: usize,
    start_addr: usize,
    size: usize,
) -> SbiRet {
    const FUNCTION_ID: usize = 0x4;
    unsafe {
        crate::ecall4(
            hart_mask,
            hart_mask_base,
            start_addr,
            size,
            EXTENSION_ID,
            FUNCTION_ID,
        )
    }
}

/// Instructs the remote harts to execute one or more `HFENCE.VVMA`
/// instructions.
///
/// This coverts the range of guest virtual addresses between `start_addr` and
/// `start_addr + size` for the given `ASID` and current `VMID` (in `hgatp` CSR)
/// of calling hart. This function call is only valid for
/// harts implementing hypervisor extension.
pub fn sbi_remote_hfence_vvma_asid(
    hart_mask: usize,
    hart_mask_base: usize,
    start_addr: usize,
    size: usize,
    asid: usize,
) -> SbiRet {
    const FUNCTION_ID: usize = 0x5;
    unsafe {
        crate::ecall5(
            hart_mask,
            hart_mask_base,
            start_addr,
            size,
            asid,
            EXTENSION_ID,
            FUNCTION_ID,
        )
    }
}

/// Instructs the remote harts to execute one or more `HFENCE.VVMA`
/// instructions.
///
/// This coverts the range of guest virtual addresses between `start_addr` and
/// `start_addr + size` for current `VMID` (in `hgatp` CSR) of calling hart.
/// This function call is only valid for harts implementing hypervisor
/// extension.
pub fn sbi_remote_hfence_vvma(
    hart_mask: usize,
    hart_mask_base: usize,
    start_addr: usize,
    size: usize,
) -> SbiRet {
    const FUNCTION_ID: usize = 0x6;
    unsafe {
        crate::ecall4(
            hart_mask,
            hart_mask_base,
            start_addr,
            size,
            EXTENSION_ID,
            FUNCTION_ID,
        )
    }
}
