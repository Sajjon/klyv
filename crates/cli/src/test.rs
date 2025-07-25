#![cfg(test)]

use std::{env, fs};
use tempfile::TempDir;
use test_log::test;

use crate::{Input, run};

#[test]
fn test() {
    let mut source_path = env::current_dir().unwrap();
    source_path.push("src/fixtures/fixt0");

    // Use a temporary directory that will be cleaned up automatically
    let temp_dir = TempDir::new().unwrap();
    let out_path = temp_dir.path().to_path_buf();

    let input = Input::builder()
        .source(source_path.to_path_buf())
        .maybe_out(Some(out_path.clone()))
        .build();

    let tree = run(input).unwrap();

    // Verify the tree contains expected items (quick smoke test)
    let debug = format!("{:#?}", tree);
    assert!(debug.contains("AaaaStructB"));
    assert!(debug.contains("global_gen_magic"));
    assert!(debug.contains("AbAStructA"));

    // Create snapshots of key generated files (now split into separate files with snake_case names)
    assert_generated_file_snapshot(
        &out_path,
        "d0_a/d1_a/d2_a/aaaa_struct_a.rs",
        "aaaa_struct_a_file",
    );
    assert_generated_file_snapshot(
        &out_path,
        "d0_a/d1_a/d2_a/aaaa_struct_b.rs",
        "aaaa_struct_b_file",
    );
    assert_generated_file_snapshot(&out_path, "d0_a/d1_a/d2_a/aaaa_enum.rs", "aaaa_enum_file");
    assert_generated_file_snapshot(
        &out_path,
        "d0_a/d1_a/d2_a/magic_trait.rs",
        "magic_trait_file",
    );
    assert_generated_file_snapshot(&out_path, "d0_a/d1_b/d2_a.rs", "d2_a_mixed_content");
    assert_generated_file_snapshot(&out_path, "d0_a/d1_a/d2_b.rs", "d2_b_global_magic");

    // Assert directory structure snapshot
    let dir_structure = collect_directory_structure(&out_path);
    insta::assert_yaml_snapshot!("generated_directory_structure", dir_structure);
}

fn assert_generated_file_snapshot(
    base_path: &std::path::Path,
    relative_path: &str,
    snapshot_name: &str,
) {
    let file_path = base_path.join(relative_path);
    if file_path.exists() {
        let content = fs::read_to_string(&file_path)
            .unwrap_or_else(|_| panic!("Failed to read file: {}", file_path.display()));
        insta::assert_snapshot!(snapshot_name, content);
    } else {
        panic!("Expected file not found: {}", file_path.display());
    }
}

fn collect_directory_structure(dir: &std::path::Path) -> Vec<String> {
    let mut files = Vec::new();
    collect_files_recursive(dir, dir, &mut files);
    files.sort();
    files
}

fn collect_files_recursive(
    base: &std::path::Path,
    current: &std::path::Path,
    files: &mut Vec<String>,
) {
    if let Ok(entries) = fs::read_dir(current) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && path.extension().is_some_and(|ext| ext == "rs") {
                if let Ok(relative) = path.strip_prefix(base) {
                    files.push(relative.to_string_lossy().to_string());
                }
            } else if path.is_dir() {
                collect_files_recursive(base, &path, files);
            }
        }
    }
}
