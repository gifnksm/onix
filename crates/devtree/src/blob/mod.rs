pub use self::{
    devicetree::*, header::*, item::*, node::*, property::*, reserved_memory::*, struct_block::*,
};

#[cfg(feature = "alloc")]
mod alloc;
mod devicetree;
mod header;
mod item;
mod node;
mod property;
mod reserved_memory;
mod struct_block;

pub(crate) static UNIT_ADDRESS_SEPARATOR: u8 = b'@';
pub(crate) static PATH_SEPARATOR: u8 = b'/';
