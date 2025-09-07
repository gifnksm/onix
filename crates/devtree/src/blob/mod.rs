use core::{fmt, ptr, slice};

use dataview::DataView;
use snafu::{ResultExt as _, Snafu, ensure};
use snafu_utils::Location;

pub use self::{header::*, node::*, property::*, reserved_memory::*, struct_block::*};

#[cfg(feature = "alloc")]
mod alloc;
mod header;
mod node;
mod property;
mod reserved_memory;
mod struct_block;

pub(crate) static UNIT_ADDRESS_SEPARATOR: u8 = b'@';
pub(crate) static PATH_SEPARATOR: u8 = b'/';

#[repr(transparent)]
pub struct Devicetree {
    blob: [u8],
}

impl fmt::Debug for Devicetree {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        struct FmtDataview<'a>(&'a DataView);
        impl fmt::Debug for FmtDataview<'_> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.debug_struct("FmtDataview")
                    .field("len", &self.0.len())
                    .finish()
            }
        }

        f.debug_struct("Devicetree")
            .field("header", &self.header())
            .field("memory_reservation_map", &self.memory_reservation_map())
            .field("struct_block", &FmtDataview(self.struct_block()))
            .field("strings_block", &FmtDataview(self.strings_block()))
            .finish()
    }
}

#[derive(Debug, Snafu)]
#[snafu(module)]
#[non_exhaustive]
pub enum ParseDevicetreeError {
    #[snafu(display("invalid DTB header"))]
    #[snafu(provide(ref, priority, Location => location))]
    InvalidHeader {
        #[snafu(implicit)]
        location: Location,
        #[snafu(source)]
        source: HeaderValidationError,
    },
    #[snafu(display("buffer has insufficient bytes for DTB: {actual} < {needed}"))]
    #[snafu(provide(ref, priority, Location => location))]
    InsufficientBytes {
        needed: usize,
        actual: usize,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("memory reservation block is unterminated"))]
    #[snafu(provide(ref, priority, Location => location))]
    UnterminatedMemRsvmap {
        #[snafu(implicit)]
        location: Location,
    },
}

impl Devicetree {
    pub unsafe fn from_ptr(ptr: *const u8) -> Result<&'static Self, ParseDevicetreeError> {
        #[cfg_attr(not(test), expect(clippy::wildcard_imports))]
        use self::parse_devicetree_error::*;

        let header = unsafe { Header::from_ptr(ptr).context(InvalidHeaderSnafu)? };
        let total_size = header.total_size();
        let bytes = unsafe { slice::from_raw_parts(ptr, total_size) };
        Self::from_bytes_internal(bytes, header)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<&Self, ParseDevicetreeError> {
        #[cfg_attr(not(test), expect(clippy::wildcard_imports))]
        use self::parse_devicetree_error::*;

        let header = Header::from_bytes(bytes).context(InvalidHeaderSnafu)?;
        let total_size = header.total_size();
        ensure!(
            bytes.len() >= total_size,
            InsufficientBytesSnafu {
                needed: total_size,
                actual: bytes.len()
            }
        );
        Self::from_bytes_internal(bytes, header)
    }

    fn from_bytes_internal<'blob>(
        bytes: &'blob [u8],
        header: &Header,
    ) -> Result<&'blob Self, ParseDevicetreeError> {
        #[cfg_attr(not(test), expect(clippy::wildcard_imports))]
        use self::parse_devicetree_error::*;

        assert!(bytes.len() >= header.total_size());
        assert_eq!(bytes.as_ptr().addr(), ptr::from_ref(header).addr());

        let data = DataView::from(bytes);
        let mem_rsvmap = data.slice::<ReserveEntry>(
            header.memory_reservation_block_offset(),
            header.memory_reservation_block_max_len(),
        );
        ensure!(
            mem_rsvmap.iter().any(ReserveEntry::is_terminator),
            UnterminatedMemRsvmapSnafu
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
        #[expect(clippy::missing_panics_doc)]
        let terminator_index = entries
            .iter()
            .position(ReserveEntry::is_terminator)
            .unwrap();
        &entries[..terminator_index]
    }

    #[must_use]
    pub fn struct_block(&self) -> &DataView {
        let header = self.header();
        &self.as_dataview()[header.struct_block_offset()..][..header.struct_block_size()]
    }

    #[must_use]
    pub fn strings_block(&self) -> &DataView {
        let header = self.header();
        &self.as_dataview()[header.strings_block_offset()..][..header.strings_block_size()]
    }
}
