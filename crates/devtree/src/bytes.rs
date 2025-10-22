use core::{
    cmp, fmt,
    sync::atomic::{AtomicUsize, Ordering},
};

use crate::types::ByteStr;

pub(crate) struct LazyCStr<'blob> {
    len: AtomicUsize,
    bytes: &'blob [u8],
}

impl<'blob> LazyCStr<'blob> {
    pub(crate) fn new<S>(bytes: &'blob S) -> Self
    where
        S: AsRef<[u8]> + ?Sized,
    {
        let bytes = bytes.as_ref();
        Self {
            len: AtomicUsize::new(usize::MAX),
            bytes,
        }
    }

    pub(crate) fn len(&self) -> usize {
        let len = self.len.load(Ordering::Relaxed);
        if len != usize::MAX {
            return len;
        }

        let len = self
            .bytes
            .iter()
            .position(|&b| b == 0)
            .unwrap_or(self.bytes.len());
        self.len.store(len, Ordering::Relaxed);
        len
    }

    pub(crate) fn as_bytes(&self) -> &'blob [u8] {
        &self.bytes[..self.len()]
    }

    pub(crate) fn as_byte_str(&self) -> &'blob ByteStr {
        ByteStr::new(self.as_bytes())
    }
}

impl Clone for LazyCStr<'_> {
    fn clone(&self) -> Self {
        Self {
            len: AtomicUsize::new(self.len.load(Ordering::Relaxed)),
            bytes: self.bytes,
        }
    }
}

impl fmt::Debug for LazyCStr<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self.as_byte_str(), f)
    }
}

impl PartialEq for LazyCStr<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.as_bytes() == other.as_bytes()
    }
}

macro_rules! impl_partial_eq {
    ($lhs:ty, $rhs:ty) => {
        impl<'a> PartialEq<$rhs> for $lhs {
            #[inline]
            fn eq(&self, other: &$rhs) -> bool {
                let other: &[u8] = other.as_ref();
                PartialEq::eq(self.as_bytes(), other)
            }
        }

        impl<'a> PartialEq<$lhs> for $rhs {
            #[inline]
            fn eq(&self, other: &$lhs) -> bool {
                let this: &[u8] = self.as_ref();
                PartialEq::eq(this, other.as_bytes())
            }
        }
    };
}

macro_rules! impl_partial_eq_n {
    ($lhs:ty, $rhs:ty) => {
        impl<'a, const N: usize> PartialEq<$rhs> for $lhs {
            #[inline]
            fn eq(&self, other: &$rhs) -> bool {
                let other: &[u8] = other.as_ref();
                PartialEq::eq(self.as_bytes(), other)
            }
        }

        impl<'a, const N: usize> PartialEq<$lhs> for $rhs {
            #[inline]
            fn eq(&self, other: &$lhs) -> bool {
                let this: &[u8] = self.as_ref();
                PartialEq::eq(this, other.as_bytes())
            }
        }
    };
}

impl_partial_eq!(LazyCStr<'_>, [u8]);
impl_partial_eq!(LazyCStr<'_>, &'a [u8]);
impl_partial_eq!(LazyCStr<'_>, str);
impl_partial_eq!(LazyCStr<'_>, &'a str);
impl_partial_eq!(LazyCStr<'_>, ByteStr);
impl_partial_eq!(LazyCStr<'_>, &'a ByteStr);
impl_partial_eq_n!(LazyCStr<'_>, [u8; N]);
impl_partial_eq_n!(LazyCStr<'_>, &'a [u8; N]);

impl Eq for LazyCStr<'_> {}

impl PartialOrd for LazyCStr<'_> {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

macro_rules! impl_partial_ord {
    ($lhs:ty, $rhs:ty) => {
        impl<'a> PartialOrd<$rhs> for $lhs {
            #[inline]
            fn partial_cmp(&self, other: &$rhs) -> Option<cmp::Ordering> {
                let other: &[u8] = other.as_ref();
                PartialOrd::partial_cmp(self.as_bytes(), other)
            }
        }

        impl<'a> PartialOrd<$lhs> for $rhs {
            #[inline]
            fn partial_cmp(&self, other: &$lhs) -> Option<cmp::Ordering> {
                let this: &[u8] = self.as_ref();
                PartialOrd::partial_cmp(this, other.as_bytes())
            }
        }
    };
}

macro_rules! impl_partial_ord_n {
    ($lhs:ty, $rhs:ty) => {
        impl<'a, const N: usize> PartialOrd<$rhs> for $lhs {
            #[inline]
            fn partial_cmp(&self, other: &$rhs) -> Option<cmp::Ordering> {
                let other: &[u8] = other.as_ref();
                PartialOrd::partial_cmp(self.as_bytes(), other)
            }
        }

        impl<'a, const N: usize> PartialOrd<$lhs> for $rhs {
            #[inline]
            fn partial_cmp(&self, other: &$lhs) -> Option<cmp::Ordering> {
                let this: &[u8] = self.as_ref();
                PartialOrd::partial_cmp(this, other.as_bytes())
            }
        }
    };
}

