use core::fmt;

#[derive(Debug)]
pub struct StackOverflowError;

impl fmt::Display for StackOverflowError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "stack overflow")
    }
}

impl core::error::Error for StackOverflowError {}
