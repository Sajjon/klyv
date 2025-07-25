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
            syn::Item::Const(item_const) => Ok(SourceItem::unsplittable(item_const)),
            syn::Item::Enum(item_enum) => Ok(SourceItem::r#enum(item_enum)),
            syn::Item::ExternCrate(item_extern_crate) => {
                Ok(SourceItem::unsplittable(item_extern_crate))
            }
            syn::Item::Fn(item_fn) => Ok(SourceItem::function(item_fn)),
            syn::Item::ForeignMod(item_foreign_mod) => {
                Ok(SourceItem::unsplittable(item_foreign_mod))
            }
            syn::Item::Impl(item_impl) => Ok(SourceItem::r#impl(item_impl)),
            syn::Item::Macro(item_macro) => Ok(SourceItem::r#macro(item_macro)),
            syn::Item::Mod(item_mod) => Ok(SourceItem::unsplittable(item_mod)),
            syn::Item::Static(item_static) => Ok(SourceItem::unsplittable(item_static)),
            syn::Item::Struct(item_struct) => Ok(SourceItem::r#struct(item_struct)),
            syn::Item::Trait(item_trait) => Ok(SourceItem::r#trait(item_trait)),
            syn::Item::TraitAlias(item_trait_alias) => {
                Ok(SourceItem::unsplittable(item_trait_alias))
            }
            syn::Item::Type(item_type) => Ok(SourceItem::r#type(item_type)),
            syn::Item::Union(item_union) => Ok(SourceItem::r#union(item_union)),
            syn::Item::Use(item_use) => Ok(SourceItem::r#use(item_use)),
            syn::Item::Verbatim(token_stream) => Ok(SourceItem::Verbatim(token_stream)),
            _ => Err(Error::bail(
                "new unsupported item type, please file an issue at https://github.com/Sajjon/klyv/issues/new",
            )),
        }
    }
}

impl SourceItem {
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
