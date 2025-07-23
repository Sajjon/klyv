use crate::prelude::*;

/// A named collection of source items from a single Rust file
#[derive(Clone, Debug, Getters, Builder)]
pub struct NamedSourceItems {
    #[getset(get = "pub")]
    items: Vec<SourceItem>,

    /// Name of the file
    #[getset(get = "pub")]
    name: String,
}
