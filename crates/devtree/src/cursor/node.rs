use core::{cell::RefCell, fmt};

use snafu::{ResultExt as _, Snafu, ensure};
use snafu_utils::Location;

use super::{ItemCursor, PropertyCursor, ReadTokenError, TokenCursor};
use crate::{
    blob::{Devicetree, Node},
    cursor::Token,
    types::ByteStr,
};

pub struct NodeCursor<'parent, 'blob> {
    node: Node<'blob>,
    parent: Option<&'parent Self>,
    state: ReadState,
    cursor: TokenCursorRef<'parent, 'blob>,
}

impl<'parent, 'blob> NodeCursor<'parent, 'blob> {
    #[must_use]
    pub fn node(&self) -> &Node<'blob> {
        &self.node
    }

    #[must_use]
    pub fn devicetree(&self) -> &'blob Devicetree {
        self.cursor.devicetree()
    }

    #[must_use]
    pub fn parent(&self) -> Option<&'parent Self> {
        self.parent.as_ref().copied()
    }

    #[must_use]
    pub fn root<'root>(&self) -> NodeCursor<'root, 'blob> {
        let mut cursor = self;
        while let Some(parent) = cursor.parent() {
            cursor = parent;
        }
        NodeCursor {
            node: cursor.node.clone(),
            parent: None,
            state: ReadState::Property,
            cursor: TokenCursorRef::new_root(cursor.node.items_start_cursor()),
        }
    }

    #[must_use]
    pub fn is_root(&self) -> bool {
        self.parent.is_none()
    }
}

impl Clone for NodeCursor<'_, '_> {
    fn clone(&self) -> Self {
        NodeCursor {
            node: self.node.clone(),
            parent: self.parent(),
            state: ReadState::default(),
            cursor: TokenCursorRef::new_root(self.node.items_start_cursor()),
        }
    }
}

impl Drop for NodeCursor<'_, '_> {
    fn drop(&mut self) {
        if !self.cursor.is_root() {
            while let Ok(Some(_)) = self.read_item() {}
        }
    }
}

#[derive(Debug)]
struct TokenCursorRef<'parent, 'blob> {
    devicetree: &'blob Devicetree,
    position: PositionRef<'parent>,
}

#[derive(Debug)]
enum PositionRef<'parent> {
    Root(RefCell<usize>),
    Ref(&'parent RefCell<usize>),
}

impl<'blob> TokenCursorRef<'_, 'blob> {
    fn new_root(cursor: &TokenCursor<'blob>) -> TokenCursorRef<'static, 'blob> {
        let devicetree = cursor.devicetree();
        let position = PositionRef::Root(RefCell::new(cursor.position()));
        TokenCursorRef {
            devicetree,
            position,
        }
    }

    fn make_ref<'this>(&'this self) -> TokenCursorRef<'this, 'blob> {
        TokenCursorRef {
            devicetree: self.devicetree,
            position: PositionRef::Ref(self.position_cell()),
        }
    }

    fn is_root(&self) -> bool {
        matches!(self.position, PositionRef::Root(_))
    }

    fn position_cell(&self) -> &RefCell<usize> {
        match &self.position {
            PositionRef::Root(cell) => cell,
            PositionRef::Ref(cell) => cell,
        }
    }

    fn read_token(&self) -> Result<Option<Token<'blob>>, ReadTokenError> {
        let mut cursor = self.make_cursor();
        let res = cursor.read_token();
        *self.position_cell().borrow_mut() = cursor.position();
        res
    }

    fn devicetree(&self) -> &'blob Devicetree {
        self.devicetree
    }

    fn position(&self) -> usize {
        *self.position_cell().borrow()
    }

    fn make_cursor(&self) -> TokenCursor<'blob> {
        TokenCursor::from_parts(self.devicetree, self.position())
    }
}

