use crate::prelude::*;
use syn::Ident;

#[derive(Clone, Deref, From)]
pub struct Struct(ItemStruct);

impl std::fmt::Debug for Struct {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Struct")
            .field(&self.vis)
            .field(&self.ident)
            .field(&self.generics)
            .field(&self.attrs)
            .field(&self.fields)
            .finish()
    }
}

impl Identifiable for Struct {
    fn ident(&self) -> &Ident {
        &self.ident
    }
}
