use core::ops::Range;

// polyfill of unstable rust feature `slice_split_once`<https://github.com/rust-lang/rust/issues/112811>
pub(crate) fn slice_split_once<P, T>(s: &[T], pred: P) -> Option<(&[T], &[T])>
where
    P: FnMut(&T) -> bool,
{
    let pos = s.iter().position(pred)?;
    Some((&s[..pos], &s[pos + 1..]))
}

// polyfill of unstable rust feature `slice_split_once`<https://github.com/rust-lang/rust/issues/112811>
pub(crate) fn slice_rsplit_once<P, T>(s: &[T], pred: P) -> Option<(&[T], &[T])>
where
    P: FnMut(&T) -> bool,
{
    let pos = s.iter().rposition(pred)?;
    Some((&s[..pos], &s[pos + 1..]))
}

// polyfill of unstable rust feature `substr_range` <https://github.com/rust-lang/rust/issues/126769>
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

// polyfill of unstable rust feature `ptr_is_aligned_to` <https://github.com/rust-lang/rust/issues/96284>
pub(crate) fn ptr_is_aligned_to<T>(ptr: *const T, align: usize) -> bool {
    assert!(align.is_power_of_two());
    (ptr.addr() & (align - 1)) == 0
}

// polyfill of unstable rust feature `pointer_try_cast_aligned` <https://github.com/rust-lang/rust/issues/141221>
pub(crate) fn ptr_cast_aligned<T, U>(ptr: *const T) -> Option<*const U> {
    ptr_is_aligned_to(ptr, align_of::<U>()).then(|| ptr.cast())
}
