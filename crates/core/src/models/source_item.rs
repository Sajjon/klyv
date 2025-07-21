use proc_macro2::TokenStream;
use syn::Item;

use crate::prelude::*;

/// A Rust type, struct, enum, typealias, function, macro or implementation of
/// struct or enum.
#[derive(Clone, Debug)]
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

#[derive(Clone, Deref, From)]
pub struct Function(ItemFunction);

impl std::fmt::Debug for Function {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Function")
            .field(&self.vis)
            .field(&self.sig)
            .field(&self.attrs)
            .finish()
    }
}

#[derive(Clone, Deref, From)]
pub struct MacroRules(ItemMacro);

impl std::fmt::Debug for MacroRules {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("MacroRules")
            .field(&self.ident)
            .field(&self.attrs)
            .field(&self.mac)
            .finish()
    }
}

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
