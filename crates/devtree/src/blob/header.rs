use core::ptr;

use dataview::{DataView, Pod};
use endian::Be;
use platform_cast::CastFrom as _;

use super::error::{ReadDevicetreeError, ReadDevicetreeErrorKind};
use crate::{
    blob::{ReserveEntry, struct_block::TokenType},
    polyfill,
};

pub const MAGIC: u32 = 0xd00d_feed;
pub const VERSION: u32 = 17;
pub const LAST_COMPATIBLE_VERSION: u32 = 16;
const HEADER_ALIGNMENT: usize = 8;
const MEM_RSVMAP_ALIGNMENT: usize = 8;
const STRUCTURE_ALIGNMENT: usize = 4;
const STRINGS_ALIGNMENT: usize = 1;

const _: () = {
    assert!(HEADER_ALIGNMENT == align_of::<Header>());
    assert!(MEM_RSVMAP_ALIGNMENT == align_of::<ReserveEntry>());
    assert!(STRUCTURE_ALIGNMENT == align_of::<TokenType>());
    assert!(HEADER_ALIGNMENT.is_multiple_of(MEM_RSVMAP_ALIGNMENT));
    assert!(HEADER_ALIGNMENT.is_multiple_of(STRUCTURE_ALIGNMENT));
    assert!(HEADER_ALIGNMENT.is_multiple_of(STRINGS_ALIGNMENT));
};

#[repr(C, align(8))]
#[derive(Debug, Pod)]
pub struct Header {
    magic: Be<u32>,
    total_size: Be<u32>,
    off_dt_struct: Be<u32>,
    off_dt_strings: Be<u32>,
    off_mem_rsvmap: Be<u32>,
    version: Be<u32>,
    last_compatible_version: Be<u32>,
    boot_cpuid_phys: Be<u32>,
    size_dt_strings: Be<u32>,
    size_dt_struct: Be<u32>,
}

impl Header {
    #[cfg(feature = "testing")]
    pub(crate) fn new_for_test(
        magic: u32,
        version: u32,
        last_compatible_version: u32,
        boot_cpuid_phys: u32,
        mem_rsvmap: &[ReserveEntry],
        struct_block: &[u8],
        strings_block: &[u8],
    ) -> Self {
        let mem_rsvmap_offset = u32::try_from(size_of::<Self>().next_multiple_of(8)).unwrap();
        let mem_rsvmap_size = u32::try_from(size_of_val(mem_rsvmap)).unwrap();
        let struct_block_offset = (mem_rsvmap_offset + mem_rsvmap_size).next_multiple_of(8);
        let struct_block_size = u32::try_from(struct_block.len()).unwrap();
        let strings_block_offset = (struct_block_offset + struct_block_size).next_multiple_of(8);
        let strings_block_size = u32::try_from(strings_block.len()).unwrap();
        let total_size = strings_block_offset + strings_block_size;

        Self {
            magic: Be::new(&magic),
            total_size: Be::new(&total_size),
            off_dt_struct: Be::new(&struct_block_offset),
            off_dt_strings: Be::new(&strings_block_offset),
            off_mem_rsvmap: Be::new(&mem_rsvmap_offset),
            version: Be::new(&version),
            last_compatible_version: Be::new(&last_compatible_version),
            boot_cpuid_phys: Be::new(&boot_cpuid_phys),
            size_dt_strings: Be::new(&strings_block_size),
            size_dt_struct: Be::new(&struct_block_size),
        }
    }

    #[must_use]
    pub fn magic(&self) -> u32 {
        self.magic.read()
    }

    #[must_use]
    pub fn total_size(&self) -> usize {
        usize::cast_from(self.total_size.read())
    }

    #[must_use]
    pub fn version(&self) -> u32 {
        self.version.read()
    }

    #[must_use]
    pub fn last_compatible_version(&self) -> u32 {
        self.last_compatible_version.read()
    }

    #[must_use]
    pub fn boot_cpuid_phys(&self) -> u32 {
        self.boot_cpuid_phys.read()
    }

    #[must_use]
    pub fn memory_reservation_block_offset(&self) -> usize {
        usize::cast_from(self.off_mem_rsvmap.read())
    }

    #[must_use]
    pub fn memory_reservation_block_max_len(&self) -> usize {
        (self.struct_block_offset() - self.memory_reservation_block_offset())
            / size_of::<ReserveEntry>()
    }

    #[must_use]
    pub fn struct_block_offset(&self) -> usize {
        usize::cast_from(self.off_dt_struct.read())
    }

    #[must_use]
    pub fn struct_block_size(&self) -> usize {
        usize::cast_from(self.size_dt_struct.read())
    }

    #[must_use]
    pub fn strings_block_offset(&self) -> usize {
        usize::cast_from(self.off_dt_strings.read())
    }

