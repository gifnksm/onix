//! This crate provides low-level Rust bindings for the RISC-V Supervisor Binary
//! Interface (SBI).
//!
//! It is intended to be used as a low-level building block for implementing
//! safe and high-level SBI APIs in Rust. The library exposes the raw SBI call
//! interface and error codes, allowing higher-level abstractions to be built on
//! top.

#![no_std]

use core::{error::Error, fmt, num::NonZeroIsize};

pub mod debug_console;
pub mod hart_state_management;
pub mod rfence;

/// Represents an SBI error code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SbiError(Option<NonZeroIsize>);

impl SbiError {
    /// Completed successfully.
    pub const SUCCESS: Self = Self(None);

    /// Failed.
    pub const FAILED: Self = Self(NonZeroIsize::new(-1));

    /// Not supported.
    pub const NOT_SUPPORTED: Self = Self(NonZeroIsize::new(-2));

    /// Invalid parameter(s).
    pub const INVALID_PARAM: Self = Self(NonZeroIsize::new(-3));

    /// Denied or not allowed.
    pub const DENIED: Self = Self(NonZeroIsize::new(-4));

    /// Invalid address(s).
    pub const INVALID_ADDRESS: Self = Self(NonZeroIsize::new(-5));

    /// Already available.
    pub const ALREADY_AVAILABLE: Self = Self(NonZeroIsize::new(-6));

    /// Already started.
    pub const ALREADY_STARTED: Self = Self(NonZeroIsize::new(-7));

    /// Already stopped.
    pub const ALREADY_STOPPED: Self = Self(NonZeroIsize::new(-8));

    /// Shared memory not available.
    pub const NO_SHMEM: Self = Self(NonZeroIsize::new(-9));
}

impl fmt::Display for SbiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::SUCCESS => write!(f, "completed successfully"),
            Self::FAILED => write!(f, "failed"),
            Self::NOT_SUPPORTED => write!(f, "not supported"),
            Self::INVALID_PARAM => write!(f, "invalid parameter(s)"),
            Self::DENIED => write!(f, "denied or not allowed"),
            Self::INVALID_ADDRESS => write!(f, "invalid address(s)"),
            Self::ALREADY_AVAILABLE => write!(f, "already available"),
            Self::ALREADY_STARTED => write!(f, "already started"),
            Self::ALREADY_STOPPED => write!(f, "already stopped"),
            Self::NO_SHMEM => write!(f, "shared memory not available"),
            Self(Some(code)) => write!(f, "unknown error ({code})"),
        }
    }
}

impl Error for SbiError {}

/// The return value of an SBI call.
///
/// Contains both the error code and the return value from the SBI call.
#[repr(C)]
#[must_use]
pub struct SbiRet {
    /// SBI error code (0 for success, negative for errors).
    pub error: isize,
    /// SBI return value.
    pub value: isize,
}

impl From<SbiRet> for Result<isize, SbiError> {
    fn from(ret: SbiRet) -> Self {
        if ret.error == 0 {
            Ok(ret.value)
        } else {
            Err(SbiError(NonZeroIsize::new(ret.error)))
        }
    }
}

impl SbiRet {
    /// Converts this `SbiRet` into a `Result`.
    ///
    /// Returns `Ok(value)` if `error` is 0, otherwise returns `Err(SbiError)`.
    pub fn into_result(self) -> Result<isize, SbiError> {
        Result::from(self)
    }
}

/// Performs an SBI call with 0 arguments.
///
/// # Safety
///
/// This function is unsafe because it performs a raw SBI call, which may have
/// side effects or cause undefined behavior if used incorrectly.
#[inline]
#[cfg_attr(
    not(any(target_arch = "riscv32", target_arch = "riscv64")),
    expect(unused_variables)
)]
pub unsafe fn ecall0(extension_id: usize, function_id: usize) -> SbiRet {
    match () {
        #[cfg(any(target_arch = "riscv32", target_arch = "riscv64"))]
        () => {
            let error;
            let value;
            unsafe {
                core::arch::asm!(
                    "ecall",
                    lateout("a0") error,
                    lateout("a1") value,
                    in("a6") function_id,
                    in("a7") extension_id,
                );
            }

            SbiRet { error, value }
        }
        #[cfg(not(any(target_arch = "riscv32", target_arch = "riscv64")))]
        () => unimplemented!(),
    }
}

/// Performs an SBI call with 1 argument.
///
/// # Safety
///
/// This function is unsafe because it performs a raw SBI call, which may have
/// side effects or cause undefined behavior if used incorrectly.
#[inline]
#[cfg_attr(
    not(any(target_arch = "riscv32", target_arch = "riscv64")),
    expect(unused_variables)
)]
pub unsafe fn ecall1(arg0: usize, extension_id: usize, function_id: usize) -> SbiRet {
    match () {
        #[cfg(any(target_arch = "riscv32", target_arch = "riscv64"))]
        () => {
            let error;
            let value;

            unsafe {
                core::arch::asm!(
                    "ecall",
                    inlateout("a0") arg0 => error,
                    lateout("a1") value,
                    in("a6") function_id,
                    in("a7") extension_id,
                );
            }

            SbiRet { error, value }
        }
        #[cfg(not(any(target_arch = "riscv32", target_arch = "riscv64")))]
        () => unimplemented!(),
    }
}

