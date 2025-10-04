use core::{array, fmt, mem};

use crate::node_stack::{NodeStack, error::StackOverflowError};

#[derive(Clone)]
pub struct ArrayNodeStack<T, const N: usize> {
    items: [T; N],
    len: usize,
}

impl<T, const N: usize> Default for ArrayNodeStack<T, N>
where
    T: Default,
{
    fn default() -> Self {
        Self {
            items: array::from_fn(|_| T::default()),
            len: 0,
        }
    }
}

impl<T, const N: usize> ArrayNodeStack<T, N> {
    #[must_use]
    pub fn new() -> Self
    where
        T: Default,
    {
        Self::default()
    }
}

impl<T, const N: usize> NodeStack<T> for ArrayNodeStack<T, N>
where
    T: Default,
{
    fn try_clone(&self) -> Option<Self>
    where
        T: Clone,
    {
        Some(self.clone())
    }

    fn as_slice(&self) -> &[T] {
        &self.items[..self.len]
    }

    fn len(&self) -> usize {
        self.len
    }

    fn current(&self) -> Option<&T> {
        self.items[..self.len].last()
    }

    fn push(&mut self, item: T) -> Result<(), StackOverflowError> {
        let Some(slot) = self.items.get_mut(self.len) else {
            bail!(StackOverflowError);
        };
        *slot = item;
        self.len += 1;
        Ok(())
    }

    fn pop(&mut self) -> Option<T> {
        let item = mem::take(self.items[..self.len].last_mut()?);
        self.len -= 1;
        Some(item)
    }

    fn clear(&mut self) {
        self.len = 0;
    }

    fn clone_from_stack<U>(&mut self, stack: &U) -> Result<(), StackOverflowError>
    where
        U: NodeStack<T>,
        T: Clone,
    {
        if N < stack.len() {
            bail!(StackOverflowError);
        }
        self.items[..stack.len()].clone_from_slice(stack.as_slice());
        self.len = stack.len();
        Ok(())
    }
}

impl<T, const N: usize> fmt::Debug for ArrayNodeStack<T, N>
where
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.items[..self.len], f)
    }
}
