extern crate alloc;

use alloc::borrow::ToOwned;
use core::{borrow::Borrow, fmt, ops::Deref};

use super::{DEVICETREE_ALIGNMENT, Devicetree};
use crate::util::AlignedByteBuffer;

#[derive(Clone)]
pub struct OwnedDevicetree {
    buffer: AlignedByteBuffer<DEVICETREE_ALIGNMENT>,
}

unsafe impl Send for OwnedDevicetree {}
unsafe impl Sync for OwnedDevicetree {}

impl ToOwned for Devicetree {
    type Owned = OwnedDevicetree;

    fn to_owned(&self) -> Self::Owned {
        OwnedDevicetree {
            buffer: AlignedByteBuffer::from_slice(self.as_bytes()),
        }
    }
}

impl Borrow<Devicetree> for OwnedDevicetree {
    fn borrow(&self) -> &Devicetree {
        unsafe { Devicetree::from_bytes_unchecked(self.buffer.as_slice()) }
    }
}

impl AsRef<Devicetree> for OwnedDevicetree {
    fn as_ref(&self) -> &Devicetree {
        self.borrow()
    }
}

impl AsRef<[u8]> for OwnedDevicetree {
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl Deref for OwnedDevicetree {
    type Target = Devicetree;

    fn deref(&self) -> &Self::Target {
        self.borrow()
    }
}

impl fmt::Debug for OwnedDevicetree {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

#[cfg_attr(coverage_nightly, coverage(off))]
#[cfg(test)]
mod tests {
    extern crate alloc;

    use alloc::format;

    use super::*;

    #[repr(align(8))]
    struct Bytes<const N: usize>([u8; N]);

    #[test]
    fn test_to_owned() {
        let blob = Bytes(*include_bytes!("../../../examples/assets/qemu-virt.dtb"));
        let dt = Devicetree::from_bytes(&blob.0).unwrap();
        let owned = dt.to_owned();
        assert_eq!(owned.as_bytes(), dt.as_bytes());
    }

    #[test]
    fn test_clone_owned_devicetree() {
        let blob = Bytes(*include_bytes!("../../../examples/assets/qemu-virt.dtb"));
        let dt = Devicetree::from_bytes(&blob.0).unwrap();
        let owned = dt.to_owned();
        let cloned = owned.clone();
        assert_eq!(cloned.as_bytes(), owned.as_bytes());
        assert_ne!(cloned.as_bytes().as_ptr(), owned.as_bytes().as_ptr()); // Should be a deep copy
    }

    #[test]
    fn test_deref_and_borrow() {
        let blob = Bytes(*include_bytes!("../../../examples/assets/qemu-virt.dtb"));
        let dt = Devicetree::from_bytes(&blob.0).unwrap();
        let owned = dt.to_owned();
        let deref_dt: &Devicetree = &owned;
        let borrow_dt: &Devicetree = owned.borrow();
        assert_eq!(deref_dt.as_bytes(), dt.as_bytes());
        assert_eq!(borrow_dt.as_bytes(), dt.as_bytes());
    }

    #[test]
    fn test_as_ref_slice_and_devicetree() {
        let blob = Bytes(*include_bytes!("../../../examples/assets/qemu-virt.dtb"));
        let dt = Devicetree::from_bytes(&blob.0).unwrap();
        let owned = dt.to_owned();
        let as_ref_slice: &[u8] = owned.as_ref();
        let as_ref_dt: &Devicetree = owned.as_ref();
        assert_eq!(as_ref_slice, dt.as_bytes());
        assert_eq!(as_ref_dt.as_bytes(), dt.as_bytes());
    }

    #[test]
    fn test_debug_fmt() {
        let blob = Bytes(*include_bytes!("../../../examples/assets/qemu-virt.dtb"));
        let dt = Devicetree::from_bytes(&blob.0).unwrap();
        let owned = dt.to_owned();
        let debug_str = format!("{owned:?}");
        let dt_debug_str = format!("{dt:?}");
        assert_eq!(debug_str, dt_debug_str);
    }
}
