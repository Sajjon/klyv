use crate::prelude::*;

/// A file system node that can be either a directory or a Rust file
#[derive(Clone, Debug)]
pub enum FileSystemNode {
    /// A directory containing other nodes
    Directory(DirectoryContent),
    /// A Rust file with parsed content
    RustFile(RustFileContent),
}

pub trait FileWritable {
    fn write_to(&self, path: impl AsRef<Path>) -> Result<()>;
}

impl FileWritable for FileSystemNode {
    fn write_to(&self, path: impl AsRef<Path>) -> Result<()> {
        match self {
            Self::Directory(dir) => dir.write_to(path),
            Self::RustFile(file) => file.write_to(path),
        }
    }
}

impl FileSystemNode {
    pub fn name(&self) -> &str {
        match self {
            Self::Directory(dir) => dir.name(),
            Self::RustFile(file) => file.name(),
        }
    }

    pub fn path(&self) -> &PathBuf {
        match self {
            Self::Directory(dir) => dir.path(),
            Self::RustFile(file) => file.path(),
        }
    }

    /// Get all Rust files recursively from this node
    pub fn rust_files(&self) -> Vec<&NamedSourceItems> {
        match self {
            Self::Directory(dir) => dir
                .content()
                .iter()
                .flat_map(|child| child.rust_files())
                .collect(),
            Self::RustFile(file) => vec![file.content()],
        }
    }

    /// Get all directories recursively from this node
    pub fn directories(&self) -> Vec<&FileSystemNode> {
        match self {
            Self::Directory(dir) => {
                let mut dirs = vec![self];
                dirs.extend(dir.content().iter().flat_map(|child| child.directories()));
                dirs
            }
            Self::RustFile(..) => vec![],
        }
    }
}

impl PartialEq for FileSystemNode {
    fn eq(&self, other: &Self) -> bool {
        self.name() == other.name() && self.path() == other.path()
    }
}

impl Eq for FileSystemNode {}

impl PartialOrd for FileSystemNode {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for FileSystemNode {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let self_is_dir = matches!(self, FileSystemNode::Directory { .. });
        let other_is_dir = matches!(other, FileSystemNode::Directory { .. });

        match (self_is_dir, other_is_dir) {
            (true, false) => std::cmp::Ordering::Less, // Directories come first
            (false, true) => std::cmp::Ordering::Greater, // Files come after directories
            _ => self.name().cmp(other.name()),        // Same type: sort by name
        }
    }
}
