mod logic;
mod models;

pub mod prelude {

    pub use crate::logic::*;
    pub use crate::models::*;

    pub use bon::{Builder, bon, builder};
    pub use derive_more::{Deref, From};
    pub use getset::Getters;
    pub use log::*;
    pub use std::path::Path;
    pub use syn::{
        ItemEnum, ItemFn as ItemFunction, ItemImpl, ItemMacro, ItemStruct, ItemTrait, ItemType,
        ItemUnion, ItemUse,
    };
}
