#![no_std]

use core::fmt;

use snafu::GenerateImplicitData;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Location(&'static core::panic::Location<'static>);

impl Default for Location {
    #[track_caller]
    fn default() -> Self {
        Self(core::panic::Location::caller())
    }
}

impl GenerateImplicitData for Location {
    #[track_caller]
    fn generate() -> Self {
        Self::default()
    }
}

impl fmt::Debug for Location {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.0, f)
    }
}

impl fmt::Display for Location {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}