    #[must_use]
    pub fn strings_block_size(&self) -> usize {
        usize::cast_from(self.size_dt_strings.read())
    }
}

impl Header {
    /// Constructs a reference to a DTB header from a pointer.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the pointer is valid and points to a memory
    /// region that is at least the size of `Header`.
    pub unsafe fn from_ptr(ptr: *const u8) -> Result<&'static Self, ReadDevicetreeError> {
        let ptr: *const Self = polyfill::ptr_cast_aligned(ptr).ok_or_else(|| {
            ReadDevicetreeErrorKind::UnalignedPointer {
                address: ptr.addr(),
                expected_alignment: align_of::<Self>(),
            }
        })?;
        let header = unsafe { ptr.as_ref() }.ok_or(ReadDevicetreeErrorKind::NullPointer)?;
        header.validate()?;
        Ok(header)
    }

    /// Constructs a reference to a DTB header from a byte slice.
    pub fn from_bytes(bytes: &[u8]) -> Result<&Self, ReadDevicetreeError> {
        ensure!(
            bytes.len() >= size_of::<Self>(),
            ReadDevicetreeErrorKind::InsufficientBytes {
                needed: size_of::<Self>(),
                actual: bytes.len(),
            }
        );
        let data = DataView::from(bytes);
        let header =
            data.try_get::<Self>(0)
                .ok_or_else(|| ReadDevicetreeErrorKind::UnalignedPointer {
                    address: bytes.as_ptr().addr(),
                    expected_alignment: align_of::<Self>(),
                })?;
        header.validate()?;
        Ok(header)
    }

    fn validate(&self) -> Result<(), ReadDevicetreeError> {
        let magic = self.magic.read();
        ensure!(
            magic == MAGIC,
            ReadDevicetreeErrorKind::InvalidMagic { magic }
        );

        let ptr = ptr::from_ref(self).cast::<u8>();
        let total_size = usize::cast_from(self.total_size.read());
        ensure!(
            total_size >= size_of::<Self>() && ptr.addr().checked_add(total_size).is_some(),
            ReadDevicetreeErrorKind::InvalidTotalSize { total_size }
        );

        let version = self.version.read();
        let last_compatible_version = self.last_compatible_version.read();
        ensure!(
            last_compatible_version == LAST_COMPATIBLE_VERSION,
            ReadDevicetreeErrorKind::IncompatibleVersion {
                version,
                last_compatible_version,
            }
        );

        let total_size = self.total_size.read();
        let min_size_mem_rsvmap = u32::try_from(size_of::<ReserveEntry>()).unwrap();
        let mut prev_block_end = u32::try_from(size_of::<Self>()).unwrap();

        check_block_layout(
            "memory reservation block",
            MEM_RSVMAP_ALIGNMENT,
            self.off_mem_rsvmap.read(),
            min_size_mem_rsvmap,
            &mut prev_block_end,
            total_size,
        )?;
        check_block_layout(
            "structure block",
            STRUCTURE_ALIGNMENT,
            self.off_dt_struct.read(),
            self.size_dt_struct.read(),
            &mut prev_block_end,
            total_size,
        )?;
        check_block_layout(
            "strings block",
            STRINGS_ALIGNMENT,
            self.off_dt_strings.read(),
            self.size_dt_strings.read(),
            &mut prev_block_end,
            total_size,
        )?;
        Ok(())
    }
}

fn check_block_layout(
    block_name: &'static str,
    block_alignment: usize,
    block_offset: u32,
    block_size: u32,
    prev_block_end: &mut u32,
    whole_block_end: u32,
) -> Result<(), ReadDevicetreeError> {
    ensure!(
        usize::cast_from(block_offset).is_multiple_of(block_alignment),
        ReadDevicetreeErrorKind::UnalignedBlock {
            block_name,
            block_alignment,
            block_offset,
            block_size,
        }
    );
    ensure!(
        *prev_block_end <= block_offset
            && block_offset
                .checked_add(block_size)
                .is_some_and(|end| end <= whole_block_end),
        ReadDevicetreeErrorKind::BlockOutOfBounds {
            block_name,
            block_offset,
            block_size,
            valid_range: *prev_block_end..whole_block_end,
        }
    );
    *prev_block_end = block_offset.checked_add(block_size).unwrap();
    Ok(())
}

#[cfg_attr(coverage_nightly, coverage(off))]
#[cfg(test)]
mod tests {
    use core::ptr;

    use dataview::PodMethods as _;

    use super::*;
    use crate::blob::error::ReadDevicetreeErrorKind;

    fn header_to_ptr(header: &Header) -> *const u8 {
        ptr::from_ref(header).cast()
    }

    fn valid_header() -> Header {
        extern crate std;
        std::dbg!(Header::new_for_test(
            MAGIC,
            VERSION,
            LAST_COMPATIBLE_VERSION,
            0,
            &[ReserveEntry::terminator()],
            &[0; 32],
            &[0; 16],
        ))
    }

