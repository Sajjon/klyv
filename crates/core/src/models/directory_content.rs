use crate::prelude::*;

/// Type alias for a directory content, which is a NodeContent containing a vector of FileSystemNode
pub type DirectoryContent = NodeContent<Vec<FileSystemNode>>;
