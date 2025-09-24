use super::error::StackOverflowError;

pub trait NodeStack<T> {
    #[must_use]
    fn try_clone(&self) -> Option<Self>
    where
        T: Clone,
        Self: Sized;
    fn as_slice(&self) -> &[T];
    #[must_use]
    fn len(&self) -> usize;
    #[must_use]
    fn current(&self) -> Option<&T>;

    fn push(&mut self, item: T) -> Result<(), StackOverflowError>;
    fn pop(&mut self) -> Option<T>;
    fn clear(&mut self);
    fn clone_from_stack<U>(&mut self, stack: &U) -> Result<(), StackOverflowError>
    where
        U: NodeStack<T>,
        T: Clone;

    #[must_use]
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
