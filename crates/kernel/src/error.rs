use ansi_term::{Color, WithFg};
pub use snafu_utils::GenericError;
use snafu_utils::Report;

#[track_caller]
pub fn report<E>(err: E) -> !
where
    E: core::error::Error,
{
    let panic_message = WithFg::new(Color::Red, "Critical error occurred");
    let report = Report::new(err);
    panic!("{panic_message}\n\n{report}");
}
