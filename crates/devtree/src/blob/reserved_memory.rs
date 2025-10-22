use core::ops::Range;

use dataview::Pod;
use endian::Be;

/// Represents a memory reservation entry in the DTB.
///
/// Each entry describes a reserved memory region with a start address and size.
#[repr(C)]
#[derive(Debug, Pod, Clone, Copy, PartialEq, Eq)]
pub struct ReserveEntry {
    /// The start address of the reserved memory region.
    address: Be<u64>,
    /// The size of the reserved memory region in bytes.
    size: Be<u64>,
}

impl ReserveEntry {
    #[must_use]
    pub fn terminator() -> Self {
        Self::new(0, 0)
    }

    #[must_use]
    pub fn is_terminator(&self) -> bool {
        self.address.read() == 0 && self.size.read() == 0
    }

    #[must_use]
    pub fn new(address: u64, size: u64) -> Self {
        Self {
            address: Be::new(&address),
            size: Be::new(&size),
        }
    }

    #[must_use]
    pub fn address(&self) -> u64 {
        self.address.read()
    }

    #[must_use]
    pub fn size(&self) -> u64 {
        self.size.read()
    }

    #[must_use]
    pub fn address_range(&self) -> Range<u64> {
        let start = self.address.read();
        let end = start.saturating_add(self.size.read());
        start..end
    }
}

#[cfg_attr(coverage_nightly, coverage(off))]
#[cfg(test)]
mod tests {
    use dataview::PodMethods as _;

    use super::*;

    #[test]
    fn test_reserve_entry_bigendian() {
        let entry = ReserveEntry::new(0x_1234_5678_90ab_cdef, 0x_fedc_ba09_8765_4321);
        assert_eq!(entry.address(), 0x_1234_5678_90ab_cdef);
        assert_eq!(entry.size(), 0x_fedc_ba09_8765_4321);
        assert_eq!(
            entry.as_bytes(),
            &[
                0x12, 0x34, 0x56, 0x78, 0x90, 0xab, 0xcd, 0xef, // address
                0xfe, 0xdc, 0xba, 0x09, 0x87, 0x65, 0x43, 0x21, // size
            ]
        );
    }

    #[test]
    fn test_reserve_entry_zero() {
        let entry = ReserveEntry::new(0, 0);
        assert_eq!(entry.address(), 0);
        assert_eq!(entry.size(), 0);
        assert!(entry.is_terminator());
        assert_eq!(entry.address_range(), 0..0);
    }

    #[test]
    fn test_reserve_entry_nonzero() {
        let entry = ReserveEntry::new(0x1000, 0x2000);
        assert_eq!(entry.address(), 0x1000);
        assert_eq!(entry.size(), 0x2000);
        assert!(!entry.is_terminator());
        assert_eq!(entry.address_range(), 0x1000..0x3000);
    }

    #[test]
    fn test_terminator_method() {
        let entry = ReserveEntry::terminator();
        assert_eq!(entry, ReserveEntry::new(0, 0));
        assert!(entry.is_terminator());
    }

    #[test]
    fn test_address_range_saturating_add() {
        let entry = ReserveEntry::new(u64::MAX - 1, 10);
        // Should saturate at u64::MAX
        assert_eq!(entry.address_range(), (u64::MAX - 1)..u64::MAX);
    }

    #[test]
    fn test_equality_and_clone() {
        let entry1 = ReserveEntry::new(0xDEAD_BEEF, 0x1000);
        let entry2 = entry1.clone();
        assert_eq!(entry1, entry2);
        assert_eq!(entry1.address(), entry2.address());
        assert_eq!(entry1.size(), entry2.size());
    }
}
