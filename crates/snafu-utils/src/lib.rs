#![feature(error_generic_member_access)]
#![no_std]

extern crate alloc;

use alloc::{boxed::Box, string::String};
use core::{
    error::{self, Error},
    fmt,
};

use ansi_term::{Color, WithFg};
use snafu::{GenerateImplicitData, Snafu};

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

#[derive(Debug, Snafu)]
#[snafu(whatever, display("{message}"))]
#[snafu(provide(ref, priority, Location => location))]
#[snafu(provide(opt, ref, chain, dyn core::error::Error => source.as_deref()))]
pub struct GenericError {
    message: String,
    #[snafu(implicit)]
    location: Location,
    #[snafu(source(from(Box<dyn core::error::Error>, Some)))]
    #[snafu(provide(false))]
    source: Option<Box<dyn core::error::Error>>,
}

pub struct Report<E> {
    error: E,
}

impl<E> fmt::Debug for Report<E>
where
    E: Error,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl<E> fmt::Display for Report<E>
where
    E: Error,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Error: {}", WithFg::new(Color::Red, &self.error))?;
        if let Some(loc) = error::request_ref::<Location>(&self.error) {
            writeln!(f, "  at {}", WithFg::new(Color::DarkGray, loc))?;
        }
        let mut source = self.error.source();
        if source.is_some() {
            writeln!(f)?;
            writeln!(f, "Caused by:")?;
        }
        let mut index = 0;
        while let Some(s) = source {
            writeln!(f, "{index:4}: {}", WithFg::new(Color::Red, s))?;
            if let Some(loc) = error::request_ref::<Location>(s) {
                writeln!(f, "      at {}", WithFg::new(Color::DarkGray, loc))?;
            }
            source = s.source();
            index += 1;
        }
        Ok(())
    }
}

impl<E> Report<E> {
    pub fn new(error: E) -> Self {
        Self { error }
    }
}
