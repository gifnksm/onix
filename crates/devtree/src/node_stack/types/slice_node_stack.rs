use core::{fmt, mem};

use crate::node_stack::{NodeStack, error::StackOverflowError};

pub struct SliceNodeStack<'a, T> {
    items: &'a mut [T],
    len: usize,
}

impl<'a, T> SliceNodeStack<'a, T> {
    pub fn new(items: &'a mut [T]) -> Self {
        Self { items, len: 0 }
    }
}

impl<T> NodeStack<T> for SliceNodeStack<'_, T>
where
    T: Default,
{
    fn try_clone(&self) -> Option<Self>
    where
        T: Clone,
    {
        None
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
            return Err(StackOverflowError);
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
        if self.items.len() < stack.len() {
            return Err(StackOverflowError);
        }
        self.items[..stack.len()].clone_from_slice(stack.as_slice());
        self.len = stack.len();
        Ok(())
    }
}

impl<T> fmt::Debug for SliceNodeStack<'_, T>
where
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.items[..self.len], f)
    }
}
