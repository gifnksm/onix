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
        ensure!(self.items.len() >= stack.len(), StackOverflowError);
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

#[cfg_attr(coverage_nightly, coverage(off))]
#[cfg(test)]
mod tests {
    extern crate alloc;

    use alloc::format;

    use super::*;

    #[test]
    fn test_new_stack_is_empty() {
        let mut buf = [0; 4];
        let stack = SliceNodeStack::new(&mut buf);
        assert_eq!(stack.len(), 0);
        assert!(stack.is_empty());
        assert_eq!(stack.as_slice(), &[]);
        assert!(stack.current().is_none());
    }

    #[test]
    fn test_push_and_pop() {
        let mut buf = [0; 2];
        let mut stack = SliceNodeStack::new(&mut buf);
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
        let mut buf = [0; 1];
        let mut stack = SliceNodeStack::new(&mut buf);

        stack.push(1).unwrap();
        assert!(stack.push(2).is_err());
    }

    #[test]
    fn test_clear() {
        let mut buf = [0; 3];
        let mut stack = SliceNodeStack::new(&mut buf);
        stack.push(1).unwrap();
        stack.push(2).unwrap();
        stack.clear();
        assert!(stack.is_empty());
        assert_eq!(stack.as_slice(), &[]);
        assert!(stack.current().is_none());
    }

    #[test]
    fn test_as_slice() {
        let mut buf = [0; 3];
        let mut stack = SliceNodeStack::new(&mut buf);
        stack.push(1).unwrap();
        stack.push(2).unwrap();
        assert_eq!(stack.as_slice(), &[1, 2]);
    }

    #[test]
    fn test_try_clone() {
        let mut buf = [0; 2];
        let mut stack = SliceNodeStack::new(&mut buf);
        stack.push(7).unwrap();
        assert!(stack.try_clone().is_none());
    }

    #[test]
    fn test_clone_from_stack() {
        let mut buf1 = [0; 3];
        let mut buf2 = [0; 3];
        let mut stack1 = SliceNodeStack::new(&mut buf1);
        let mut stack2 = SliceNodeStack::new(&mut buf2);

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
        let mut buf1 = [0; 2];
        let mut buf2 = [0; 1];
        let mut stack1 = SliceNodeStack::new(&mut buf1);
        let mut stack2 = SliceNodeStack::new(&mut buf2);

        stack1.push(1).unwrap();
        stack1.push(2).unwrap();

        let result = stack2.clone_from_stack(&stack1);
        assert!(result.is_err());
    }

    #[test]
    fn test_debug_fmt() {
        let mut buf = [1, 2, 3];
        let mut stack = SliceNodeStack::new(&mut buf);
        stack.push(10).unwrap();
        stack.push(20).unwrap();
        let s = format!("{stack:?}");
        assert_eq!(s, "[10, 20]");
    }
}
