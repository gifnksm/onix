pub use self::{item::*, node::*, property::*, token::*};
use crate::blob::Devicetree;

mod item;
mod node;
mod property;
mod token;

impl Devicetree {
    #[must_use]
    pub fn read_token(&self) -> TokenCursor<'_> {
        TokenCursor::new(self)
    }

    pub fn read_root_node(&self) -> Result<NodeCursor<'_, '_>, ReadNodeError> {
        let cursor = TokenCursor::new(self);
        NodeCursor::read(cursor)
    }
}
