use std::{fs, path::PathBuf};

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

#[derive(Clone, Debug, Getters, Builder)]
pub struct NodeContent<C> {
    #[getset(get = "pub")]
    name: String,
    #[getset(get = "pub")]
    path: PathBuf,
    #[getset(get = "pub")]
    content: C,
}
pub type DirectoryContent = NodeContent<Vec<FileSystemNode>>;
pub type RustFileContent = NodeContent<NamedSourceItems>;

/// A file system node that can be either a directory or a Rust file
#[derive(Clone, Debug)]
pub enum FileSystemNode {
    /// A directory containing other nodes
    Directory(DirectoryContent),
    /// A Rust file with parsed content
    RustFile(RustFileContent),
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

/// Main entry point - recursively find and parse all Rust files in a directory
#[bon::builder]
pub fn find_in(path: impl AsRef<std::path::Path>) -> Result<FileSystemNode> {
    let path = path.as_ref().to_path_buf();

    if !path.exists() {
        return Err(Error::bail(format!(
            "Path does not exist: {}",
            path.display()
        )));
    }

    if path.is_file() {
        return parse_rust_file(path);
    }

    if path.is_dir() {
        return scan_directory(path);
    }

    Err(Error::bail(format!(
        "Invalid path type: {}",
        path.display()
    )))
}

/// Parse a single Rust file
fn parse_rust_file(path: PathBuf) -> Result<FileSystemNode> {
    let name = path
        .file_name()
        .ok_or_else(|| Error::bail("Invalid file name"))?
        .to_string_lossy()
        .to_string();

    // Only process .rs files
    if path.extension().is_none_or(|ext| ext != "rs") {
        return Err(Error::bail(format!("Not a Rust file: {}", path.display())));
    }

    let content = fs::read_to_string(&path)
        .map_err(|e| Error::bail(format!("Failed to read file {}: {}", path.display(), e)))?;

    let items = parse_file().content(content).call()?;

    let named_items = NamedSourceItems::builder()
        .name(name.clone())
        .items(items)
        .build();

    let rust_file_content = NodeContent::builder()
        .name(name)
        .path(path)
        .content(named_items)
        .build();

    Ok(FileSystemNode::RustFile(rust_file_content))
}

/// Scan a directory recursively using DFS
fn scan_directory(path: PathBuf) -> Result<FileSystemNode> {
    let name = path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    let entries = fs::read_dir(&path).map_err(|e| {
        Error::bail(format!(
            "Failed to read directory {}: {}",
            path.display(),
            e
        ))
    })?;

    let mut children = entries
        .filter_map(|entry| {
            let entry = match entry {
                Ok(e) => e,
                Err(e) => {
                    eprintln!("Warning: Failed to read directory entry: {}", e);
                    return None;
                }
            };
            let entry_path = entry.path();

            if entry_path.is_dir() {
                scan_directory(entry_path).ok()
            } else if entry_path.is_file() && entry_path.extension().is_some_and(|ext| ext == "rs")
            {
                parse_rust_file(entry_path).ok()
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    children.sort();

    let directory_content = NodeContent::builder()
        .name(name)
        .path(path)
        .content(children)
        .build();

    Ok(FileSystemNode::Directory(directory_content))
}

/// Parse a file content string into SourceItems
#[bon::builder]
fn parse_file(content: String) -> Result<Vec<SourceItem>> {
    let parsed_file = syn::parse_file(&content)
        .map_err(|e| Error::bail(format!("Failed to parse Rust syntax: {}", e)))?;

    parsed_file
        .items
        .into_iter()
        .map(SourceItem::try_from)
        .collect::<Result<Vec<SourceItem>>>()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_rust_file() {
        let content = r#"
            pub struct TestStruct {
                field: u32,
            }
            
            impl TestStruct {
                pub fn new() -> Self {
                    Self { field: 0 }
                }
            }
        "#;

        let items = parse_file().content(content.to_string()).call().unwrap();
        assert_eq!(items.len(), 2); // struct + impl
    }

    #[test]
    fn test_file_system_node_methods() {
        let named_items = NamedSourceItems::builder()
            .name("test.rs".to_string())
            .items(vec![])
            .build();

        let rust_file_content = NodeContent::builder()
            .name("test.rs".to_string())
            .path(PathBuf::from("/test.rs"))
            .content(named_items)
            .build();

        let rust_file = FileSystemNode::RustFile(rust_file_content);

        assert_eq!(rust_file.name(), "test.rs");
        assert_eq!(rust_file.rust_files().len(), 1);
        assert_eq!(rust_file.directories().len(), 0);
    }
}

#[cfg(test)]
mod extensive_tests {
    use super::*;
    use std::fs::{self, File};
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_parse_valid_rust_file() {
        let content = r#"
            fn foo() {}
            struct Bar;
        "#;

        let result = parse_file().content(content.to_string()).call();
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 2);
    }

    #[test]
    fn test_parse_invalid_rust_file() {
        let content = "invalid rust syntax {}}";
        let result = parse_file().content(content.to_string()).call();
        assert!(result.is_err());
    }

    #[test]
    fn test_find_in_file_path() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.rs");
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "fn main() {{}}").unwrap();

        let node = find_in().path(&file_path).call().unwrap();
        assert_eq!(node.name(), "test.rs");
        assert_eq!(node.rust_files().len(), 1);
    }

    #[test]
    fn test_find_in_directory() {
        let dir = tempdir().unwrap();
        let file1 = dir.path().join("a.rs");
        let file2 = dir.path().join("b.txt");
        let subdir = dir.path().join("subdir");
        fs::create_dir(&subdir).unwrap();
        let file3 = subdir.join("c.rs");

        File::create(&file1)
            .unwrap()
            .write_all(b"struct A;")
            .unwrap();
        File::create(&file2)
            .unwrap()
            .write_all(b"not rust")
            .unwrap();
        File::create(&file3)
            .unwrap()
            .write_all(b"fn sub() {}")
            .unwrap();

        let node = find_in().path(dir.path()).call().unwrap();
        assert!(matches!(node, FileSystemNode::Directory { .. }));
        assert_eq!(node.rust_files().len(), 2);
        assert_eq!(node.directories().len(), 2); // root + subdir
    }

    #[test]
    fn test_empty_directory() {
        let dir = tempdir().unwrap();
        let node = find_in().path(dir.path()).call().unwrap();
        assert!(matches!(node, FileSystemNode::Directory { .. }));
        assert_eq!(node.rust_files().len(), 0);
    }

    #[test]
    fn test_filesystem_node_ordering() {
        // Test that directories come before files and items are sorted by name
        let dir1 = FileSystemNode::Directory(
            NodeContent::builder()
                .name("b_dir".to_string())
                .path(PathBuf::from("/b_dir"))
                .content(vec![])
                .build(),
        );
        let dir2 = FileSystemNode::Directory(
            NodeContent::builder()
                .name("a_dir".to_string())
                .path(PathBuf::from("/a_dir"))
                .content(vec![])
                .build(),
        );
        let file1 = FileSystemNode::RustFile(
            NodeContent::builder()
                .name("z_file.rs".to_string())
                .path(PathBuf::from("/z_file.rs"))
                .content(
                    NamedSourceItems::builder()
                        .name("z_file.rs".to_string())
                        .items(vec![])
                        .build(),
                )
                .build(),
        );
        let file2 = FileSystemNode::RustFile(
            NodeContent::builder()
                .name("a_file.rs".to_string())
                .path(PathBuf::from("/a_file.rs"))
                .content(
                    NamedSourceItems::builder()
                        .name("a_file.rs".to_string())
                        .items(vec![])
                        .build(),
                )
                .build(),
        );

        let mut nodes = vec![file1, dir1, file2, dir2];
        nodes.sort();

        // Should be: a_dir, b_dir, a_file.rs, z_file.rs
        assert_eq!(nodes[0].name(), "a_dir");
        assert_eq!(nodes[1].name(), "b_dir");
        assert_eq!(nodes[2].name(), "a_file.rs");
        assert_eq!(nodes[3].name(), "z_file.rs");
    }
}
