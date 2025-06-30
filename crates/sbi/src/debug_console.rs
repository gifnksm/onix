//! High-level interface for the SBI Debug Console Extension.
//!
//! This module provides safe Rust wrappers for reading from and writing to the
//! SBI debug console.

use sbi_sys::{SbiError, debug_console};

/// Writes bytes to the debug console from input memory.
pub fn write(bytes: &[u8]) -> Result<usize, SbiError> {
    let num_bytes = bytes.len();
    let base_addr_lo = bytes.as_ptr().addr();
    let base_addr_hi = 0; // Assuming no high address part is needed for this example
    let ret = unsafe { debug_console::write(num_bytes, base_addr_lo, base_addr_hi) };
    let written_bytes = ret.into_result()?;
    Ok(written_bytes.cast_unsigned())
}

/// Reads bytes from the debug console into output memory.
pub fn read(bytes: &mut [u8]) -> Result<usize, SbiError> {
    let num_bytes = bytes.len();
    let base_addr_lo = bytes.as_mut_ptr().addr();
    let base_addr_hi = 0; // Assuming no high address part is needed for this example
    let ret = unsafe { debug_console::read(num_bytes, base_addr_lo, base_addr_hi) };
    let read_bytes = ret.into_result()?;
    Ok(read_bytes.cast_unsigned())
}

/// Writes a single byte to the debug console.
pub fn write_byte(byte: u8) -> Result<(), SbiError> {
    let ret = debug_console::write_byte(byte);
    let _ = ret.into_result()?;
    Ok(())
}
