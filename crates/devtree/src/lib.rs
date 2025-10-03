#![cfg_attr(
    feature = "unstable-provider-api",
    feature(error_generic_member_access)
)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![no_std]

pub use devtree_derive::DeserializeNode;

pub use self::blob::Devicetree;

#[doc(hidden)]
pub mod __private;
pub mod blob;
pub mod de;
pub mod node_stack;
mod polyfill;
pub mod token_cursor;
pub mod tree_cursor;
pub mod types;