    #[test]
    fn test_header_bigendian() {
        let header = valid_header();
        let bytes = header.as_bytes();
        let (chunks, rest) = bytes.as_chunks::<4>();
        assert!(rest.is_empty());

        assert_eq!(u32::from_be_bytes(chunks[0]), header.magic.read());
        assert_eq!(u32::from_be_bytes(chunks[1]), header.total_size.read());
        assert_eq!(u32::from_be_bytes(chunks[2]), header.off_dt_struct.read());
        assert_eq!(u32::from_be_bytes(chunks[3]), header.off_dt_strings.read());
        assert_eq!(u32::from_be_bytes(chunks[4]), header.off_mem_rsvmap.read());
        assert_eq!(u32::from_be_bytes(chunks[5]), header.version.read());
        assert_eq!(
            u32::from_be_bytes(chunks[6]),
            header.last_compatible_version.read()
        );
        assert_eq!(u32::from_be_bytes(chunks[7]), header.boot_cpuid_phys.read());
        assert_eq!(u32::from_be_bytes(chunks[8]), header.size_dt_strings.read());
        assert_eq!(u32::from_be_bytes(chunks[9]), header.size_dt_struct.read());
    }

    #[test]
    fn test_valid_header() {
        let header = valid_header();
        let ptr = header_to_ptr(&header);
        let _header = unsafe { Header::from_ptr(ptr) }.unwrap();
    }

    #[test]
    fn test_null_pointer() {
        let ptr: *const u8 = core::ptr::null();
        let err = unsafe { Header::from_ptr(ptr) }.unwrap_err();
        assert!(
            matches!(err.kind(), ReadDevicetreeErrorKind::NullPointer),
            "err: {err:?}",
        );
    }

    #[test]
    fn test_unaligned_header() {
        let header = valid_header();
        let ptr = header_to_ptr(&header).map_addr(|addr| addr + 1);
        let err = unsafe { Header::from_ptr(ptr) }.unwrap_err();
        assert!(
            matches!(err.kind(), ReadDevicetreeErrorKind::UnalignedPointer { .. }),
            "err: {err:?}",
        );
    }

    #[test]
    fn test_invalid_magic() {
        let header = Header {
            magic: 0xdead_beef.into(),
            ..valid_header()
        };
        let ptr = header_to_ptr(&header);
        let err = unsafe { Header::from_ptr(ptr) }.unwrap_err();
        assert!(
            matches!(err.kind(), ReadDevicetreeErrorKind::InvalidMagic { .. }),
            "err: {err:?}",
        );
    }

    #[test]
    fn test_invalid_total_size() {
        let header = Header {
            total_size: 10.into(),
            ..valid_header()
        };
        let ptr = header_to_ptr(&header);
        let err = unsafe { Header::from_ptr(ptr) }.unwrap_err();
        assert!(
            matches!(err.kind(), ReadDevicetreeErrorKind::InvalidTotalSize { .. }),
            "err: {err:?}",
        );
    }

    #[test]
    fn test_incompatible_version() {
        let header = Header {
            version: (LAST_COMPATIBLE_VERSION + 1).into(),
            last_compatible_version: (LAST_COMPATIBLE_VERSION + 1).into(),
            ..valid_header()
        };
        let ptr = header_to_ptr(&header);
        let err = unsafe { Header::from_ptr(ptr) }.unwrap_err();
        assert!(
            matches!(
                err.kind(),
                ReadDevicetreeErrorKind::IncompatibleVersion { .. }
            ),
            "err: {err:?}",
        );
    }

    #[test]
    fn test_unaligned_mem_rsvmap_block() {
        let header = Header {
            off_mem_rsvmap: 41.into(), // Not aligned to MEM_RSVMAP_ALIGNMENT (8)
            ..valid_header()
        };
        let ptr = header_to_ptr(&header);
        let err = unsafe { Header::from_ptr(ptr) }.unwrap_err();
        assert!(
            matches!(err.kind(), ReadDevicetreeErrorKind::UnalignedBlock { block_name, .. } if *block_name == "memory reservation block"),
            "err: {err:?}",
        );
    }

    #[test]
    fn test_unaligned_struct_block() {
        let header = Header {
            off_dt_struct: 65.into(),
            ..valid_header()
        };
        let ptr = header_to_ptr(&header);
        let err = unsafe { Header::from_ptr(ptr) }.unwrap_err();
        assert!(
            matches!(err.kind(), ReadDevicetreeErrorKind::UnalignedBlock { block_name, .. } if *block_name == "structure block"),
            "err: {err:?}",
        );
    }

