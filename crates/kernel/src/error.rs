use alloc::{boxed::Box, string::String};

use ansi_term::{Color, WithFg};
use snafu::Snafu;
use snafu_utils::{Location, Report};

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

#[track_caller]
pub fn report<E>(err: E) -> !
where
    E: core::error::Error,
{
    let panic_message = WithFg::new(Color::Red, "Critical error occurred");
    let report = Report::new(err);
    panic!("{panic_message}\n\n{report}");
}
