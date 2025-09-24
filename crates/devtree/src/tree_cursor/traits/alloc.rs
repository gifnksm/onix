extern crate alloc;

use alloc::vec::Vec;

use super::TreeCursor;
use crate::types::ByteString;

pub trait TreeCursorAllocExt<'blob>: TreeCursor<'blob> {
    #[must_use]
    fn path(&self) -> ByteString {
        let mut components = Vec::new();
        for parent in self.parents() {
            if parent.is_root() {
                continue;
            }
            components.push(parent.full_name());
        }
        components.reverse();

        let mut path = ByteString::default();
        for component in &components {
            path.push(b'/');
            path.extend_from_slice(component);
        }
        path
    }
}

impl<'blob, T> TreeCursorAllocExt<'blob> for T where T: TreeCursor<'blob> + ?Sized {}
