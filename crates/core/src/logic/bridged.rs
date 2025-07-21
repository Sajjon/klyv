use crate::prelude::*;
use std::fs::{self, ReadDir};

use bon::builder;
use syn::File;

#[builder]
pub fn read_dir(path: impl AsRef<Path>) -> Result<ReadDir> {
    fs::read_dir(path).map_err(Error::from)
}

#[builder]
pub fn read_to_string(path: impl AsRef<Path>) -> Result<String> {
    fs::read_to_string(path).map_err(Error::from)
}

#[builder]
pub fn parse_file(content: &str) -> Result<File> {
    syn::parse_file(content).map_err(Error::from)
}
