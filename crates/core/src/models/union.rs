use crate::prelude::*;

#[derive(Clone, Deref, From)]
pub struct Union(ItemUnion);

impl std::fmt::Debug for Union {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Union")
            .field(&self.vis)
            .field(&self.ident)
            .field(&self.generics)
            .field(&self.attrs)
            .field(&self.fields)
            .finish()
    }
}