/// Performs an SBI call with 2 arguments.
///
/// # Safety
///
/// This function is unsafe because it performs a raw SBI call, which may have
/// side effects or cause undefined behavior if used incorrectly.
#[inline]
#[cfg_attr(
    not(any(target_arch = "riscv32", target_arch = "riscv64")),
    expect(unused_variables)
)]
pub unsafe fn ecall2(arg0: usize, arg1: usize, extension_id: usize, function_id: usize) -> SbiRet {
    match () {
        #[cfg(any(target_arch = "riscv32", target_arch = "riscv64"))]
        () => {
            let error;
            let value;

            unsafe {
                core::arch::asm!(
                    "ecall",
                    inlateout("a0") arg0 => error,
                    inlateout("a1") arg1 => value,
                    in("a6") function_id,
                    in("a7") extension_id,
                );
            }

            SbiRet { error, value }
        }
        #[cfg(not(any(target_arch = "riscv32", target_arch = "riscv64")))]
        () => unimplemented!(),
    }
}

/// Performs an SBI call with 3 arguments.
///
/// # Safety
///
/// This function is unsafe because it performs a raw SBI call, which may have
/// side effects or cause undefined behavior if used incorrectly.
#[inline]
#[cfg_attr(
    not(any(target_arch = "riscv32", target_arch = "riscv64")),
    expect(unused_variables)
)]
pub unsafe fn ecall3(
    arg0: usize,
    arg1: usize,
    arg2: usize,
    extension_id: usize,
    function_id: usize,
) -> SbiRet {
    match () {
        #[cfg(any(target_arch = "riscv32", target_arch = "riscv64"))]
        () => {
            let error;
            let value;

            unsafe {
                core::arch::asm!(
                    "ecall",
                    inlateout("a0") arg0 => error,
                    inlateout("a1") arg1 => value,
                    in("a2") arg2,
                    in("a6") function_id,
                    in("a7") extension_id,
                );
            }

            SbiRet { error, value }
        }
        #[cfg(not(any(target_arch = "riscv32", target_arch = "riscv64")))]
        () => unimplemented!(),
    }
}

/// Performs an SBI call with 4 arguments.
///
/// # Safety
///
/// This function is unsafe because it performs a raw SBI call, which may have
/// side effects or cause undefined behavior if used incorrectly.
#[inline]
#[cfg_attr(
    not(any(target_arch = "riscv32", target_arch = "riscv64")),
    expect(unused_variables)
)]
pub unsafe fn ecall4(
    arg0: usize,
    arg1: usize,
    arg2: usize,
    arg3: usize,
    extension_id: usize,
    function_id: usize,
) -> SbiRet {
    match () {
        #[cfg(any(target_arch = "riscv32", target_arch = "riscv64"))]
        () => {
            let error;
            let value;

            unsafe {
                core::arch::asm!(
                    "ecall",
                    inlateout("a0") arg0 => error,
                    inlateout("a1") arg1 => value,
                    in("a2") arg2,
                    in("a3") arg3,
                    in("a6") function_id,
                    in("a7") extension_id,
                );
            }

            SbiRet { error, value }
        }
        #[cfg(not(any(target_arch = "riscv32", target_arch = "riscv64")))]
        () => unimplemented!(),
    }
}

/// Performs an SBI call with 5 arguments.
///
/// # Safety
///
/// This function is unsafe because it performs a raw SBI call, which may have
/// side effects or cause undefined behavior if used incorrectly.
#[inline]
#[cfg_attr(
    not(any(target_arch = "riscv32", target_arch = "riscv64")),
    expect(unused_variables)
)]
pub unsafe fn ecall5(
    arg0: usize,
    arg1: usize,
    arg2: usize,
    arg3: usize,
    arg4: usize,
    extension_id: usize,
    function_id: usize,
) -> SbiRet {
    match () {
        #[cfg(any(target_arch = "riscv32", target_arch = "riscv64"))]
        () => {
            let error;
            let value;

            unsafe {
                core::arch::asm!(
                    "ecall",
                    inlateout("a0") arg0 => error,
                    inlateout("a1") arg1 => value,
                    in("a2") arg2,
                    in("a3") arg3,
                    in("a4") arg4,
                    in("a6") function_id,
                    in("a7") extension_id,
                );
            }

            SbiRet { error, value }
        }
        #[cfg(not(any(target_arch = "riscv32", target_arch = "riscv64")))]
        () => unimplemented!(),
    }
}

/// Performs an SBI call with 6 arguments.
///
/// # Safety
///
/// This function is unsafe because it performs a raw SBI call, which may have
/// side effects or cause undefined behavior if used incorrectly.
#[inline]
#[cfg_attr(
    not(any(target_arch = "riscv32", target_arch = "riscv64")),
    expect(unused_variables)
)]
#[expect(clippy::too_many_arguments)]
pub unsafe fn ecall6(
    arg0: usize,
    arg1: usize,
    arg2: usize,
    arg3: usize,
    arg4: usize,
    arg5: usize,
    extension_id: usize,
    function_id: usize,
) -> SbiRet {
    match () {
        #[cfg(any(target_arch = "riscv32", target_arch = "riscv64"))]
        () => {
            let error;
            let value;

            unsafe {
                core::arch::asm!(
                    "ecall",
                    inlateout("a0") arg0 => error,
                    inlateout("a1") arg1 => value,
                    in("a2") arg2,
                    in("a3") arg3,
                    in("a4") arg4,
                    in("a5") arg5,
                    in("a6") function_id,
                    in("a7") extension_id,
                );
            }

            SbiRet { error, value }
        }
        #[cfg(not(any(target_arch = "riscv32", target_arch = "riscv64")))]
        () => unimplemented!(),
    }
}
