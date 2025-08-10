//! SBI RFENCE Extension interface.
//!
//! This module provides functions to interact with the SBI RFENCE Extension,
//! defining remote fence functions.

use sbi_sys::{SbiError, rfence};

pub const EXTENSION_ID: usize = 0x52_46_4E_43; // 'RFNC' in ASCII

/// Instructs remote harts to execute `FENCE.I` instruction.
pub fn remote_fence_i(hart_mask: usize, hart_mask_base: usize) -> Result<(), SbiError> {
    let ret = rfence::remote_fence_i(hart_mask, hart_mask_base);
    ret.into_result()?;
    Ok(())
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
) -> Result<(), SbiError> {
    let ret = rfence::remote_sfence_vma(hart_mask, hart_mask_base, start_addr, size);
    ret.into_result()?;
    Ok(())
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
) -> Result<(), SbiError> {
    let ret = rfence::remote_sfence_vma_asid(hart_mask, hart_mask_base, start_addr, size, asid);
    ret.into_result()?;
    Ok(())
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
) -> Result<(), SbiError> {
    let ret = rfence::remote_hfence_gvma_vmid(hart_mask, hart_mask_base, start_addr, size, vmid);
    ret.into_result()?;
    Ok(())
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
) -> Result<(), SbiError> {
    let ret = rfence::sbi_remote_hfence_gvma(hart_mask, hart_mask_base, start_addr, size);
    ret.into_result()?;
    Ok(())
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
) -> Result<(), SbiError> {
    let ret =
        rfence::sbi_remote_hfence_vvma_asid(hart_mask, hart_mask_base, start_addr, size, asid);
    ret.into_result()?;
    Ok(())
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
) -> Result<(), SbiError> {
    let ret = rfence::sbi_remote_hfence_vvma(hart_mask, hart_mask_base, start_addr, size);
    ret.into_result()?;
    Ok(())
}
