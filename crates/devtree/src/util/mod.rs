#[cfg(feature = "alloc")]
pub use self::aligned_buffer::*;

#[cfg(feature = "alloc")]
mod aligned_buffer;
