//! SBI Hart State Management Extension interface.
//!
//! This module provides functions to interact with the SBI Hart State
//! Management Extension, allowing the supervisor-mode software to request a
//! hart state change.

use platform_cast::CastInto as _;

use crate::SbiRet;

pub const EXTENSION_ID: usize = 0x48_53_4D; // 'HSM' in ASCII

/// Requests the SBI implementation to start executing the target hart in
/// supervisor-mode.
///
/// # Safety
///
/// This function is unsafe because it performs a raw SBI call with the provided
/// memory addresses. The caller must ensure that the `start_addr` is a valid
/// memory address and that the `opaque` value is properly initialized.
pub unsafe fn start(hartid: usize, start_addr: usize, opaque: usize) -> SbiRet {
    const FUNCTION_ID: usize = 0x0;
    unsafe { crate::ecall3(hartid, start_addr, opaque, EXTENSION_ID, FUNCTION_ID) }
}

/// Requests the SBI implementation to stop executing the calling hart in
/// supervisor mode and returns its ownership to the SBI implementation.
///
/// This call is not expected to return under normal conditions.
///
/// This function must be called with supervisor-mode interrupts disabled.
pub fn stop() -> SbiRet {
    const FUNCTION_ID: usize = 0x1;
    unsafe { crate::ecall0(EXTENSION_ID, FUNCTION_ID) }
}

/// The hart is physically powered-up and executing normally.
pub const HART_STATE_STARTED: isize = 0;
/// The hart is not executing in supervisor-mode or any lower privilege mode.
///
/// It is probably powered-down by the SBI implementation if the underlying
/// platform has a mechanism to physically power-down harts.
pub const HART_STATE_STOPPED: isize = 1;
/// The hart is transitioning to the `STARTED` state.
///
/// Some other hart has requested to start (or power-up) the hart from the
/// `STOPPED` state and the SBI implementation is still working to get the hart
/// in the `STARTED` state.
pub const HART_STATE_START_PENDING: isize = 2;
/// The hart is transitioning to the `STOPPED` state.
///
/// The hart has requested to stop (or power-down) itself from the `STARTED`
/// state and the SBI implementation is still working to get the hart in the
/// `STOPPED` state.
pub const HART_STATE_STOP_PENDING: isize = 3;
/// This hart is in a platform specific suspend (or low power) state.
pub const HART_STATE_SUSPENDED: isize = 4;
/// The hart is transitioning to a platform specific suspend state.
///
/// The hart has requested to put itself in a platform specific low power state
/// from the `STARTED` state and the SBI implementation is still working to get
/// the hart in the platform specific `SUSPENDED` state.
pub const HART_STATE_SUSPEND_PENDING: isize = 5;
/// The hart is resuming from a platform specific suspend state.
///
/// An interrupt or platform specific hardware event has caused the hart to
/// resume normal execution from the `SUSPENDED` state and the SBI
/// implementation is still working to get the hart in the `STARTED` state.
pub const HART_STATE_RESUME_PENDING: isize = 6;

/// Gets the current status (or HSM state id) of the given hart.
pub fn get_status(hartid: usize) -> SbiRet {
    const FUNCTION_ID: usize = 0x2;
    unsafe { crate::ecall1(hartid, EXTENSION_ID, FUNCTION_ID) }
}

/// Requests the SBI implementation to put the calling hart in a platform
/// specific suspend (or low power) state.
///
/// # Safety
///
/// This function is unsafe because it performs a raw SBI call with the provided
/// memory addresses. The caller must ensure that the `resume_addr` is a valid
/// memory address and that the `opaque` value is properly initialized.
pub unsafe fn suspend(suspend_type: u32, resume_addr: usize, opaque: usize) -> SbiRet {
    const FUNCTION_ID: usize = 0x3;
    unsafe {
        crate::ecall3(
            suspend_type.cast_into(),
            resume_addr,
            opaque,
            EXTENSION_ID,
            FUNCTION_ID,
        )
    }
}
