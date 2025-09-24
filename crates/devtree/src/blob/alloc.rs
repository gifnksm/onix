extern crate alloc;

use alloc::{borrow::ToOwned, boxed::Box};

use super::Devicetree;

impl ToOwned for Devicetree {
    type Owned = Box<Self>;

    fn to_owned(&self) -> Self::Owned {
        let owned: Box<[u8]> = Box::from(self.as_bytes());
        let raw = Box::into_raw(owned) as *mut Self;
        unsafe { Box::from_raw(raw) }
    }
}
