use syn::Ident;

pub trait Identifiable {
    fn ident(&self) -> &Ident;
}
