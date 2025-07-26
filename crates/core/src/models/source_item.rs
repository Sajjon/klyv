use crate::prelude::*;
use derive_more::TryUnwrap;
use proc_macro2::TokenStream;
use syn::Item;

/// A Rust type, struct, enum, typealias, function, macro or implementation of
/// struct or enum.
#[derive(Clone, Debug, TryUnwrap)]
pub enum SourceItem {
    Enum(Enum),
    Struct(Struct),
    Trait(Trait),
    Type(Type),
    Union(Union),
    Function(Function),
    MacroRules(MacroRules),
    Impl(Implementation),
    Use(Use),
    /// Item for which klyv is unable to determine corresponding type
    Unsplittable(syn::Item),
    /// Item for which klyv - and even `syn` crate is unable to determine corresponding type
    Verbatim(TokenStream),
}

impl TryFrom<syn::Item> for SourceItem {
    type Error = Error;
    fn try_from(value: syn::Item) -> Result<Self> {
        match value {
            syn::Item::Const(item) => Ok(SourceItem::unsplittable(item)),
            syn::Item::Enum(item) => Ok(SourceItem::r#enum(item)),
            syn::Item::ExternCrate(item) => Ok(SourceItem::unsplittable(item)),
            syn::Item::Fn(item) => Ok(SourceItem::function(item)),
            syn::Item::ForeignMod(item) => Ok(SourceItem::unsplittable(item)),
            syn::Item::Impl(item) => Ok(SourceItem::r#impl(item)),
            syn::Item::Macro(item) => Ok(SourceItem::r#macro(item)),
            syn::Item::Mod(item) => Ok(SourceItem::unsplittable(item)),
            syn::Item::Static(item) => Ok(SourceItem::unsplittable(item)),
            syn::Item::Struct(item) => Ok(SourceItem::r#struct(item)),
            syn::Item::Trait(item) => Ok(SourceItem::r#trait(item)),
            syn::Item::TraitAlias(item) => Ok(SourceItem::unsplittable(item)),
            syn::Item::Type(item) => Ok(SourceItem::r#type(item)),
            syn::Item::Union(item) => Ok(SourceItem::r#union(item)),
            syn::Item::Use(item) => Ok(SourceItem::r#use(item)),
            syn::Item::Verbatim(tokens) => Ok(SourceItem::Verbatim(tokens)),
            _ => Self::handle_unsupported_item(),
        }
    }
}

impl SourceItem {
    /// Handles unsupported item types
    fn handle_unsupported_item() -> Result<Self> {
        Err(Error::bail(
            "new unsupported item type, please file an issue at https://github.com/Sajjon/klyv/issues/new",
        ))
    }
    pub fn r#enum(item: impl Into<Enum>) -> Self {
        Self::Enum(item.into())
    }
    pub fn r#struct(item: impl Into<Struct>) -> Self {
        Self::Struct(item.into())
    }
    pub fn r#trait(item: impl Into<Trait>) -> Self {
        Self::Trait(item.into())
    }
    pub fn r#type(item: impl Into<Type>) -> Self {
        Self::Type(item.into())
    }
    pub fn r#union(item: impl Into<Union>) -> Self {
        Self::Union(item.into())
    }
    pub fn function(item: impl Into<Function>) -> Self {
        Self::Function(item.into())
    }
    pub fn r#macro(item: impl Into<MacroRules>) -> Self {
        Self::MacroRules(item.into())
    }
    pub fn r#impl(item: impl Into<Implementation>) -> Self {
        Self::Impl(item.into())
    }
    pub fn r#use(item: impl Into<Use>) -> Self {
        Self::Use(item.into())
    }
    pub fn unsplittable(item: impl Into<Item>) -> Self {
        Self::Unsplittable(item.into())
    }
}
