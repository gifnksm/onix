use core::{fmt, ops::Range};

// alternatives to rust feature `slice_split_once`<https://github.com/rust-lang/rust/issues/112811>
pub(crate) fn slice_split_once<P, T>(s: &[T], pred: P) -> Option<(&[T], &[T])>
where
    P: FnMut(&T) -> bool,
{
    let pos = s.iter().position(pred)?;
    Some((&s[..pos], &s[pos + 1..]))
}

// alternatives to rust feature `slice_split_once`<https://github.com/rust-lang/rust/issues/112811>
pub(crate) fn slice_rsplit_once<P, T>(s: &[T], pred: P) -> Option<(&[T], &[T])>
where
    P: FnMut(&T) -> bool,
{
    let pos = s.iter().rposition(pred)?;
    Some((&s[..pos], &s[pos + 1..]))
}

// alternatives to rust feature `substr_range` <https://github.com/rust-lang/rust/issues/126769>
pub(crate) fn slice_subslice_range<T>(slice: &[T], subslice: &[T]) -> Option<Range<usize>> {
    assert!(size_of::<T>() != 0, "elements are zero-sized");

    let self_start = slice.as_ptr().addr();
    let subslice_start = subslice.as_ptr().addr();

    let byte_start = subslice_start.wrapping_sub(self_start);

    if !byte_start.is_multiple_of(size_of::<T>()) {
        return None;
    }

    let start = byte_start / size_of::<T>();
    let end = start.wrapping_add(subslice.len());

    (start <= slice.len() && end <= slice.len()).then_some(start..end)
}

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
