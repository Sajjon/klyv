use crate::prelude::*;

#[derive(Clone, Deref, From)]
pub struct Trait(ItemTrait);

impl std::fmt::Debug for Trait {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Trait")
            .field(&self.vis)
            .field(&self.ident)
            .field(&self.generics)
            .field(&self.attrs)
            .field(&self.items)
            .finish()
    }
}
