use core::convert::Infallible;

use sbi_sys::{
    SbiError,
    hart_state_management::{
        self, HART_STATE_RESUME_PENDING, HART_STATE_START_PENDING, HART_STATE_STARTED,
        HART_STATE_STOP_PENDING, HART_STATE_STOPPED, HART_STATE_SUSPEND_PENDING,
        HART_STATE_SUSPENDED,
    },
};

/// Requests the SBI implementation to start executing the target hart in
/// supervisor-mode.
///
/// # Safety
///
/// This function is unsafe because it performs a raw SBI call with the provided
/// memory addresses. The caller must ensure that the `start_addr` is a valid
/// memory address and that the `opaque` value is properly initialized.
pub unsafe fn hart_start(hartid: usize, start_addr: usize, opaque: usize) -> Result<(), SbiError> {
    let ret = unsafe { hart_state_management::hart_start(hartid, start_addr, opaque) };
    ret.into_result()?;
    Ok(())
}

/// Requests the SBI implementation to stop executing the calling hart in
/// supervisor mode and returns its ownership to the SBI implementation.
///
/// This call is not expected to return under normal conditions.
///
/// This function must be called with supervisor-mode interrupts disabled.
pub fn hart_stop() -> Result<Infallible, SbiError> {
    let ret = hart_state_management::hart_stop();
    ret.into_result()?;
    unreachable!("SBI stop should not return under normal conditions");
}

/// Hart states as defined by the SBI specification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HartState {
    /// The hart is physically powered-up and executing normally.
    Started,
    /// The hart is not executing in supervisor-mode or any lower privilege
    /// mode.
    ///
    /// It is probably powered-down by the SBI implementation if the underlying
    /// platform has a mechanism to physically power-down harts.
    Stopped,
    /// The hart is transitioning to the `STARTED` state.
    ///
    /// Some other hart has requested to start (or power-up) the hart from the
    /// `STOPPED` state and the SBI implementation is still working to get the
    /// hart in the `STARTED` state.
    StartPending,
    /// The hart is transitioning to the `STOPPED` state.
    ///
    /// The hart has requested to stop (or power-down) itself from the `STARTED`
    /// state and the SBI implementation is still working to get the hart in the
    /// `STOPPED` state.
    StopPending,
    /// This hart is in a platform specific suspend (or low power) state.
    Suspended,
    /// The hart is transitioning to a platform specific suspend state.
    ///
    /// The hart has requested to put itself in a platform specific low power
    /// state from the `STARTED` state and the SBI implementation is still
    /// working to get the hart in the platform specific `SUSPENDED` state.
    SuspendPending,
    /// The hart is resuming from a platform specific suspend state.
    ///
    /// An interrupt or platform specific hardware event has caused the hart to
    /// resume normal execution from the `SUSPENDED` state and the SBI
    /// implementation is still working to get the hart in the `STARTED` state.
    ResumePending,
    /// The hart is in an unknown state.
    Unknown(isize),
}

impl HartState {
    fn from_sbi_state(state: isize) -> Self {
        match state {
            HART_STATE_STARTED => Self::Started,
            HART_STATE_STOPPED => Self::Stopped,
            HART_STATE_START_PENDING => Self::StartPending,
            HART_STATE_STOP_PENDING => Self::StopPending,
            HART_STATE_SUSPENDED => Self::Suspended,
            HART_STATE_SUSPEND_PENDING => Self::SuspendPending,
            HART_STATE_RESUME_PENDING => Self::ResumePending,
            _ => Self::Unknown(state),
        }
    }
}

/// Gets the current status (or HSM state id) of the given hart.
pub fn hart_get_status(hartid: usize) -> Result<HartState, SbiError> {
    let ret = hart_state_management::hart_get_status(hartid);
    let state = ret.into_result()?;
    Ok(HartState::from_sbi_state(state))
}

/// Requests the SBI implementation to put the calling hart in a platform
/// specific suspend (or low power) state.
///
/// # Safety
///
/// This function is unsafe because it performs a raw SBI call with the provided
/// memory addresses. The caller must ensure that the `resume_addr` is a valid
/// memory address and that the `opaque` value is properly initialized.
pub unsafe fn hart_suspend(
    suspend_type: u32,
    resume_addr: usize,
    opaque: usize,
) -> Result<(), SbiError> {
    let ret = unsafe { hart_state_management::hart_suspend(suspend_type, resume_addr, opaque) };
    ret.into_result()?;
    Ok(())
}
