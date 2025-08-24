//! Flattened Devicetree (FDT) binary format structures and utilities.
//!
//! This module provides low-level types and functions for working with
//! FDT blobs as specified by the Devicetree specification. It handles
//! the binary format details including headers, memory reservations,
//! structure tokens, and property headers.
//!
//! # FDT Binary Format
//!
//! An FDT blob consists of several sections:
//!
//! 1. **Header**: Contains magic number, version info, and section offsets
//! 2. **Memory Reservation Block**: List of reserved memory regions
//! 3. **Structure Block**: Device tree hierarchy as binary tokens
//! 4. **Strings Block**: Null-terminated strings referenced by properties
//!
//! All multi-byte values in FDT are stored in big-endian format.
//!
//! # Alignment Requirements
//!
//! Different sections have specific alignment requirements:
//!
//! - Header: 8-byte aligned
//! - Memory reservations: 8-byte aligned
//! - Structure tokens: 4-byte aligned
//!
//! # Usage
//!
//! This module is primarily used internally by higher-level APIs,
//! but can be useful for low-level FDT manipulation or validation.

//! The structures and utilities for working with Flattened Device Tree (FDT)
//! blobs.
//!
//! It provides types and functions for parsing, validating, and handling
//! FDT headers, memory reservation entries, structure tokens, and property
//! headers, as specified by the Devicetree specification. Error types for
//! header validation are also included.

use core::{ops::Range, str::Utf8Error};

use dataview::Pod;
use endian::Be;
use platform_cast::CastFrom as _;
use snafu::{Snafu, ensure};
use snafu_utils::Location;

const MAGIC: u32 = 0xd00d_feed;
const SPEC_VERSION: u32 = 17;
const HEADER_ALIGNMENT: usize = 8;
const MEM_RSVMAP_ALIGNMENT: usize = 8;
const STRUCTURE_ALIGNMENT: usize = 4;

/// Errors that can occur during validation of a Devicetree header.
///
/// These errors indicate problems such as invalid magic numbers, incompatible
/// versions, layout inconsistencies, or malformed strings within the FDT.
#[derive(Debug, Snafu)]
pub enum HeaderValidationError {
    #[snafu(display("invalid magic number: {magic}"))]
    #[snafu(provide(ref, priority, Location => location))]
    InvalidMagic {
        magic: u32,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display(
        "incompatible device tree version: version={version}, \
         last_comp_version={last_comp_version}"
    ))]
    #[snafu(provide(ref, priority, Location => location))]
    IncompatibleVersion {
        version: u32,
        last_comp_version: u32,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display(
        "invalid device tree layout: totalsize={totalsize:#x}, off_dt_struct={off_dt_struct:#x}, \
         off_dt_strings={off_dt_strings:#x}, off_mem_rsvmap={off_mem_rsvmap:#x}, \
         size_dt_strings={size_dt_strings:#x}, size_dt_struct={size_dt_struct:#x}"
    ))]
    #[snafu(provide(ref, priority, Location => location))]
    InvalidLayout {
        totalsize: u32,
        off_dt_struct: u32,
        off_dt_strings: u32,
        off_mem_rsvmap: u32,
        size_dt_strings: u32,
        size_dt_struct: u32,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("invalid token: {token:#x} at offset {offset}"))]
    #[snafu(provide(ref, priority, Location => location))]
    InvalidToken {
        token: u32,
        offset: usize,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("invalid string in structure block at offset {offset}"))]
    #[snafu(provide(ref, priority, Location => location))]
    InvalidStringInStructBlock {
        offset: usize,
        #[snafu(source)]
        source: Utf8Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("invalid string in strings block at offset {offset}"))]
    #[snafu(provide(ref, priority, Location => location))]
    InvalidStringInStringsBlock {
        offset: usize,
        #[snafu(source)]
        source: Utf8Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("missing prop header at offset {offset}"))]
    #[snafu(provide(ref, priority, Location => location))]
    MissingPropHeader {
        offset: usize,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("unexpected end of struct block at offset {offset}"))]
    #[snafu(provide(ref, priority, Location => location))]
    UnexpectedEndOfStructBlock {
        offset: usize,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("unexpected end of strings block at offset {offset}"))]
    #[snafu(provide(ref, priority, Location => location))]
    UnexpectedEndOfStringsBlock {
        offset: usize,
        #[snafu(implicit)]
        location: Location,
    },
}

/// Header for the devicetree.
///
/// This structure appears at the beginning of a FDT blob and describes
/// the layout and versioning of the FDT.
#[repr(C)]
#[derive(custom_debug_derive::Debug, Pod)]
pub struct Header {
    /// The value `0xd00dfeed` (big-endian)
    #[debug(format = "{:#x}")]
    pub magic: Be<u32>,
    /// The total size in bytes of the devicetree data structure.
    pub totalsize: Be<u32>,
    /// The offset in bytes of the structure block from the beginning of the
    /// header.
    pub off_dt_struct: Be<u32>,
    /// The offset in bytes of the strings block from the beginning of the
    /// header.
    pub off_dt_strings: Be<u32>,
    /// The offset in bytes of the memory reservation block from the beginning
    /// of the header.
    pub off_mem_rsvmap: Be<u32>,
    /// The version of the devicetree data structure.
    pub version: Be<u32>,
    /// The lowest version of the devicetree data structure with which the
    /// version used is backwards compatible.
    pub last_comp_version: Be<u32>,
    /// The physical ID of the system’s boot CPU.
    pub boot_cpuid_phys: Be<u32>,
    /// The length in bytes of the strings block section of the devicetree blob.
    pub size_dt_strings: Be<u32>,
    /// The length in bytes of the structure block section of the devicetree
    /// blob.
    pub size_dt_struct: Be<u32>,
}

