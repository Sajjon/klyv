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

/// A file system node that can be either a directory or a Rust file
#[derive(Clone, Debug)]
pub enum FileSystemNode {
    /// A directory containing other nodes
    Directory {
        name: String,
        path: PathBuf,
        children: Vec<FileSystemNode>,
    },
    /// A Rust file with parsed content
    RustFile {
        name: String,
        path: PathBuf,
        content: NamedSourceItems,
    },
}

impl FileSystemNode {
    pub fn name(&self) -> &str {
        match self {
            Self::Directory { name, .. } => name,
            Self::RustFile { name, .. } => name,
        }
    }

    pub fn path(&self) -> &PathBuf {
        match self {
            Self::Directory { path, .. } => path,
            Self::RustFile { path, .. } => path,
        }
    }

    /// Get all Rust files recursively from this node
    pub fn rust_files(&self) -> Vec<&NamedSourceItems> {
        match self {
            Self::Directory { children, .. } => children
                .iter()
                .flat_map(|child| child.rust_files())
                .collect(),
            Self::RustFile { content, .. } => vec![content],
        }
    }

    /// Get all directories recursively from this node
    pub fn directories(&self) -> Vec<&FileSystemNode> {
        match self {
            Self::Directory { children, .. } => {
                let mut dirs = vec![self];
                dirs.extend(children.iter().flat_map(|child| child.directories()));
                dirs
            }
            Self::RustFile { .. } => vec![],
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

    Ok(FileSystemNode::RustFile {
        name,
        path,
        content: named_items,
    })
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

    let mut children = Vec::new();

    for entry in entries {
        let entry =
            entry.map_err(|e| Error::bail(format!("Failed to read directory entry: {}", e)))?;

        let entry_path = entry.path();

        if entry_path.is_dir() {
            // Recursively scan subdirectory
            match scan_directory(entry_path) {
                Ok(child_node) => children.push(child_node),
                Err(e) => {
                    // Log error but continue processing other entries
                    eprintln!("Warning: Failed to scan directory: {}", e);
                }
            }
        } else if entry_path.is_file() && entry_path.extension().is_some_and(|ext| ext == "rs") {
            // Parse Rust file
            match parse_rust_file(entry_path) {
                Ok(file_node) => children.push(file_node),
                Err(e) => {
                    // Log error but continue processing other files
                    eprintln!("Warning: Failed to parse Rust file: {}", e);
                }
            }
        }
        // Skip non-Rust files silently
    }

    // Sort children for consistent output (directories first, then files, both alphabetically)
    children.sort_by(|a, b| match (a, b) {
        (
            FileSystemNode::Directory { name: a_name, .. },
            FileSystemNode::Directory { name: b_name, .. },
        ) => a_name.cmp(b_name),
        (
            FileSystemNode::RustFile { name: a_name, .. },
            FileSystemNode::RustFile { name: b_name, .. },
        ) => a_name.cmp(b_name),
        (FileSystemNode::Directory { .. }, FileSystemNode::RustFile { .. }) => {
            std::cmp::Ordering::Less
        }
        (FileSystemNode::RustFile { .. }, FileSystemNode::Directory { .. }) => {
            std::cmp::Ordering::Greater
        }
    });

    Ok(FileSystemNode::Directory {
        name,
        path,
        children,
    })
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
        let rust_file = FileSystemNode::RustFile {
            name: "test.rs".to_string(),
            path: PathBuf::from("/test.rs"),
            content: NamedSourceItems::builder()
                .name("test.rs".to_string())
                .items(vec![])
                .build(),
        };

        assert_eq!(rust_file.name(), "test.rs");
        assert_eq!(rust_file.rust_files().len(), 1);
        assert_eq!(rust_file.directories().len(), 0);
    }
}
