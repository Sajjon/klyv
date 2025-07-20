use std::path::PathBuf;

use bon::bon;

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
