extern crate alloc;

use core::{
    alloc::Layout,
    borrow::{Borrow, BorrowMut},
    fmt,
    ops::{Deref, DerefMut},
    ptr, slice,
};

pub struct AlignedByteBuffer<const ALIGN: usize> {
    ptr: *mut u8,
    size: usize,
}

unsafe impl<const ALIGN: usize> Send for AlignedByteBuffer<ALIGN> {}
unsafe impl<const ALIGN: usize> Sync for AlignedByteBuffer<ALIGN> {}

impl<const ALIGN: usize> AlignedByteBuffer<ALIGN> {
    #[must_use]
    pub fn new_zeroed(size: usize) -> Self {
        if size == 0 {
            return Self {
                ptr: ptr::dangling_mut(),
                size,
            };
        }

        let layout = Layout::from_size_align(size, ALIGN).unwrap();
        let ptr = unsafe { alloc::alloc::alloc_zeroed(layout) };
        if ptr.is_null() {
            alloc::alloc::handle_alloc_error(layout);
        }
        Self { ptr, size }
    }

    #[must_use]
    pub fn from_slice(data: &[u8]) -> Self {
        let size = data.len();
        if data.is_empty() {
            return Self {
                ptr: ptr::dangling_mut(),
                size,
            };
        }

        let layout = Layout::from_size_align(size, ALIGN).unwrap();
        let ptr = unsafe { alloc::alloc::alloc(layout) };
        if ptr.is_null() {
            alloc::alloc::handle_alloc_error(layout);
        }
        unsafe {
            ptr.copy_from_nonoverlapping(data.as_ptr(), size);
        }
        Self { ptr, size }
    }

    #[must_use]
    pub fn as_ptr(&self) -> *const u8 {
        self.ptr
    }

    #[must_use]
    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        self.ptr
    }

    #[must_use]
    pub fn as_slice(&self) -> &[u8] {
        unsafe { slice::from_raw_parts(self.ptr, self.size) }
    }

    #[must_use]
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        unsafe { slice::from_raw_parts_mut(self.ptr, self.size) }
    }
}

impl<const ALIGN: usize> Drop for AlignedByteBuffer<ALIGN> {
    fn drop(&mut self) {
        if self.size == 0 {
            self.ptr = ptr::null_mut();
            return;
        }

        unsafe {
            let layout = Layout::from_size_align(self.size, ALIGN).unwrap();
            alloc::alloc::dealloc(self.ptr, layout);
            self.ptr = ptr::null_mut();
            self.size = 0;
        }
    }
}

impl<const ALIGN: usize> Deref for AlignedByteBuffer<ALIGN> {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl<const ALIGN: usize> DerefMut for AlignedByteBuffer<ALIGN> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut_slice()
    }
}

impl<const ALIGN: usize> Borrow<[u8]> for AlignedByteBuffer<ALIGN> {
    fn borrow(&self) -> &[u8] {
        self.as_slice()
    }
}

impl<const ALIGN: usize> BorrowMut<[u8]> for AlignedByteBuffer<ALIGN> {
    fn borrow_mut(&mut self) -> &mut [u8] {
        self.as_mut_slice()
    }
}

impl<const ALIGN: usize> AsRef<[u8]> for AlignedByteBuffer<ALIGN> {
    fn as_ref(&self) -> &[u8] {
        self.as_slice()
    }
}

impl<const ALIGN: usize> AsMut<[u8]> for AlignedByteBuffer<ALIGN> {
    fn as_mut(&mut self) -> &mut [u8] {
        self.as_mut_slice()
    }
}

impl<const ALIGN: usize> Clone for AlignedByteBuffer<ALIGN> {
    fn clone(&self) -> Self {
        Self::from_slice(self.as_slice())
    }
}

impl<const ALIGN: usize> fmt::Debug for AlignedByteBuffer<ALIGN> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.as_slice(), f)
    }
}

#[cfg_attr(coverage_nightly, coverage(off))]
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_zeroed() {
        let buf = AlignedByteBuffer::<16>::new_zeroed(64);
        assert_eq!(buf.len(), 64);
        assert!(buf.iter().all(|&b| b == 0));
    }

    #[test]
    fn test_new_zeroed_empty() {
        let buf = AlignedByteBuffer::<16>::new_zeroed(0);
        assert_eq!(buf.len(), 0);
        assert!(buf.is_empty());
    }

    #[test]
    fn test_from_slice() {
        let data = [1, 2, 3, 4, 5];
        let buf = AlignedByteBuffer::<8>::from_slice(&data);
        assert_eq!(buf.as_slice(), &data);
    }

    #[test]
    fn test_from_slice_empty() {
        let buf = AlignedByteBuffer::<8>::from_slice(&[]);
        assert_eq!(buf.len(), 0);
    }

    #[test]
    fn test_as_mut_slice() {
        let mut buf = AlignedByteBuffer::<16>::new_zeroed(10);
        buf.as_mut_slice()[0] = 42;
        assert_eq!(buf[0], 42);
    }

    #[test]
    fn test_deref() {
        let data = [1, 2, 3];
        let buf = AlignedByteBuffer::<4>::from_slice(&data);
        assert_eq!(&*buf, &data);
    }

    #[test]
    fn test_deref_mut() {
        let mut buf = AlignedByteBuffer::<4>::from_slice(&[1, 2, 3]);
        buf[1] = 10;
        assert_eq!(buf[1], 10);
    }

    #[test]
    fn test_clone() {
        let buf1 = AlignedByteBuffer::<8>::from_slice(&[1, 2, 3, 4]);
        let buf2 = buf1.clone();
        assert_eq!(buf1.as_slice(), buf2.as_slice());
    }

    #[test]
    fn test_borrow() {
        let buf = AlignedByteBuffer::<16>::from_slice(&[5, 6, 7]);
        let borrowed: &[u8] = buf.borrow();
        assert_eq!(borrowed, &[5, 6, 7]);
    }

    #[test]
    fn test_borrow_mut() {
        let mut buf = AlignedByteBuffer::<16>::from_slice(&[5, 6, 7]);
        let borrowed: &mut [u8] = buf.borrow_mut();
        borrowed[0] = 99;
        assert_eq!(buf[0], 99);
    }

    #[test]
    fn test_as_ref() {
        let buf = AlignedByteBuffer::<8>::from_slice(&[10, 20]);
        let slice: &[u8] = buf.as_ref();
        assert_eq!(slice, &[10, 20]);
    }

    #[test]
    fn test_as_mut() {
        let mut buf = AlignedByteBuffer::<8>::from_slice(&[10, 20]);
        let slice: &mut [u8] = buf.as_mut();
        slice[1] = 30;
        assert_eq!(buf[1], 30);
    }

    #[test]
    fn test_alignment() {
        let mut buf = AlignedByteBuffer::<64>::new_zeroed(128);
        assert_eq!(buf.as_ptr() as usize % 64, 0);
        assert_eq!(buf.as_mut_ptr() as usize % 64, 0);
    }

    #[test]
    fn test_drop() {
        let buf = AlignedByteBuffer::<16>::new_zeroed(100);
        drop(buf); // Should not panic
    }

    #[test]
    fn test_debug_fmt() {
        let buf = AlignedByteBuffer::<8>::from_slice(&[1, 2, 3]);
        assert_eq!(alloc::format!("{buf:?}"), "[1, 2, 3]");
    }
}
