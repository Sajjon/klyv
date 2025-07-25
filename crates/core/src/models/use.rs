use crate::prelude::*;

#[derive(Clone, Deref, From)]
pub struct Use(ItemUse);

impl std::fmt::Debug for Use {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Use")
            .field(&self.vis)
            .field(&self.attrs)
            .field(&self.tree)
            .finish()
    }
}
