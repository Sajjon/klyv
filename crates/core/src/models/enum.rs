use crate::prelude::*;
use syn::Ident;

#[derive(Clone, Deref, From)]
pub struct Enum(ItemEnum);

impl std::fmt::Debug for Enum {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Enum")
            .field(&self.vis)
            .field(&self.ident)
            .field(&self.generics)
            .field(&self.attrs)
            .field(&self.variants)
            .finish()
    }
}

impl Identifiable for Enum {
    fn ident(&self) -> &Ident {
        &self.ident
    }
}
