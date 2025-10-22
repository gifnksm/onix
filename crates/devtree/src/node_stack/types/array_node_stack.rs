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

#[cfg_attr(coverage_nightly, coverage(off))]
#[cfg(test)]
mod tests {
    extern crate alloc;

    use alloc::format;

    use super::*;

    #[test]
    fn test_default_and_new() {
        let stack: ArrayNodeStack<i32, 4> = ArrayNodeStack::default();
        assert_eq!(stack.len(), 0);

        let stack2: ArrayNodeStack<i32, 4> = ArrayNodeStack::new();
        assert_eq!(stack2.len(), 0);
    }

    #[test]
    fn test_push_and_pop() {
        let mut stack: ArrayNodeStack<i32, 2> = ArrayNodeStack::new();
        assert!(stack.current().is_none());

        stack.push(1).unwrap();
        assert_eq!(stack.current(), Some(&1));
        stack.push(2).unwrap();
        assert_eq!(stack.current(), Some(&2));
        assert_eq!(stack.len(), 2);
        assert_eq!(stack.as_slice(), &[1, 2]);
        assert_eq!(stack.current(), Some(&2));

        assert_eq!(stack.pop(), Some(2));
        assert_eq!(stack.pop(), Some(1));
        assert_eq!(stack.pop(), None);
        assert_eq!(stack.len(), 0);
    }

    #[test]
    fn test_push_overflow() {
        let mut stack: ArrayNodeStack<i32, 1> = ArrayNodeStack::new();
        stack.push(1).unwrap();
        assert!(stack.push(2).is_err());
    }

    #[test]
    fn test_clear() {
        let mut stack: ArrayNodeStack<i32, 3> = ArrayNodeStack::new();
        stack.push(1).unwrap();
        stack.push(2).unwrap();
        stack.clear();
        assert!(stack.is_empty());
        assert_eq!(stack.as_slice(), &[]);
        assert!(stack.current().is_none());
    }

    #[test]
    fn test_as_slice() {
        let mut stack: ArrayNodeStack<i32, 3> = ArrayNodeStack::new();
        stack.push(1).unwrap();
        stack.push(2).unwrap();
        assert_eq!(stack.as_slice(), &[1, 2]);
    }

    #[test]
    fn test_try_clone() {
        let mut stack: ArrayNodeStack<i32, 2> = ArrayNodeStack::new();
        stack.push(7).unwrap();
        let cloned = stack.try_clone().unwrap();
        assert_eq!(cloned.as_slice(), &[7]);
    }

    #[test]
    fn test_clone_from_stack() {
        let mut stack1: ArrayNodeStack<i32, 3> = ArrayNodeStack::new();
        let mut stack2: ArrayNodeStack<i32, 3> = ArrayNodeStack::new();

        stack1.push(10).unwrap();
        stack1.push(20).unwrap();

        stack2.push(1).unwrap();
        stack2.push(2).unwrap();
        stack2.push(3).unwrap();

        stack2.clone_from_stack(&stack1).unwrap();
        assert_eq!(stack2.as_slice(), &[10, 20]);
        assert_eq!(stack2.len(), 2);
    }

    #[test]
    fn test_clone_from_stack_overflow() {
        let mut stack1: ArrayNodeStack<i32, 2> = ArrayNodeStack::new();
        let mut stack2: ArrayNodeStack<i32, 1> = ArrayNodeStack::new();

        stack1.push(1).unwrap();
        stack1.push(2).unwrap();

        let result = stack2.clone_from_stack(&stack1);
        assert!(result.is_err());
    }

    #[test]
    fn test_debug_fmt() {
        let mut stack: ArrayNodeStack<i32, 2> = ArrayNodeStack::new();
        stack.push(10).unwrap();
        stack.push(20).unwrap();
        let s = format!("{stack:?}");
        assert_eq!(s, "[10, 20]");
    }
}