impl_partial_ord!(LazyCStr<'_>, [u8]);
impl_partial_ord!(LazyCStr<'_>, &'a [u8]);
impl_partial_ord!(LazyCStr<'_>, str);
impl_partial_ord!(LazyCStr<'_>, &'a str);
impl_partial_ord!(LazyCStr<'_>, ByteStr);
impl_partial_ord!(LazyCStr<'_>, &'a ByteStr);
impl_partial_ord_n!(LazyCStr<'_>, [u8; N]);
impl_partial_ord_n!(LazyCStr<'_>, &'a [u8; N]);

impl Ord for LazyCStr<'_> {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.as_bytes().cmp(other.as_bytes())
    }
}

impl AsRef<Self> for LazyCStr<'_> {
    fn as_ref(&self) -> &Self {
        self
    }
}

impl AsRef<ByteStr> for LazyCStr<'_> {
    fn as_ref(&self) -> &ByteStr {
        self.as_byte_str()
    }
}

impl AsRef<[u8]> for LazyCStr<'_> {
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}

#[cfg_attr(coverage_nightly, coverage(off))]
#[cfg(test)]
mod tests {
    extern crate alloc;

    use alloc::format;

    use super::*;

    #[test]
    fn test_new_and_len() {
        let bytes = b"hello\0world";
        let cstr = LazyCStr::new(bytes);
        assert_eq!(cstr.len(), 5);
        assert_eq!(cstr.as_bytes(), b"hello");
    }

    #[test]
    fn test_no_null_terminator() {
        let bytes = b"hello";
        let cstr = LazyCStr::new(bytes);
        assert_eq!(cstr.len(), 5);
        assert_eq!(cstr.as_bytes(), b"hello");
    }

    #[test]
    fn test_empty() {
        let bytes = b"";
        let cstr = LazyCStr::new(bytes);
        assert_eq!(cstr.len(), 0);
        assert_eq!(cstr.as_bytes(), b"");
    }

    #[test]
    fn test_clone_preserves_len() {
        let bytes = b"abc\0def";
        let cstr = LazyCStr::new(bytes);
        assert_eq!(cstr.len(), 3);
        let cloned = cstr.clone();
        assert_eq!(cloned.len(), 3);
        assert_eq!(cloned.as_bytes(), b"abc");
    }

    #[test]
    fn test_eq_and_ord() {
        let a = LazyCStr::new(b"abc\0def");
        let b = LazyCStr::new(b"abc\0xyz");
        let c = LazyCStr::new(b"abd\0def");
        assert_eq!(a, b);
        assert!(a < c);
        assert!(c > b);
    }

    #[test]
    fn test_partial_eq_with_bytestr() {
        let bytes = b"foo\0bar";
        let cstr = LazyCStr::new(bytes);
        let bs = ByteStr::new(b"foo");
        assert_eq!(cstr, bs);
        assert_eq!(bs, cstr);

        let bs = b"foo";
        assert_eq!(cstr, bs);
        assert_eq!(bs, cstr);
    }

    #[test]
    fn test_partial_ord_with_bytestr() {
        let bytes = b"abc\0bar";
        let cstr = LazyCStr::new(bytes);
        let bs = ByteStr::new(b"abd");
        assert!(cstr < bs);
        assert!(bs > cstr);

        let bs = b"abd";
        assert!(cstr < bs);
        assert!(bs > cstr);
    }

    #[test]
    fn test_as_ref_impls() {
        let bytes = b"xyz\0";
        let cstr = LazyCStr::new(bytes);
        let as_self: &LazyCStr = cstr.as_ref();
        assert_eq!(as_self.as_bytes(), b"xyz");
        let as_bytes: &[u8] = cstr.as_ref();
        assert_eq!(as_bytes, b"xyz");
        let as_bytestr: &ByteStr = cstr.as_ref();
        assert_eq!(as_bytestr, ByteStr::new(b"xyz"));
    }

    #[test]
    fn test_debug_fmt() {
        let bytes = b"abc\0";
        let cstr = LazyCStr::new(bytes);
        let debug_str = format!("{cstr:?}");
        assert_eq!(debug_str, format!("{:?}", "abc"));
    }
}