#[derive(Debug, Snafu)]
#[snafu(module)]
#[non_exhaustive]
pub enum ReadNodeError {
    #[snafu(display("failed to read DTB token"))]
    #[snafu(provide(ref, priority, Location => location))]
    ReadToken {
        #[snafu(source)]
        source: ReadTokenError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("missing begin node token in DTB structure block: offset={offset}"))]
    #[snafu(provide(ref, priority, Location => location))]
    MissingBeginNodeToken {
        offset: usize,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("unexpected property token found in DTB structure block: offset={offset}"))]
    UnexpectedPropertyToken {
        offset: usize,
        #[snafu(implicit)]
        location: Location,
    },
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum ReadState {
    #[default]
    Property,
    Child,
    Done,
}

impl<'root, 'blob> NodeCursor<'root, 'blob> {
    pub(crate) fn read(mut cursor: TokenCursor<'blob>) -> Result<Self, ReadNodeError> {
        #[cfg_attr(not(test), expect(clippy::wildcard_imports))]
        use self::read_node_error::*;

        let node_start = cursor.position();
        match cursor.read_token().context(ReadTokenSnafu)? {
            Some(Token::BeginNode { full_name }) => {
                let node = Node::new(full_name, cursor.clone());
                Ok(NodeCursor {
                    node,
                    parent: None,
                    state: ReadState::default(),
                    cursor: TokenCursorRef::new_root(&cursor),
                })
            }
            _ => MissingBeginNodeTokenSnafu { offset: node_start }.fail(),
        }
    }

    fn new_child(
        parent: &'root Self,
        full_name: &'blob ByteStr,
        cursor: TokenCursorRef<'root, 'blob>,
    ) -> Self {
        let node = Node::new(full_name, cursor.make_cursor());
        Self {
            node,
            parent: Some(parent),
            state: ReadState::default(),
            cursor,
        }
    }
}

impl<'blob> NodeCursor<'_, 'blob> {
    pub fn read_item(&mut self) -> Result<Option<ItemCursor<'_, 'blob>>, ReadNodeError> {
        #[cfg_attr(not(test), expect(clippy::wildcard_imports))]
        use self::read_node_error::*;

        let in_property_state = match self.state {
            ReadState::Property => true,
            ReadState::Child => false,
            ReadState::Done => return Ok(None),
        };

        let position = self.cursor.position();
        let token = self.cursor.read_token().context(ReadTokenSnafu)?;
        match token {
            Some(Token::Property { name_offset, value }) => {
                ensure!(
                    in_property_state,
                    UnexpectedPropertyTokenSnafu { offset: position }
                );
                Ok(Some(ItemCursor::Property(PropertyCursor::new(
                    self,
                    name_offset,
                    value,
                ))))
            }
            Some(Token::BeginNode { full_name }) => {
                if in_property_state {
                    self.state = ReadState::Child;
                }
                Ok(Some(ItemCursor::Node(NodeCursor::new_child(
                    self,
                    full_name,
                    self.cursor.make_ref(),
                ))))
            }
            Some(Token::EndNode) | None => {
                self.state = ReadState::Done;
                Ok(None)
            }
        }
    }
}

impl fmt::Debug for NodeCursor<'_, '_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        struct FmtItems<'root, 'blob>(RefCell<NodeCursor<'root, 'blob>>);

        impl fmt::Debug for FmtItems<'_, '_> {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                let mut dm = f.debug_map();
                loop {
                    let mut node = self.0.borrow_mut();
                    let res = node.read_item();
                    match res {
                        Ok(Some(ItemCursor::Property(cursor))) => {
                            let property = cursor.property();
                            dm.entry(&property.name(), &ByteStr::new(property.value()));
                        }
                        Ok(Some(ItemCursor::Node(cursor))) => {
                            let node = cursor.node();
                            dm.entry(&node.full_name(), &FmtItems(RefCell::new(cursor)));
                        }
                        Ok(None) => break,
                        Err(err) => {
                            dm.entry(&"<<error>>", &err);
                            return Err(fmt::Error);
                        }
                    }
                }
                dm.finish()
            }
        }

        let mut dm = f.debug_map();
        if self.is_root() {
            dm.entry(&"/", &FmtItems(RefCell::new(self.clone())));
        } else {
            dm.entry(
                &self.node().full_name(),
                &FmtItems(RefCell::new(self.clone())),
            );
        }
        dm.finish()
    }
}
