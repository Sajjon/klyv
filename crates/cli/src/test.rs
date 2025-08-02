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
        .allow_git_dirty(true)
        .allow_git_staged(true)
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
    // d2_a.rs is now split into separate files - test each part
    assert_generated_file_snapshot(&out_path, "d0_a/d1_b/magic.rs", "magic_type_file");
    assert_generated_file_snapshot(&out_path, "d0_a/d1_b/ab_astruct_a.rs", "ab_astruct_a_file");
    assert_generated_file_snapshot(&out_path, "d0_a/d1_b/functions.rs", "d2_a_mixed_content");
    assert_generated_file_snapshot(&out_path, "d0_a/d1_a/d2_b.rs", "d2_b_global_magic");

    // Assert directory structure snapshot
    let dir_structure = collect_directory_structure(&out_path);
    insta::assert_yaml_snapshot!("generated_directory_structure", dir_structure);
}

/// Asserts that a generated file matches its snapshot
/// Using early return to handle file existence check immediately
fn assert_generated_file_snapshot(
    base_path: &std::path::Path,
    relative_path: &str,
    snapshot_name: &str,
) {
    let file_path = base_path.join(relative_path);

    // Early return if file doesn't exist - fail the test
    if !file_path.exists() {
        panic!("Expected file not found: {}", file_path.display());
    }

    // File exists - read content and assert snapshot
    let content = fs::read_to_string(&file_path)
        .unwrap_or_else(|_| panic!("Failed to read file: {}", file_path.display()));
    insta::assert_snapshot!(snapshot_name, content);
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

#[test]
fn test_lib_rs_special_case_handling() {
    let mut source_path = env::current_dir().unwrap();
    source_path.push("src/fixtures/lib_rs_special/lib.rs");

    // Use a temporary directory that will be cleaned up automatically
    let temp_dir = TempDir::new().unwrap();
    let out_path = temp_dir.path().to_path_buf();

    let input = Input::builder()
        .source(source_path.to_path_buf())
        .maybe_out(Some(out_path.clone()))
        .allow_git_dirty(true)
        .allow_git_staged(true)
        .build();

    let tree = run(input).unwrap();

    // Verify the tree structure
    let debug = format!("{:#?}", tree);
    insta::assert_snapshot!("lib_rs_tree_structure", debug);

    // Collect all generated files and their contents
    let mut files_content = std::collections::BTreeMap::new();
    collect_all_files_content(&out_path, &mut files_content);

    // Snapshot all file contents
    for (file_path, content) in &files_content {
        let snapshot_name = format!("lib_rs_generated_{}", file_path.replace(['/', '\\'], "_"));
        insta::assert_snapshot!(snapshot_name, content);
    }

    // Verify expected structure exists
    assert!(
        out_path.join("lib.rs").exists(),
        "lib.rs should be generated"
    );
    assert!(
        out_path.join("types").exists(),
        "types directory should be created"
    );
    assert!(
        out_path.join("logic").exists(),
        "logic directory should be created"
    );
    assert!(
        out_path.join("types/mod.rs").exists(),
        "types/mod.rs should be created"
    );
    assert!(
        out_path.join("logic/mod.rs").exists(),
        "logic/mod.rs should be created"
    );
    assert!(
        out_path.join("logic/functions.rs").exists(),
        "logic/functions.rs should be created"
    );

    // Verify some specific type files exist
    assert!(
        out_path.join("types/user.rs").exists(),
        "types/user.rs should be created"
    );
    assert!(
        out_path.join("types/role.rs").exists(),
        "types/role.rs should be created"
    );
    assert!(
        out_path.join("types/status.rs").exists(),
        "types/status.rs should be created"
    );
}

/// Recursively collects all .rs file contents from a directory
fn collect_all_files_content(
    base: &std::path::Path,
    files_content: &mut std::collections::BTreeMap<String, String>,
) {
    collect_files_content_recursive(base, base, files_content);
}

/// Helper function to recursively collect file contents
fn collect_files_content_recursive(
    base: &std::path::Path,
    current: &std::path::Path,
    files_content: &mut std::collections::BTreeMap<String, String>,
) {
    if let Ok(entries) = fs::read_dir(current) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && path.extension().is_some_and(|ext| ext == "rs") {
                if let Ok(relative) = path.strip_prefix(base) {
                    let relative_str = relative.to_string_lossy().to_string();
                    if let Ok(content) = fs::read_to_string(&path) {
                        files_content.insert(relative_str, content);
                    }
                }
            } else if path.is_dir() {
                collect_files_content_recursive(base, &path, files_content);
            }
        }
    }
}

