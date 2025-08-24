//! Utilities for parsing Devicetree.
//!
//! This crate provides utilities for handling Devicetree structures, as
//! described in the [Devicetree Specification]. It allows parsing, traversing,
//! and extracting information from Flattened Devicetree (FDT) blobs, including
//! memory reservation entries and structure entries.
//!
//! [Devicetree Specification]: https://devicetree-specification.readthedocs.io/en/stable/flattened-format.html

#![feature(substr_range)]
#![feature(error_generic_member_access)]
#![no_std]

extern crate alloc;

pub mod common;
pub mod flattened;
pub mod parsed;
