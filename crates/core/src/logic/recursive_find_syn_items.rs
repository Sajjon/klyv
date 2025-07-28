use crate::prelude::*;
use log::warn;

#[allow(unused_imports)]
use std::process::Command;

/// Input for the CLI, containing the source path and optional output path
#[derive(Clone, Debug, Builder, Getters)]
pub struct Input {
    #[getset(get = "pub")]
    source: PathBuf,
    /// If None, same dir as `source` will be used
    #[getset(get = "pub")]
    out: Option<PathBuf>,
    #[getset(get = "pub")]
    allow_git_staged: bool,
    #[getset(get = "pub")]
    allow_git_dirty: bool,
}

#[bon::builder]
pub fn split(input: Input) -> Result<FileSystemNode> {
    #[cfg(not(debug_assertions))]
    ensure_git_status_clean(*input.allow_git_staged(), *input.allow_git_dirty())?;
    let out = input.out().as_ref().unwrap_or(input.source());
    do_split().source(input.source()).out(out).call()
}

#[allow(dead_code)]
fn ensure_git_status_clean(allow_git_staged: bool, allow_git_dirty: bool) -> Result<()> {
    // Dirty is a superset of staged, so if we allow dirty, we also allow staged
    let allow_git_staged = allow_git_staged || allow_git_dirty;

    // Check if git is available and we're in a git repository
    let git_dir_check = Command::new("git")
        .args(["rev-parse", "--git-dir"])
        .output();

    let git_dir_output = match git_dir_check {
        Ok(output) => output,
        Err(_) => {
            // Git is not available, proceed without checking
            warn!("Git is not available, skipping repository status check");
            return Ok(());
        }
    };

    if !git_dir_output.status.success() {
        // Not in a git repository, that's fine
        return Ok(());
    }

    // We're in a git repository, check for uncommitted changes
    let status_output = Command::new("git")
        .args(["status", "--porcelain"])
        .output()
        .map_err(|e| Error::bail(format!("Failed to execute git status: {}", e)))?;

    if !status_output.status.success() {
        return Err(Error::bail("Git status command failed"));
    }

    let status_stdout = String::from_utf8_lossy(&status_output.stdout);

    if status_stdout.trim().is_empty() {
        // Working tree is clean
        return Ok(());
    }

    // Parse git status output to distinguish between staged and unstaged changes
    let mut has_staged_changes = false;
    let mut has_unstaged_changes = false;

    for line in status_stdout.lines() {
        if line.len() < 2 {
            continue;
        }

        let index_status = line.chars().next().unwrap_or(' ');
        let worktree_status = line.chars().nth(1).unwrap_or(' ');

        // Check for staged changes (index status is not ' ' or '?')
        if index_status != ' ' && index_status != '?' {
            has_staged_changes = true;
        }

        // Check for unstaged changes (worktree status is not ' ')
        if worktree_status != ' ' {
            has_unstaged_changes = true;
        }
    }

    // Build error message based on what changes exist and what's allowed
    let mut error_parts = Vec::new();

    if has_staged_changes && !allow_git_staged {
        error_parts.push("staged changes");
    }

    if has_unstaged_changes && !allow_git_dirty {
        error_parts.push("unstaged changes");
    }

    if !error_parts.is_empty() {
        let changes_desc = error_parts.join(" and ");
        return Err(Error::bail(format!(
            "The git repository has {}. Please commit or stash your changes before running this command.",
            changes_desc
        )));
    }

    Ok(())
}

#[bon::builder]
fn do_split(source: impl AsRef<Path>, out: impl AsRef<Path>) -> Result<FileSystemNode> {
    let node = find_in().path(source).call()?;
    write().node(node.clone()).out(out).call()?;
    Ok(node)
}

/// This function splits all the Rust types identified in the given path into separate files if the type is supported - see `enum SourceItem` for list of supported types.
/// The `Unsplittable` and `Verbatim` types are not split, as they do not have
/// a corresponding file type - they will be put in files named `unsplittable_0.rs`,
/// `unsplittable_1.rs` and `verbatim_0.rs`, `verbatim_1.rs` etc, where the index
/// is the order in which they were found according to the files in that folder.
/// I.e. if `foo.rs`, `bar.rs` and `baz.rs` are found in the same directory and
///  each of these three files contain any unsplittable items, they will be put in `unsplittable_0.rs`, `unsplittable_1.rs` and `unsplittable_2.rs` respectively.
/// If two types with the same name are found e.g. `enum Foo` in `foo.rs` and
/// `bar.rs`, the first one will be put in `foo_0.rs` and the second one in `bar_0.rs`.
///
/// For types which has `impl` blocks the impl blocks will be moved to the same
/// file as the type they implement.
#[bon::builder]
fn write(node: FileSystemNode, out: impl AsRef<Path>) -> Result<()> {
    node.write_to(out.as_ref())
}