#[test]
fn test_main_rs_special_case_handling() {
    let mut source_path = env::current_dir().unwrap();
    source_path.push("src/fixtures/main_rs_special/main.rs");

    // Use a temporary directory that will be cleaned up automatically
    let temp_dir = TempDir::new().unwrap();
    let out_path = temp_dir.path().to_path_buf();

    let input = Input::builder()
        .source(source_path.to_path_buf())
        .maybe_out(Some(out_path.clone()))
        .allow_git_dirty(true)
        .allow_git_staged(true)
        .build();

    let result = run(input);
    assert!(result.is_ok(), "Running klyv on main.rs should succeed");

    // Verify the expected file structure was created
    let main_rs_path = out_path.join("main.rs");
    let types_dir = out_path.join("types");
    let logic_dir = out_path.join("logic");
    let types_mod_rs = out_path.join("types/mod.rs");
    let logic_mod_rs = out_path.join("logic/mod.rs");

    assert!(main_rs_path.exists(), "main.rs should exist");
    assert!(types_dir.exists(), "types directory should exist");
    assert!(logic_dir.exists(), "logic directory should exist");
    assert!(types_mod_rs.exists(), "types/mod.rs should exist");
    assert!(logic_mod_rs.exists(), "logic/mod.rs should exist");

    // Individual snapshot tests for each generated file
    assert_generated_file_snapshot(&out_path, "main.rs", "main_rs_special_main");
    assert_generated_file_snapshot(&out_path, "types/mod.rs", "main_rs_special_types_mod");
    assert_generated_file_snapshot(&out_path, "logic/mod.rs", "main_rs_special_logic_mod");
    assert_generated_file_snapshot(
        &out_path,
        "logic/functions.rs",
        "main_rs_special_logic_functions",
    );

    // Check for individual type files in models directory
    assert_generated_file_snapshot(
        &out_path,
        "types/cli_config.rs",
        "main_rs_special_cli_config",
    );
    assert_generated_file_snapshot(
        &out_path,
        "types/argument_parser.rs",
        "main_rs_special_argument_parser",
    );
    assert_generated_file_snapshot(&out_path, "types/document.rs", "main_rs_special_document");
    assert_generated_file_snapshot(
        &out_path,
        "types/file_processor.rs",
        "main_rs_special_file_processor",
    );
    assert_generated_file_snapshot(
        &out_path,
        "types/processing_error.rs",
        "main_rs_special_processing_error",
    );
    assert_generated_file_snapshot(
        &out_path,
        "types/document_metadata.rs",
        "main_rs_special_document_metadata",
    );
    assert_generated_file_snapshot(
        &out_path,
        "types/document_metrics.rs",
        "main_rs_special_document_metrics",
    );
}

#[test]
fn test_impl_method_spacing_issue() {
    let mut source_path = env::current_dir().unwrap();
    source_path.push("src/fixtures/impl_spacing_test/lib.rs");

    // Use a temporary directory that will be cleaned up automatically
    let temp_dir = TempDir::new().unwrap();
    let out_path = temp_dir.path().to_path_buf();

    let input = Input::builder()
        .source(source_path.to_path_buf())
        .maybe_out(Some(out_path.clone()))
        .allow_git_dirty(true)
        .allow_git_staged(true)
        .build();

    let _tree = run(input).unwrap();

    // Check the my_struct.rs file which should have an impl block with multiple methods
    let my_struct_file_path = out_path.join("types/my_struct.rs");
    assert!(
        my_struct_file_path.exists(),
        "types/my_struct.rs should exist"
    );

    let content =
        fs::read_to_string(&my_struct_file_path).expect("Should be able to read my_struct.rs file");

    // This test should show that methods in impl blocks are missing blank lines between them
    // The impl MyStruct block has `new` and `display` methods that should be separated by blank lines
    insta::assert_snapshot!("impl_method_spacing_my_struct_file", content);

    // Also check my_enum.rs which has multiple methods that should be spaced
    let my_enum_file_path = out_path.join("types/my_enum.rs");
    assert!(my_enum_file_path.exists(), "types/my_enum.rs should exist");

    let enum_content =
        fs::read_to_string(&my_enum_file_path).expect("Should be able to read my_enum.rs file");

    // The impl MyEnum block has magic_self, magic_mut_self, and magic methods
    // that should be separated by blank lines
    insta::assert_snapshot!("impl_method_spacing_my_enum_file", enum_content);
}

#[test]
fn test_nested_file_splitting() {
    let mut source_path = env::current_dir().unwrap();
    source_path.push("src/fixtures/nested_structure_test/deeply_nested/a/b/c/d/foo.rs");

    // Use a temporary directory that will be cleaned up automatically
    let temp_dir = TempDir::new().unwrap();
    let out_path = temp_dir.path().to_path_buf();

    let input = Input::builder()
        .source(source_path.to_path_buf())
        .maybe_out(Some(out_path.clone()))
        .allow_git_dirty(true)
        .allow_git_staged(true)
        .build();

    let _result = run(input).unwrap();

    // Check that files are properly organized
    let functions_file = out_path.join("functions.rs");
    let deep_foo_file = out_path.join("deep_foo.rs");
    let deep_foobar_file = out_path.join("deep_foobar.rs");
    let mod_file = out_path.join("mod.rs");

    // Verify the files exist
    assert!(functions_file.exists(), "functions.rs should exist");
    assert!(deep_foo_file.exists(), "deep_foo.rs should exist");
    assert!(deep_foobar_file.exists(), "deep_foobar.rs should exist");
    assert!(mod_file.exists(), "mod.rs should exist");

    // Check contents
    let functions_content =
        fs::read_to_string(&functions_file).expect("Should be able to read functions.rs");
    insta::assert_snapshot!("nested_file_splitting_functions", functions_content);

    let deep_foo_content =
        fs::read_to_string(&deep_foo_file).expect("Should be able to read deep_foo.rs");
    insta::assert_snapshot!("nested_file_splitting_deep_foo", deep_foo_content);

    let mod_content = fs::read_to_string(&mod_file).expect("Should be able to read mod.rs");
    insta::assert_snapshot!("nested_file_splitting_mod", mod_content);
}
