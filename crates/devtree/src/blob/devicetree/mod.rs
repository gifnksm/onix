use core::{fmt, ptr, slice};

use dataview::DataView;

#[cfg(feature = "alloc")]
pub use self::alloc::*;
use super::error::ReadDevicetreeError;
use crate::{
    blob::{Header, ReserveEntry, error::ReadDevicetreeErrorKind},
    debug::SliceDebug as _,
    node_stack::{NodeStack, types::ArrayNodeStack},
    token_cursor::types::{BlobNodeHandle, BlobTokenCursor},
    tree_cursor::{error::ReadTreeError, types::StackBasedTreeCursor},
};

#[cfg(feature = "alloc")]
mod alloc;

pub static DEVICETREE_ALIGNMENT: usize = 8;

#[repr(transparent)]
pub struct Devicetree {
    blob: [u8],
}

impl fmt::Debug for Devicetree {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Devicetree")
            .field("header", &self.header())
            .field(
                "memory_reservation_map",
                &self.memory_reservation_map().slice_debug(16),
            )
            .field("struct_block", &self.struct_block().slice_debug(16))
            .field("strings_block", &self.strings_block().slice_debug(16))
            .finish()
    }
}

impl Devicetree {
    pub unsafe fn from_ptr(ptr: *const u8) -> Result<&'static Self, ReadDevicetreeError> {
        let header = unsafe { Header::from_ptr(ptr)? };
        let total_size = header.total_size();
        let bytes = unsafe { slice::from_raw_parts(ptr, total_size) };
        Self::from_bytes_internal(bytes, header)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<&Self, ReadDevicetreeError> {
        let header = Header::from_bytes(bytes)?;
        let total_size = header.total_size();
        ensure!(
            bytes.len() >= total_size,
            ReadDevicetreeErrorKind::InsufficientBytes {
                needed: total_size,
                actual: bytes.len()
            }
        );
        Self::from_bytes_internal(bytes, header)
    }

    fn from_bytes_internal<'blob>(
        bytes: &'blob [u8],
        header: &Header,
    ) -> Result<&'blob Self, ReadDevicetreeError> {
        assert!(bytes.len() >= header.total_size());
        assert_eq!(bytes.as_ptr().addr(), ptr::from_ref(header).addr());

        let data = DataView::from(bytes);
        let mem_rsvmap = data.slice::<ReserveEntry>(
            header.memory_reservation_block_offset(),
            header.memory_reservation_block_max_len(),
        );
        ensure!(
            mem_rsvmap.iter().any(ReserveEntry::is_terminator),
            ReadDevicetreeErrorKind::UnterminatedMemRsvmap
        );

        unsafe { Ok(Self::from_bytes_unchecked(bytes)) }
    }

    unsafe fn from_bytes_unchecked(bytes: &[u8]) -> &Self {
        assert!(bytes.as_ptr().addr().is_multiple_of(DEVICETREE_ALIGNMENT));
        // SAFETY: Devicetree is #[repr(transparent)] over [u8]
        unsafe { (ptr::from_ref(bytes) as *const Self).as_ref().unwrap() }
    }

    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.blob
    }

    #[must_use]
    fn as_dataview(&self) -> &DataView {
        DataView::from(&self.blob)
    }

    #[must_use]
    pub fn header(&self) -> &Header {
        self.as_dataview().get(0)
    }

    #[must_use]
    pub fn memory_reservation_map(&self) -> &[ReserveEntry] {
        let header = self.header();
        let entries = self.as_dataview().slice(
            header.memory_reservation_block_offset(),
            header.memory_reservation_block_max_len(),
        );
        let terminator_index = entries
            .iter()
            .position(ReserveEntry::is_terminator)
            .unwrap_or(entries.len());
        &entries[..terminator_index]
    }

    #[must_use]
    pub fn struct_block(&self) -> &[u8] {
        let header = self.header();
        &self.blob[header.struct_block_offset()..][..header.struct_block_size()]
    }

    #[must_use]
    pub fn strings_block(&self) -> &[u8] {
        let header = self.header();
        &self.blob[header.strings_block_offset()..][..header.strings_block_size()]
    }

    #[must_use]
    pub fn token_cursor(&self) -> BlobTokenCursor<'_> {
        BlobTokenCursor::new(self.struct_block(), self.strings_block())
    }

    pub fn tree_cursor(
        &self,
    ) -> Result<StackBasedTreeCursor<'_, BlobTokenCursor<'_>>, ReadTreeError> {
        StackBasedTreeCursor::new(self.token_cursor())
    }

    pub fn tree_cursor_with_stack_size<const STACK_SIZE: usize>(
        &self,
    ) -> Result<
        StackBasedTreeCursor<'_, BlobTokenCursor<'_>, ArrayNodeStack<BlobNodeHandle, STACK_SIZE>>,
        ReadTreeError,
    > {
        StackBasedTreeCursor::with_stack_size(self.token_cursor())
    }

    pub fn tree_cursor_with_node_stack<S>(
        &self,
        node_stack: S,
    ) -> Result<StackBasedTreeCursor<'_, BlobTokenCursor<'_>, S>, ReadTreeError>
    where
        S: NodeStack<BlobNodeHandle>,
    {
        StackBasedTreeCursor::with_node_stack(self.token_cursor(), node_stack)
    }
}