#[bon::builder]
fn find_in(path: impl AsRef<std::path::Path>) -> Result<FileSystemNode> {
    let path = path.as_ref().to_path_buf();

    validate_path_exists(&path)?;
    determine_path_type_and_parse(path)
}

/// Validates that the given path exists
fn validate_path_exists(path: &Path) -> Result<()> {
    if !path.exists() {
        return Err(Error::bail(format!(
            "Path does not exist: {}",
            path.display()
        )));
    }
    Ok(())
}

/// Determines path type (file/directory) and calls appropriate parser
fn determine_path_type_and_parse(path: PathBuf) -> Result<FileSystemNode> {
    if path.is_file() {
        return parse_rust_file().path(path).call();
    }

    if path.is_dir() {
        return scan_directory().path(path).call();
    }

    Err(Error::bail(format!(
        "Invalid path type: {}",
        path.display()
    )))
}

/// Parse a single Rust file
#[bon::builder]
fn parse_rust_file(path: PathBuf) -> Result<FileSystemNode> {
    let name = extract_file_name(&path)?;
    validate_rust_file_extension(&path)?;

    let content = read_file_content(&path)?;
    let items = parse_file().content(content).call()?;

    create_rust_file_node(name, path, items)
}

/// Extracts the file name from a path
fn extract_file_name(path: &Path) -> Result<String> {
    let name = path
        .file_name()
        .ok_or_else(|| Error::bail("Invalid file name"))?
        .to_string_lossy()
        .to_string();
    Ok(name)
}

/// Validates that the file has a .rs extension
fn validate_rust_file_extension(path: &Path) -> Result<()> {
    if path.extension().is_none_or(|ext| ext != "rs") {
        return Err(Error::bail(format!("Not a Rust file: {}", path.display())));
    }
    Ok(())
}

/// Reads the content of a file as a string
fn read_file_content(path: &Path) -> Result<String> {
    fs::read_to_string(path)
        .map_err(|e| Error::bail(format!("Failed to read file {}: {}", path.display(), e)))
}

/// Creates a FileSystemNode::RustFile from parsed components
fn create_rust_file_node(
    name: String,
    path: PathBuf,
    items: Vec<SourceItem>,
) -> Result<FileSystemNode> {
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
#[bon::builder]
fn scan_directory(path: PathBuf) -> Result<FileSystemNode> {
    let name = extract_directory_name(&path);
    let entries = read_directory_entries(&path)?;

    let mut children = process_directory_entries(entries);
    children.sort();

    create_directory_node(name, path, children)
}

/// Extracts the directory name from a path
fn extract_directory_name(path: &Path) -> String {
    path.file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string()
}

/// Reads directory entries with error handling
fn read_directory_entries(path: &Path) -> Result<fs::ReadDir> {
    fs::read_dir(path).map_err(|e| {
        Error::bail(format!(
            "Failed to read directory {}: {}",
            path.display(),
            e
        ))
    })
}

/// Processes directory entries and filters for valid Rust files and subdirectories
fn process_directory_entries(entries: fs::ReadDir) -> Vec<FileSystemNode> {
    entries.filter_map(process_single_directory_entry).collect()
}

/// Processes a single directory entry
fn process_single_directory_entry(
    entry: Result<fs::DirEntry, std::io::Error>,
) -> Option<FileSystemNode> {
    let entry = handle_directory_entry_error(entry)?;
    let entry_path = entry.path();

    classify_and_parse_path(entry_path)
}

/// Handles errors when reading directory entries
fn handle_directory_entry_error(
    entry: Result<fs::DirEntry, std::io::Error>,
) -> Option<fs::DirEntry> {
    match entry {
        Ok(e) => Some(e),
        Err(e) => {
            warn!("Warning: Failed to read directory entry: {}", e);
            None
        }
    }
}

/// Classifies path type and calls appropriate parser
fn classify_and_parse_path(entry_path: PathBuf) -> Option<FileSystemNode> {
    if entry_path.is_dir() {
        scan_directory().path(entry_path).call().ok()
    } else if is_rust_file(&entry_path) {
        parse_rust_file().path(entry_path).call().ok()
    } else {
        None
    }
}

/// Checks if a path is a Rust file
fn is_rust_file(path: &Path) -> bool {
    path.is_file() && path.extension().is_some_and(|ext| ext == "rs")
}

/// Creates a FileSystemNode::Directory from components
fn create_directory_node(
    name: String,
    path: PathBuf,
    children: Vec<FileSystemNode>,
) -> Result<FileSystemNode> {
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
