use crate::prelude::*;

#[derive(Clone, Deref, From)]
pub struct Type(ItemType);

impl std::fmt::Debug for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Type")
            .field(&self.vis)
            .field(&self.ident)
            .field(&self.generics)
            .field(&self.attrs)
            .field(&self.ty)
            .finish()
    }
}
