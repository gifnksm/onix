#[derive(Debug, derive_more::Display, derive_more::Error)]
#[display("stack overflow")]
pub struct StackOverflowError;
