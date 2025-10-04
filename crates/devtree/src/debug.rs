use core::fmt;

pub(crate) trait SliceDebug<T> {
    fn slice_debug(&self, max_len: usize) -> DebugSlice<'_, T>;
}

impl<T> SliceDebug<T> for [T] {
    fn slice_debug(&self, max_len: usize) -> DebugSlice<'_, T> {
        DebugSlice {
            data: self,
            max_len,
        }
    }
}

pub(crate) struct DebugSlice<'a, T> {
    data: &'a [T],
    max_len: usize,
}

impl<T> fmt::Debug for DebugSlice<'_, T>
where
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        struct More(usize);
        impl fmt::Debug for More {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "...({} more)", self.0)
            }
        }

        let mut dl = f.debug_list();
        dl.entries(self.data.iter().take(self.max_len));
        if self.data.len() > self.max_len {
            dl.entry(&More(self.data.len() - self.max_len));
        }
        dl.finish()
    }
}