    #[test]
    fn test_mem_rsvmap_block_out_of_bounds() {
        let header = Header {
            off_mem_rsvmap: 120.into(),
            ..valid_header()
        };
        let ptr = header_to_ptr(&header);
        let err = unsafe { Header::from_ptr(ptr) }.unwrap_err();
        assert!(
            matches!(err.kind(), ReadDevicetreeErrorKind::BlockOutOfBounds { block_name, .. } if *block_name == "memory reservation block"),
            "err: {err:?}",
        );
    }

    #[test]
    fn test_struct_block_size_out_of_bounds() {
        let header = Header {
            off_dt_struct: 64.into(),
            size_dt_struct: 100.into(), // 64 + 100 = 164 > totalsize (104)
            ..valid_header()
        };
        assert_eq!(header.total_size(), 104);
        let ptr = header_to_ptr(&header);
        let err = unsafe { Header::from_ptr(ptr) }.unwrap_err();
        assert!(
            matches!(err.kind(), ReadDevicetreeErrorKind::BlockOutOfBounds { block_name, .. } if *block_name == "structure block"),
            "err: {err:?}",
        );
    }

    #[test]
    fn test_strings_block_size_out_of_bounds() {
        let header = Header {
            off_dt_strings: 96.into(),
            size_dt_strings: 40.into(), // 96 + 40 = 136 > totalsize (104)
            ..valid_header()
        };
        assert_eq!(header.total_size(), 104);
        let ptr = header_to_ptr(&header);
        let err = unsafe { Header::from_ptr(ptr) }.unwrap_err();
        assert!(
            matches!(err.kind(), ReadDevicetreeErrorKind::BlockOutOfBounds { block_name, .. } if *block_name == "strings block"),
            "err: {err:?}",
        );
    }

    #[test]
    fn test_from_bytes_insufficient() {
        let buf = [0_u8; 8];
        let err = Header::from_bytes(&buf).unwrap_err();
        assert!(
            matches!(
                err.kind(),
                ReadDevicetreeErrorKind::InsufficientBytes { .. }
            ),
            "err: {err:?}",
        );
    }

    #[test]
    fn test_from_bytes_unaligned() {
        #[repr(align(8))]
        struct Bytes([u8; size_of::<Header>() + HEADER_ALIGNMENT]);
        let buf = Bytes([0_u8; size_of::<Header>() + HEADER_ALIGNMENT]);
        assert!(
            buf.0.as_ptr().addr().is_multiple_of(HEADER_ALIGNMENT),
            "addr: {:#p}",
            buf.0.as_ptr()
        );
        let err = Header::from_bytes(&buf.0[1..]).unwrap_err();
        assert!(
            matches!(err.kind(), ReadDevicetreeErrorKind::UnalignedPointer { .. }),
            "err: {err:?}",
        );
    }

    #[test]
    fn test_from_bytes_valid() {
        let header = valid_header();
        let bytes = header.as_bytes();
        let h = Header::from_bytes(bytes).unwrap();
        assert_eq!(h.magic(), MAGIC);
    }

    #[test]
    fn test_overlapping_struct_block() {
        let overlap_off =
            u32::try_from(size_of::<Header>() + size_of::<ReserveEntry>()).unwrap() - 4;
        let header = Header {
            off_dt_struct: overlap_off.into(), // Starts before end of mem_rsvmap block
            ..valid_header()
        };
        let ptr = header_to_ptr(&header);
        let err = unsafe { Header::from_ptr(ptr) }.unwrap_err();
        assert!(
            matches!(err.kind(), ReadDevicetreeErrorKind::BlockOutOfBounds { block_name, .. } if *block_name == "structure block"),
            "err: {err:?}",
        );
    }

    #[test]
    fn test_zero_sized_struct_block() {
        let off_struct = u32::try_from(size_of::<Header>() + size_of::<ReserveEntry>()).unwrap();
        let header = Header {
            off_dt_struct: off_struct.into(),
            size_dt_struct: 0.into(),
            off_dt_strings: off_struct.into(), // Directly after zero-sized structure block
            size_dt_strings: 16.into(),
            ..valid_header()
        };
        let ptr = header_to_ptr(&header);
        let _ = unsafe { Header::from_ptr(ptr) }.unwrap();
    }

    #[test]
    fn test_zero_sized_strings_block() {
        let header = Header {
            size_dt_strings: 0.into(),
            ..valid_header()
        };
        let ptr = header_to_ptr(&header);
        let _ = unsafe { Header::from_ptr(ptr) }.unwrap();
    }

    #[test]
    fn test_accessor_methods() {
        let header = valid_header();
        assert_eq!(header.magic(), MAGIC);
        assert_eq!(header.total_size(), 104);
        assert_eq!(header.version(), VERSION);
        assert_eq!(header.last_compatible_version(), LAST_COMPATIBLE_VERSION);
        assert_eq!(header.boot_cpuid_phys(), 0);
    }
}
