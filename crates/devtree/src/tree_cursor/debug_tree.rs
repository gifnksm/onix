use core::{
    cell::{RefCell, RefMut},
    fmt::{self, DebugMap},
};

use super::{TreeCursor, error::ReadTreeError};
use crate::{blob::Item, types::ByteStr};

enum CursorRef<'parent, 'cursor, TC>
where
    TC: ?Sized,
{
    Root(RefCell<&'cursor mut TC>),
    Ref(&'parent RefCell<&'cursor mut TC>),
}

impl<'parent, 'cursor, TC> CursorRef<'parent, 'cursor, TC>
where
    TC: ?Sized,
{
    fn new_root<'this>(cursor: &'cursor mut TC) -> CursorRef<'this, 'cursor, TC> {
        CursorRef::Root(RefCell::new(cursor))
    }

    fn cell(&self) -> &RefCell<&'cursor mut TC> {
        match self {
            CursorRef::Root(cell) => cell,
            CursorRef::Ref(cell) => cell,
        }
    }

    fn borrow_mut<'this, 'new>(&'this self) -> RefMut<'new, &'cursor mut TC>
    where
        'this: 'new,
        'parent: 'new,
    {
        match self {
            CursorRef::Root(cell) => cell.borrow_mut(),
            CursorRef::Ref(cell) => cell.borrow_mut(),
        }
    }

    fn make_ref<'this>(&'this self) -> CursorRef<'this, 'cursor, TC> {
        CursorRef::Ref(self.cell())
    }
}

pub struct DebugTree<'parent, 'cursor, TC>
where
    TC: ?Sized,
{
    cursor: CursorRef<'parent, 'cursor, TC>,
}

impl<'cursor, 'blob, TC> DebugTree<'_, 'cursor, TC>
where
    TC: TreeCursor<'blob> + ?Sized,
{
    pub fn new<'parent>(cursor: &'cursor mut TC) -> DebugTree<'parent, 'cursor, TC> {
        let cursor = CursorRef::new_root(cursor);
        DebugTree { cursor }
    }

    fn new_child<'this>(&'this self) -> DebugTree<'this, 'cursor, TC> {
        let cursor = self.cursor.make_ref();
        DebugTree { cursor }
    }

    fn dump(&self, dm: &mut DebugMap<'_, '_>) -> Result<(), ReadTreeError> {
        while let Some(item) = { self.cursor.borrow_mut().read_item_descend()? } {
            match item {
                Item::Property(property) => {
                    dm.entry(&property.name(), &ByteStr::new(property.value()));
                }
                Item::Node(node) => {
                    let name = if node.is_root() {
                        ByteStr::new(b"/")
                    } else {
                        node.full_name()
                    };
                    let child = self.new_child();
                    dm.entry(&name, &child);
                }
            }
        }
        self.cursor.borrow_mut().seek_parent_next()?;
        Ok(())
    }
}

impl<'blob, TC> fmt::Debug for DebugTree<'_, '_, TC>
where
    TC: TreeCursor<'blob> + ?Sized,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut dm = f.debug_map();
        if let Err(err) = self.dump(&mut dm) {
            dm.entry(&"<<error>>", &err);
        }
        dm.finish()
    }
}
