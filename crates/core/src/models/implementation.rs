use crate::prelude::*;

#[derive(Clone, Deref, From)]
pub struct Implementation(ItemImpl);

impl std::fmt::Debug for Implementation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Implementation")
            .field(&self.generics)
            .field(&self.self_ty)
            .field(&self.trait_)
            .field(&self.attrs)
            .field(&self.items)
            .finish()
    }
}
