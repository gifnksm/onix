pub use self::{address::*, basic::*, numeric::*, status::*, string::*};

macro_rules! forward_fmt_impls {
    ($ty:path, $($traits:path),* $(,)?) => {
        $(impl $traits for $ty {
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                <_ as $traits>::fmt(&self.value(), f)
            }
        })*
    }
}

macro_rules! forward_numeric_fmt_impls {
    ($ty:path) => {
        forward_fmt_impls!(
            $ty,
            ::core::fmt::Display,
            ::core::fmt::Binary,
            ::core::fmt::Octal,
            ::core::fmt::LowerHex,
            ::core::fmt::UpperHex
        );
    };
}

mod address;
mod basic;
mod numeric;
mod status;
mod string;

pub mod iter {
    pub use super::{address::iter::*, basic::iter::*, string::iter::*};
}
