//! Utilities for parsing Flattened Devicetree (FDT).
//!
//! This module provides utilities for handling Flattened Devicetree (FDT)
//! structures, as described in the [Devicetree Specification].
//! It allows parsing, traversing, and extracting information from FDT blobs,
//! including memory reservation entries and structure entries.
//!
//! [Devicetree Specification]: https://devicetree-specification.readthedocs.io/en/stable/flattened-format.html

use alloc::{
    collections::btree_map::BTreeMap,
    sync::{Arc, Weak},
};
use core::{ptr, slice};

use dataview::DataView;
use platform_cast::CastFrom as _;
use snafu::{ResultExt as _, Snafu, ensure};
use snafu_utils::Location;

use self::{
    layout::{Header, HeaderValidationError, ReserveEntry},
    node::{Node, ParseStructError},
    struct_lexer::StructLexer,
};
use crate::parsed;

pub mod layout;
pub mod node;
pub mod struct_lexer;

/// Errors that can occur while tokenizing a Flattened Devicetree (FDT).
///
/// This enum represents all possible error conditions encountered during
/// tokenizing or interpreting a FDT blob, such as invalid alignment, magic
/// number, version incompatibility, malformed layout, or invalid strings.
#[derive(Debug, Snafu)]
#[snafu(module)]
pub enum CreateError {
    #[snafu(display("invalid aligned address: {addr:#x}"))]
    #[snafu(provide(ref, priority, Location => location))]
    InvalidAddressAlignment {
        addr: usize,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("invalid device tree header"))]
    #[snafu(provide(ref, priority, Location => location))]
    InvalidHeader {
        #[snafu(source)]
        source: HeaderValidationError,
        #[snafu(implicit)]
        location: Location,
    },
}

/// Represents a Flattened Devicetree (FDT) structure.
///
/// This struct holds references to each section of a FDT (structure block,
/// strings block, memory reservation entries). It allows safe traversal and
/// extraction of information from a FDT blob without copying the data.
#[derive(custom_debug_derive::Debug, Clone, Copy)]
pub struct Devicetree<'fdt> {
    header: &'fdt Header,
    #[debug(skip)]
    struct_block: &'fdt DataView,
    #[debug(skip)]
    string_block: &'fdt DataView,
    #[debug(skip)]
    mem_rsvmap: &'fdt [ReserveEntry],
}

impl Devicetree<'static> {
    /// Creates a devicetree from the given physical address.
    ///
    /// # Safety
    ///
    /// The caller must ensure that `addr` points to a valid FDT in memory.
    ///
    /// Returns an error if the FDT is invalid or the address is not aligned.
    pub unsafe fn from_addr(addr: usize) -> Result<Self, CreateError> {
        #[cfg_attr(not(test), expect(clippy::wildcard_imports))]
        use self::create_error::*;

        ensure!(
            Header::is_valid_header_addr(addr),
            InvalidAddressAlignmentSnafu { addr }
        );

        let ptr = ptr::with_exposed_provenance::<u8>(addr);
        let header_bytes = unsafe { slice::from_raw_parts(ptr, size_of::<Header>()) };
        let header = DataView::from(header_bytes).get::<Header>(0);

        header.validate().context(InvalidHeaderSnafu)?;

        let totalsize = usize::cast_from(header.totalsize.read());
        let off_dt_struct = header.off_dt_struct.read();
        let off_dt_strings = header.off_dt_strings.read();
        let off_mem_rsvmap = header.off_mem_rsvmap.read();
        let size_dt_strings = header.size_dt_strings.read();
        let size_dt_struct = header.size_dt_struct.read();

        let dtb_bytes = unsafe { slice::from_raw_parts(ptr, totalsize) };
        let dtb = DataView::from(dtb_bytes);

        let struct_start = usize::cast_from(off_dt_struct);
        let struct_len = usize::cast_from(size_dt_struct);
        let struct_block = &dtb[struct_start..][..struct_len];

        let string_start = usize::cast_from(off_dt_strings);
        let string_len = usize::cast_from(size_dt_strings);
        let string_block = &dtb[string_start..][..string_len];

        let mem_rsvmap_start = usize::cast_from(off_mem_rsvmap);
        let mem_rsvmap_end = usize::cast_from(off_dt_struct);
        let mem_rsvmap_len = (mem_rsvmap_end - mem_rsvmap_start) / size_of::<ReserveEntry>();
        let mem_rsvmap = dtb.slice::<ReserveEntry>(mem_rsvmap_start, mem_rsvmap_len);
        let mem_rsvmap = mem_rsvmap
            .iter()
            .position(ReserveEntry::is_terminator)
            .map_or(mem_rsvmap, |end| &mem_rsvmap[..end]);

        Ok(Self {
            header,
            struct_block,
            string_block,
            mem_rsvmap,
        })
    }
}

impl<'fdt> Devicetree<'fdt> {
    #[must_use]
    pub fn as_bytes(&self) -> &'fdt [u8] {
        let data = ptr::from_ref(self.header).cast();
        let len = self.size();
        unsafe { slice::from_raw_parts(data, len) }
    }

    /// Returns the header of the devicetree.
    #[must_use]
    pub fn header(&self) -> &'fdt Header {
        self.header
    }

    #[must_use]
    pub fn size(&self) -> usize {
        usize::cast_from(self.header.totalsize.read())
    }

    /// Returns the memory reservation entries in the devicetree.
    #[must_use]
    pub fn mem_rsvmap(&self) -> &'fdt [ReserveEntry] {
        self.mem_rsvmap
    }

    /// Returns an iterator over the structure elements of the devicetree.
    #[must_use]
    pub fn struct_lexer<'tree>(&'tree self) -> StructLexer<'fdt, 'tree> {
        StructLexer::new(self)
    }

    pub fn root_node<'tree>(&'tree self) -> Result<Node<'fdt, 'tree>, ParseStructError> {
        Node::new(&self.struct_lexer())
    }

    pub fn parse(&self) -> Result<parsed::Devicetree, ParseStructError> {
        let mut phandle_map = BTreeMap::new();
        let root =
            self.root_node()?
                .parse(Weak::new(), self.string_block.as_ref(), &mut phandle_map)?;
        #[expect(clippy::missing_panics_doc)]
        let phandle_map = phandle_map
            .into_iter()
            .map(|(phandle, node)| (phandle, Weak::upgrade(&node).unwrap()))
            .collect();
        Ok(parsed::Devicetree::new(
            root,
            self.string_block.as_ref().into(),
            Arc::new(phandle_map),
            self.mem_rsvmap.to_vec(),
        ))
    }
}
