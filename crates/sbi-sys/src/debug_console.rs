//! SBI Debug Console Extension interface.
//!
//! This module provides functions to interact with the SBI Debug Console
//! Extension, allowing reading from and writing to the debug console using SBI
//! calls.

use crate::SbiRet;

const EXTENSION_ID: usize = 0x4442_434E; // 'DBCN' in ASCII

/// Writes bytes to the debug console from input memory.
///
/// # Safety
///
/// This function is unsafe because it performs a raw SBI call with the provided
/// memory addresses. The caller must ensure that the memory region specified by
/// `base_addr_lo` and `base_addr_hi` is valid and accessible for reading at
/// least `num_bytes` bytes.
///
/// # Parameters
///
/// - `num_bytes`: Number of bytes to write.
/// - `base_addr_lo`: Lower `XLEN` bits of the base address of the input buffer.
/// - `base_addr_hi`: Upper `XLEN` bits of the base address of the input buffer.
pub unsafe fn write(num_bytes: usize, base_addr_lo: usize, base_addr_hi: usize) -> SbiRet {
    const FUNCTION_ID: usize = 0x0;
    unsafe {
        crate::ecall3(
            num_bytes,
            base_addr_lo,
            base_addr_hi,
            EXTENSION_ID,
            FUNCTION_ID,
        )
    }
}

/// Reads bytes from the debug console into output memory.
///
/// # Safety
///
/// This function is unsafe because it performs a raw SBI call with the provided
/// memory addresses. The caller must ensure that the memory region specified by
/// `base_addr_lo` and `base_addr_hi` is valid and accessible for writing at
/// least `num_bytes` bytes.
///
/// # Parameters
///
/// - `num_bytes`: Number of bytes to read.
/// - `base_addr_lo`: Lower `XLEN` bits of the base address of the output
///   buffer.
/// - `base_addr_hi`: Upper `XLEN` bits of the base address of the output
///   buffer.
pub unsafe fn read(num_bytes: usize, base_addr_lo: usize, base_addr_hi: usize) -> SbiRet {
    const FUNCTION_ID: usize = 0x1;
    unsafe {
        crate::ecall3(
            num_bytes,
            base_addr_lo,
            base_addr_hi,
            EXTENSION_ID,
            FUNCTION_ID,
        )
    }
}

/// Writes a single byte to the debug console.
pub fn write_byte(byte: u8) -> SbiRet {
    const FUNCTION_ID: usize = 0x2;
    unsafe { crate::ecall1(usize::from(byte), EXTENSION_ID, FUNCTION_ID) }
}