impl AsRef<[u8]> for Devicetree {
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}

#[cfg_attr(coverage_nightly, coverage(off))]
#[cfg(test)]
mod tests {
    extern crate alloc;

    use alloc::format;
    use core::iter;

    use super::*;
    use crate::testing::BlobBuilder;

    #[test]
    fn test_devicetree_from_bytes_and_as_bytes() {
        let buffer = BlobBuilder::new()
            .extend_mem_rsvmap_from_slice(&[ReserveEntry::terminator()])
            .build();
        let dt = Devicetree::from_bytes(&buffer).unwrap();
        assert_eq!(dt.as_bytes(), &buffer[..]);
    }

    #[test]
    fn test_devicetree_from_ptr() {
        let buffer = BlobBuilder::new()
            .extend_mem_rsvmap_from_slice(&[ReserveEntry::terminator()])
            .build();
        let dt = unsafe { Devicetree::from_ptr(buffer.as_ptr()).unwrap() };
        assert_eq!(dt.as_bytes(), &buffer[..]);
    }

    #[test]
    fn test_devicetree_header_and_blocks() {
        let buffer = BlobBuilder::new()
            .extend_mem_rsvmap([ReserveEntry::new(0, 10), ReserveEntry::terminator()])
            .extend_struct_block_from_slice(&[1, 2, 3, 4])
            .extend_strings_block_from_slice(b"test\0")
            .build();
        let dt = Devicetree::from_bytes(&buffer).unwrap();

        let header = dt.header();
        assert_eq!(header.total_size(), buffer.len());

        let mem_rsvmap = dt.memory_reservation_map();
        assert_eq!(mem_rsvmap, &[ReserveEntry::new(0, 10)]);

        let struct_block = dt.struct_block();
        assert_eq!(struct_block, &[1, 2, 3, 4]);

        let strings_block = dt.strings_block();
        assert_eq!(strings_block, b"test\0");
    }

    #[test]
    fn test_insufficient_bytes() {
        let buffer = BlobBuilder::new()
            .extend_mem_rsvmap_from_slice(&[ReserveEntry::new(0, 10), ReserveEntry::terminator()])
            .extend_struct_block_from_slice(&[1, 2, 3, 4])
            .extend_strings_block_from_slice(b"test\0")
            .build();
        let err = Devicetree::from_bytes(&buffer[..buffer.len() - 1]).unwrap_err();
        assert!(
            matches!(
                err.kind(),
                ReadDevicetreeErrorKind::InsufficientBytes { .. }
            ),
            "err: {err:?}",
        );
    }

    #[test]
    fn test_unterminated_mem_rsvmap() {
        let buffer = BlobBuilder::new()
            .extend_mem_rsvmap_from_slice(&[ReserveEntry::new(0, 10), ReserveEntry::new(100, 200)])
            .extend_struct_block_from_slice(&[1, 2, 3, 4])
            .extend_strings_block_from_slice(b"test\0")
            .build();
        let err = Devicetree::from_bytes(&buffer).unwrap_err();
        assert!(
            matches!(err.kind(), ReadDevicetreeErrorKind::UnterminatedMemRsvmap),
            "err: {err:?}",
        );
    }

    #[test]
    fn test_entry_after_terminator() {
        let buffer = BlobBuilder::new()
            .extend_mem_rsvmap_from_slice(&[
                ReserveEntry::terminator(),
                ReserveEntry::new(0, 10),
                ReserveEntry::new(100, 200),
            ])
            .extend_struct_block_from_slice(&[1, 2, 3, 4])
            .extend_strings_block_from_slice(b"test\0")
            .build();
        let dt = Devicetree::from_bytes(&buffer).unwrap();

        let header = dt.header();
        assert_eq!(header.total_size(), buffer.len());

        let mem_rsvmap = dt.memory_reservation_map();
        assert_eq!(mem_rsvmap, &[]);

        let struct_block = dt.struct_block();
        assert_eq!(struct_block, &[1, 2, 3, 4]);

        let strings_block = dt.strings_block();
        assert_eq!(strings_block, b"test\0");
    }

    #[test]
    fn test_as_ref_trait() {
        let buffer = BlobBuilder::new()
            .extend_mem_rsvmap_from_slice(&[ReserveEntry::terminator()])
            .build();
        let dt = Devicetree::from_bytes(&buffer).unwrap();
        let as_ref_bytes: &[u8] = dt.as_ref();
        assert_eq!(as_ref_bytes, &buffer[..]);
    }

    #[test]
    fn test_debug_not_too_long() {
        let buffer = BlobBuilder::new()
            .extend_mem_rsvmap(
                (0..)
                    .map(|n| ReserveEntry::new(n * 0x100, 0x50))
                    .take(128)
                    .chain(iter::once(ReserveEntry::terminator())),
            )
            .extend_struct_block((0..).flat_map(u32::to_be_bytes).take(128))
            .extend_strings_block((0..).map(|i| i % 128).take(128))
            .build();
        assert!(buffer.len() > 128);
        let dt = Devicetree::from_bytes(&buffer).unwrap();
        let debug = format!("{dt:#?}");
        assert!(debug.lines().count() < 128);
    }
}
