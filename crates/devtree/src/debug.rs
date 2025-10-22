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

#[cfg_attr(coverage_nightly, coverage(off))]
#[cfg(test)]
mod tests {
    extern crate alloc;

    use alloc::format;

    use super::*;

    #[test]
    fn test_slice_debug_within_max_len() {
        let arr = [1, 2, 3];
        let dbg = arr.slice_debug(5);
        let s = format!("{dbg:?}");
        assert_eq!(s, "[1, 2, 3]");
    }

    #[test]
    fn test_slice_debug_exact_max_len() {
        let arr = [1, 2, 3];
        let dbg = arr.slice_debug(3);
        let s = format!("{dbg:?}");
        assert_eq!(s, "[1, 2, 3]");
    }

    #[test]
    fn test_slice_debug_exceeds_max_len() {
        let arr = [1, 2, 3, 4, 5];
        let dbg = arr.slice_debug(3);
        let s = format!("{dbg:?}");
        assert_eq!(s, "[1, 2, 3, ...(2 more)]");
    }

    #[test]
    fn test_slice_debug_zero_max_len() {
        let arr = [1, 2, 3];
        let dbg = arr.slice_debug(0);
        let s = format!("{dbg:?}");
        assert_eq!(s, "[...(3 more)]");
    }

    #[test]
    fn test_slice_debug_empty_slice() {
        let arr: [i32; 0] = [];
        let dbg = arr.slice_debug(3);
        let s = format!("{dbg:?}");
        assert_eq!(s, "[]");
    }

    #[test]
    fn test_slice_debug_non_integer_type() {
        let arr = ["a", "b", "c", "d"];
        let dbg = arr.slice_debug(2);
        let s = format!("{dbg:?}");
        assert_eq!(s, r#"["a", "b", ...(2 more)]"#);
    }

    #[test]
    fn test_slice_debug_hex() {
        let arr = [
            0x10, 0x20, 0x30, 0x40, 0x50, 0x60, 0x70, 0x80, 0x90, 0xa0, 0xb0, 0xc0, 0xd0, 0xe0,
            0xf0,
        ];
        let dbg = arr.slice_debug(3);
        let s = format!("{dbg:x?}");
        assert_eq!(s, "[10, 20, 30, ...(12 more)]");
    }

    #[test]
    fn test_slice_debug_hex_pretty() {
        let arr = [
            0x10, 0x20, 0x30, 0x40, 0x50, 0x60, 0x70, 0x80, 0x90, 0xa0, 0xb0, 0xc0, 0xd0, 0xe0,
            0xf0,
        ];
        let dbg = arr.slice_debug(3);
        let s = format!("{dbg:#x?}");
        assert_eq!(
            s,
            "[\n    0x10,\n    0x20,\n    0x30,\n    ...(12 more),\n]"
        );
    }
}
