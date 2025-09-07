pub mod node;
pub mod property;

pub use bstr::BStr as ByteStr;
#[cfg(feature = "alloc")]
pub use bstr::BString as ByteString;
