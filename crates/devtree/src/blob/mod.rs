pub use self::{devicetree::*, header::*, item::*, node::*, property::*, reserved_memory::*};

#[cfg(feature = "alloc")]
mod alloc;
mod devicetree;
pub mod error;
mod header;
mod item;
mod node;
mod property;
mod reserved_memory;
pub mod struct_block;

pub(crate) static UNIT_ADDRESS_SEPARATOR: u8 = b'@';
pub(crate) static PATH_SEPARATOR: u8 = b'/';
