#![cfg_attr(
    feature = "unstable-provider-api",
    feature(error_generic_member_access)
)]
#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;

pub use devtree_derive::DeserializeNode;

pub use self::blob::Devicetree;

pub mod blob;
pub mod cursor;
pub mod de;
pub mod search;
pub mod types;
mod utils;