impl Header {
    /// Checks if the given address is valid for a FDT header.
    ///
    /// The address must be aligned to the required header alignment.
    #[must_use]
    pub fn is_valid_header_addr(addr: usize) -> bool {
        // The header must be aligned to HEADER_ALIGNMENT.
        addr.is_multiple_of(HEADER_ALIGNMENT)
    }

    /// Validates the FDT header fields for correctness and layout.
    ///
    /// Performs comprehensive validation including:
    ///
    /// - Magic number verification
    /// - Version compatibility checking
    /// - Layout consistency validation
    /// - Alignment requirement verification
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If the header is valid and safe to use
    /// * `Err(HeaderValidationError)` - If any validation check fails
    ///
    /// # Validation Checks
    ///
    /// - Magic number must be 0xd00dfeed
    /// - Version must be compatible with specification version 17
    /// - All section offsets must be within totalsize
    /// - Sections must not overlap
    /// - Alignment requirements must be met
    pub fn validate(&self) -> Result<(), HeaderValidationError> {
        let magic = self.magic.read();
        ensure!(magic == MAGIC, InvalidMagicSnafu { magic });

        let version = self.version.read();
        let last_comp_version = self.last_comp_version.read();
        ensure!(
            version == SPEC_VERSION || last_comp_version <= SPEC_VERSION,
            IncompatibleVersionSnafu {
                version,
                last_comp_version,
            }
        );

        let totalsize = self.totalsize.read();
        let off_dt_struct = self.off_dt_struct.read();
        let off_dt_strings = self.off_dt_strings.read();
        let off_mem_rsvmap = self.off_mem_rsvmap.read();
        let size_dt_strings = self.size_dt_strings.read();
        let size_dt_struct = self.size_dt_struct.read();
        #[expect(clippy::missing_panics_doc)]
        let size_header = u32::try_from(size_of::<Self>()).unwrap();

        let is_layout_valid = size_header <= off_mem_rsvmap
            && off_mem_rsvmap <= off_dt_struct
            && off_dt_struct + size_dt_struct <= off_dt_strings
            && off_dt_strings + size_dt_strings <= totalsize
            && usize::cast_from(off_mem_rsvmap).is_multiple_of(MEM_RSVMAP_ALIGNMENT)
            && usize::cast_from(off_dt_struct).is_multiple_of(STRUCTURE_ALIGNMENT);
        ensure!(
            is_layout_valid,
            InvalidLayoutSnafu {
                totalsize,
                off_dt_struct,
                off_dt_strings,
                off_mem_rsvmap,
                size_dt_strings,
                size_dt_struct,
            }
        );

        Ok(())
    }
}

const _: () = assert!(HEADER_ALIGNMENT.is_multiple_of(align_of::<Header>()));

/// Represents a memory reservation entry in the FDT.
///
/// Each entry describes a reserved memory region with a start address and size.
#[repr(C)]
#[derive(custom_debug_derive::Debug, Pod, Clone, Copy)]
pub struct ReserveEntry {
    /// The start address of the reserved memory region.
    #[debug(format = "{:#x}")]
    pub address: Be<u64>,
    /// The size of the reserved memory region in bytes.
    #[debug(format = "{:#x}")]
    pub size: Be<u64>,
}

const _: () = assert!(MEM_RSVMAP_ALIGNMENT.is_multiple_of(align_of::<ReserveEntry>()));

impl ReserveEntry {
    /// Returns `true` if this entry is the terminator (address and size are
    /// zero).
    ///
    /// The terminator marks the end of the memory reservation block.
    #[must_use]
    pub fn is_terminator(&self) -> bool {
        self.address.read() == 0 && self.size.read() == 0
    }

    /// Returns the memory range as a Rust Range.
    ///
    /// Converts the big-endian address and size values into a range
    /// that can be used with Rust's range operations.
    ///
    /// # Returns
    ///
    /// A range from `address` to `address + size`, with overflow protection.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// for entry in devicetree.mem_rsvmap() {
    ///     if !entry.is_terminator() {
    ///         let range = entry.range();
    ///         println!("Reserved: {:#x}..{:#x}", range.start, range.end);
    ///     }
    /// }
    /// ```
    #[must_use]
    pub fn range(&self) -> Range<usize> {
        let start = usize::cast_from(self.address.read());
        let end = start.saturating_add(usize::cast_from(self.size.read()));
        start..end
    }
}

/// Structure token used in the FDT structure block.
///
/// Each token marks the beginning or end of a node, a property, or other
/// structure elements in the device tree.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Pod)]
pub struct StructToken(pub Be<u32>);

const _: () = assert!(STRUCTURE_ALIGNMENT.is_multiple_of(align_of::<StructToken>()));

impl StructToken {
    /// Token value indicating the beginning of a node.
    pub const BEGIN_NODE: u32 = 0x0000_0001;
    /// Token value indicating the end of a node.
    pub const END_NODE: u32 = 0x0000_0002;
    /// Token value indicating a property.
    pub const PROP: u32 = 0x0000_0003;
    /// Token value indicating a no-operation.
    pub const NOP: u32 = 0x0000_0004;
    /// Token value indicating the end of the structure block.
    pub const END: u32 = 0x0000_0009;
}

/// Header for a property in the FDT structure block.
///
/// Contains the length of the property value and the offset of the property
/// name in the strings block.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod)]
pub struct PropHeader {
    pub len: Be<u32>,
    pub nameoff: Be<u32>,
}
