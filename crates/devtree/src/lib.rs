#![cfg_attr(
    feature = "unstable-provider-api",
    feature(error_generic_member_access)
)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![cfg_attr(coverage_nightly, feature(coverage_attribute))]
#![no_std]

pub use devtree_derive::DeserializeNode;

pub use self::blob::Devicetree;

#[macro_use]
mod macros;

#[doc(hidden)]
pub mod __private;
pub mod blob;
mod bytes;
pub mod de;
mod debug;
pub mod model;
pub mod node_stack;
mod polyfill;
#[cfg(feature = "testing")]
pub mod testing;
pub mod token_cursor;
pub mod tree_cursor;
pub mod types;
pub mod util;
