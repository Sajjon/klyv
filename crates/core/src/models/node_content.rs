use std::path::PathBuf;

use crate::prelude::*;

/// Represents the content of a node in the file system
#[derive(Clone, Debug, Getters, Builder)]
pub struct NodeContent<C> {
    /// The name of the directory or file
    #[getset(get = "pub")]
    name: String,
    /// The path to the directory or file
    #[getset(get = "pub")]
    path: PathBuf,
    /// The content of the node, which can be a directory or a Rust file
    #[getset(get = "pub")]
    content: C,
}

impl FileWritable for DirectoryContent {
    fn write_to(&self, path: impl AsRef<Path>) -> Result<()> {
        self.content()
            .iter()
            .map(|node| node.write_to(path.as_ref().join(node.name())))
            .collect::<Result<Vec<()>>>()?;
        Ok(())
    }
}
