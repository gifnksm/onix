use core::ops::Range;

/// The kinds of errors that can occur when reading a devicetree blob.
#[derive(Debug, derive_more::Display, derive_more::Error)]
#[non_exhaustive]
pub enum ReadDevicetreeErrorKind {
    #[display(
        "unaligned devicetree pointer given: address={address:#x}, \
         expected_alignment={expected_alignment}"
    )]
    UnalignedPointer {
        address: usize,
        expected_alignment: usize,
    },
    #[display("null devicetree pointer given")]
    NullPointer,
    #[display("insufficient length of devicetree blob bytes, needed={needed}, actual={actual}")]
    InsufficientBytes { needed: usize, actual: usize },
    #[display("invalid magic number: magic={magic:#x}")]
    InvalidMagic { magic: u32 },
    #[display("invalid total size: total_size={total_size}")]
    InvalidTotalSize { total_size: usize },
    #[display(
        "incompatible version: version={version}, \
         last_compatible_version={last_compatible_version}"
    )]
    IncompatibleVersion {
        version: u32,
        last_compatible_version: u32,
    },
    #[display(
        "unaligned block: block_name={block_name}, expected_alignment={block_alignment}, \
         block_offset={block_offset}, block_size={block_size}"
    )]
    UnalignedBlock {
        block_name: &'static str,
        block_alignment: usize,
        block_offset: u32,
        block_size: u32,
    },
    #[display(
        "block out of bounds: block_name={block_name}, block_offset={block_offset}, block_size={block_size}, \
         valid_range={}..{}", valid_range.start, valid_range.end,
    )]
    BlockOutOfBounds {
        block_name: &'static str,
        block_offset: u32,
        block_size: u32,
        valid_range: Range<u32>,
    },
    #[display("unterminated memory reservation block")]
    UnterminatedMemRsvmap,
}

define_error!(
    /// The error type returned when reading a devicetree blob.
    pub struct ReadDevicetreeError {
        kind: ReadDevicetreeErrorKind,
    }
);
