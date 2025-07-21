use std::{fs::DirEntry, path::PathBuf};

use crate::prelude::*;

#[derive(Clone, Debug)]
pub enum NodeContent {
    Directory {
        /// Contents of dir, either directories or files, if any.
        children: Vec<Box<Node>>,
    },
    File {
        content: Vec<SourceItem>,
    },
}
impl From<NamedSourceItems> for NodeContent {
    fn from(value: NamedSourceItems) -> Self {
        Self::File {
            content: value.items,
        }
    }
}

#[derive(Clone, Debug, Getters, Builder)]
pub struct Node {
    /// Name of dir or file
    #[getset(get = "pub")]
    name: String,

    #[getset(get = "pub")]
    absolute_path: PathBuf,

    /// Contents of this node, if this node is a dir, it contains
    /// the children if any. If it is a file it contains its contents
    #[getset(get = "pub")]
    content: NodeContent,
}

impl From<(PathBuf, NamedSourceItems)> for Node {
    fn from((parent_dir, item): (PathBuf, NamedSourceItems)) -> Self {
        let name = item.name.clone();
        let mut absolute_path = parent_dir.clone();
        absolute_path.push(&name);
        Self::builder()
            .absolute_path(absolute_path)
            .name(name)
            .content(NodeContent::from(item))
            .build()
    }
}

#[derive(Clone, Getters, Builder)]
pub struct NamedSourceItems {
    #[getset(get = "pub")]
    items: Vec<SourceItem>,

    /// Name of the file
    #[getset(get = "pub")]
    name: String,
}

#[bon]
impl Node {
    #[builder]
    pub fn add_child(&mut self, items: NamedSourceItems) {
        let parent_dir = self.absolute_path().clone();
        match &mut self.content {
            NodeContent::Directory { children } => {
                children.push(Box::new(Node::from((parent_dir, items))));
            }
            NodeContent::File { content: _ } => {
                panic!("A file cannot have a child, incorrect implementation")
            }
        }
    }

    /// Add a pre-built node as a child (useful for directories)
    pub fn add_child_node(&mut self, child_node: Node) {
        match &mut self.content {
            NodeContent::Directory { children } => {
                children.push(Box::new(child_node));
            }
            NodeContent::File { content: _ } => {
                panic!("A file cannot have a child, incorrect implementation")
            }
        }
    }
}

#[derive(Clone, Debug)]
pub enum Tree {
    Root { path: PathBuf },
    Node(Node),
}

impl Tree {
    fn add_child_item(&mut self, child_items: NamedSourceItems) {
        match self {
            Self::Root { path } => {
                // When converting from Root to Node, we need to determine if this is a directory or file
                if path.is_dir() {
                    // Create a directory node
                    let name = path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string();
                    let mut directory_node = Node::builder()
                        .name(name)
                        .absolute_path(path.clone())
                        .content(NodeContent::Directory { children: vec![] })
                        .build();

                    // Add the child to this directory
                    directory_node.add_child().items(child_items).call();
                    *self = Self::Node(directory_node);
                } else {
                    // This is a file, convert directly
                    *self = Self::Node(Node::from((
                        path.parent().unwrap_or(path).to_path_buf(),
                        child_items,
                    )));
                }
            }
            Self::Node(node) => node.add_child().items(child_items).call(),
        }
    }

    fn add_child_node(&mut self, child_node: Node) {
        match self {
            Self::Root { path } => {
                // Create a directory node for the root if it doesn't exist
                let name = path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                let mut directory_node = Node::builder()
                    .name(name)
                    .absolute_path(path.clone())
                    .content(NodeContent::Directory { children: vec![] })
                    .build();
                directory_node.add_child_node(child_node);
                *self = Self::Node(directory_node);
            }
            Self::Node(node) => {
                node.add_child_node(child_node);
            }
        }
    }

    fn path(&self) -> PathBuf {
        self.dir().or(self.file()).expect("Either file or dir")
    }

    fn dir(&self) -> Option<PathBuf> {
        match self {
            Self::Root { path } => {
                if path.is_dir() {
                    Some(path.clone())
                } else {
                    None
                }
            }
            Self::Node(node) => {
                let path = node.absolute_path();
                if path.is_dir() {
                    Some(path.clone())
                } else {
                    None
                }
            }
        }
    }

    fn file(&self) -> Option<PathBuf> {
        match self {
            Self::Root { path } => {
                if path.is_dir() {
                    None
                } else {
                    Some(path.clone())
                }
            }
            Self::Node(node) => {
                let path = node.absolute_path();
                if path.is_dir() {
                    None
                } else {
                    Some(path.clone())
                }
            }
        }
    }
}

pub fn _find_in(tree: &mut Tree) -> Result<()> {
    println!("find in {}", tree.path().display());
    if tree.dir().is_some() {
        info!("Tree is dir");
        find_files_in().tree(tree).call()
    } else {
        info!("Tree is file");
        parse_and_append_contents_of_file().tree(tree).call()
    }
}

#[builder]
fn find_files_in(tree: &mut Tree) -> Result<()> {
    let path = tree.dir().ok_or(Error::bail("Expected dir, found file"))?;

    // Read directory entries and collect them immediately to close the ReadDir handle
    let entries: Vec<DirEntry> = read_dir()
        .path(&path)
        .call()?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(Error::from)?;

    // Process each entry
    for entry in entries {
        let entry_path = entry.path();
        let file_name = entry_path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string();

        if entry_path.is_dir() {
            // For directories, recursively process them
            let subtree = find_in().path(&entry_path).call()?;

            // Convert the processed subtree to a proper directory structure
            match subtree {
                Tree::Node(processed_node) => {
                    // The processed_node contains the directory structure
                    // We need to add this entire subtree as a child
                    // For now, we'll use a workaround since our API is limited

                    // Create a directory entry using the processed node's structure
                    let child_node = Node::builder()
                        .name(file_name)
                        .absolute_path(entry_path.clone())
                        .content(processed_node.content().clone())
                        .build();

                    // Add this as a directory by creating a proper child node
                    tree.add_child_node(child_node);
                }
                Tree::Root { .. } => {
                    // Empty directory, create empty directory node
                    let dir_items = NamedSourceItems::builder()
                        .name(file_name)
                        .items(vec![])
                        .build();
                    tree.add_child_item(dir_items);
                }
            }
        } else if entry_path.extension().is_some_and(|ext| ext == "rs") {
            // Only process Rust files
            let source_items = find_file_in_dir().path(&entry_path).call()?;
            tree.add_child_item(source_items);
        }
        // Skip non-Rust files
    }
    Ok(())
}

#[builder]
fn parse_and_append_contents_of_file(tree: &mut Tree) -> Result<()> {
    let file_path = tree.file().expect("Should be file if not dir");

    // Only process Rust files
    if file_path.extension().is_none_or(|ext| ext != "rs") {
        return Ok(());
    }

    let source_items = find_file_in_dir().path(&file_path).call()?;
    tree.add_child_item(source_items);
    Ok(())
}

#[builder]
fn find_file_in_dir(path: impl AsRef<Path>) -> Result<NamedSourceItems> {
    let path = path.as_ref();
    if path.is_dir() {
        return Err(Error::bail("Expected file, found dir"));
    }
    let content = read_to_string().path(path).call()?;
    let file_name = path
        .file_name()
        .unwrap()
        .to_os_string()
        .into_string()
        .unwrap();
    let parsed_file = parse_file().content(&content).call()?;
    let items = analyze_file().file(parsed_file).call()?;
    Ok(NamedSourceItems::builder()
        .name(file_name)
        .items(items)
        .build())
}
