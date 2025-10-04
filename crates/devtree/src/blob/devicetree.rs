use core::{fmt, ptr, slice};

use dataview::DataView;

use super::error::ReadDevicetreeError;
use crate::{
    blob::{Header, ReserveEntry, error::ReadDevicetreeErrorKind},
    debug::SliceDebug as _,
    node_stack::{NodeStack, types::ArrayNodeStack},
    token_cursor::types::{BlobNodeHandle, BlobTokenCursor},
    tree_cursor::{error::ReadTreeError, types::StackBasedTreeCursor},
};

#[repr(transparent)]
pub struct Devicetree {
    blob: [u8],
}

impl fmt::Debug for Devicetree {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Devicetree")
            .field("header", &self.header())
            .field("memory_reservation_map", &self.memory_reservation_map())
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

        // SAFETY: Devicetree is #[repr(transparent)] over [u8]
        Ok(unsafe { (ptr::from_ref(bytes) as *const Self).as_ref().unwrap() })
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
