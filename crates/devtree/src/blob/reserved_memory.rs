use core::ops::Range;

use dataview::Pod;
use endian::Be;
use platform_cast::CastFrom as _;

/// Represents a memory reservation entry in the DTB.
///
/// Each entry describes a reserved memory region with a start address and size.
#[repr(C)]
#[derive(Debug, Pod, Clone, Copy)]
pub struct ReserveEntry {
    /// The start address of the reserved memory region.
    pub address: Be<u64>,
    /// The size of the reserved memory region in bytes.
    pub size: Be<u64>,
}

impl ReserveEntry {
    #[must_use]
    pub(crate) fn is_terminator(&self) -> bool {
        self.address.read() == 0 && self.size.read() == 0
    }

    #[must_use]
    pub fn address_range(&self) -> Range<usize> {
        let start = usize::cast_from(self.address.read());
        let end = start.saturating_add(usize::cast_from(self.size.read()));
        start..end
    }
}
