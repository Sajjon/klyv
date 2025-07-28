use crate::prelude::*;

/// Type alias for a Rust file content, which is a NodeContent containing NamedSourceItems
pub type RustFileContent = NodeContent<NamedSourceItems>;

impl FileWritable for RustFileContent {
    /// Writes Rust file content to one or more files based on item types.
    ///
    /// Types are split into separate files with their implementations,
    /// while keeping use statements and global items appropriately distributed.
    fn write_to(&self, path: impl AsRef<Path>) -> Result<()> {
        let base_path = path.as_ref();

        // Ensure output directory exists
        self.ensure_output_directory_exists(base_path)?;

        self.handle_file_writing_strategy(base_path)?;

        Ok(())
    }
}

/// Helper struct for doc attribute conversion results
pub struct DocConversion {
    pub converted_text: String,
    pub next_position: usize,
}

/// Helper struct for doc attribute content parsing
pub struct DocContent {
    pub content: String,
    pub end_position: usize,
}

impl RustFileContent {
    // pub constants for file names and extensions
    pub const MAIN_RS: &'static str = "main.rs";
    pub const LIB_RS: &'static str = "lib.rs";
    pub const MOD_RS: &'static str = "mod.rs";
    pub const UTILS_RS: &'static str = "utils.rs";
    pub const FUNCTIONS_RS: &'static str = "functions.rs";
    pub const RS_EXTENSION: &'static str = ".rs";

    // pub constants for categories
    pub const CATEGORY_MAIN: &'static str = "main";
    pub const CATEGORY_LOGIC: &'static str = "logic";
    pub const CATEGORY_TYPES: &'static str = "types";
    pub const CATEGORY_CLI: &'static str = "cli";
    pub const CATEGORY_CORE: &'static str = "core";

    // pub constants for folder names
    pub const FOLDER_TYPES: &'static str = "types";
    pub const FOLDER_LOGIC: &'static str = "logic";

    // pub constants for module names
    pub const MAIN_FUNCTION: &'static str = "main";
    pub const UTILS_MODULE: &'static str = "utils";
    pub const FUNCTIONS_MODULE: &'static str = "functions";

    // pub constants for common patterns
    pub const DOC_ATTR_PREFIX: &'static str = "[doc = \"";
    pub const MOD_PREFIX: &'static str = "mod ";
    pub const COMMENT_PREFIX: &'static str = "///";
    pub const PRELUDE_IMPORT: &'static str = "use crate::prelude::*;\n\n";

    /// Determines and executes the appropriate file writing strategy
    fn handle_file_writing_strategy(&self, base_path: &Path) -> Result<()> {
        // Check if this is a special lib.rs case
        if self.is_lib_rs_special_case() {
            debug!("Detected lib.rs special case");
            self.handle_lib_rs_special_case(base_path)?;
        } else if self.is_main_rs_special_case() {
            debug!("Detected main.rs special case");
            self.handle_main_rs_special_case(base_path)?;
        } else {
            self.handle_standard_file_splitting(base_path)?;
        }
        Ok(())
    }

    /// Handles standard file splitting logic for regular files
    fn handle_standard_file_splitting(&self, base_path: &Path) -> Result<()> {
        // Standard file splitting logic
        let items = self.content().items();
        let grouped_items = self.group_items_by_target_file(items);

        debug!("Found {} item groups", grouped_items.len());

        self.process_and_write_grouped_items(base_path)?;

        // Update mod.rs if multiple files were created
        if grouped_items.len() > 1 {
            debug!("Multiple files created, updating mod.rs");
            self.update_mod_file(base_path, &grouped_items)?;
        } else {
            debug!("Only one file, skipping mod.rs update");
        }
        Ok(())
    }

    /// Creates the output directory if it doesn't exist
    fn ensure_output_directory_exists(&self, base_path: &Path) -> Result<()> {
        if let Some(parent) = base_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| Error::bail(format!("Failed to create directory: {}", e)))?;
        }
        Ok(())
    }

    /// Determines the output directory based on the base path
    /// If base_path is a file, use its parent directory; if it's a directory, use it directly
    pub(super) fn determine_output_directory(&self, base_path: &Path) -> PathBuf {
        if base_path.is_file() {
            base_path.parent().unwrap_or(Path::new(".")).to_path_buf()
        } else {
            base_path.to_path_buf()
        }
    }

    /// Groups items by target file and writes each group to its file
    fn process_and_write_grouped_items(&self, base_path: &Path) -> Result<()> {
        let items = self.content().items();
        let grouped_items = self.group_items_by_target_file(items);

        // Write each group to its corresponding file
        for (file_name, group_items) in grouped_items {
            let target_file = self.determine_target_file_path(base_path, &file_name);
            self.write_items_to_file(&group_items, &target_file)?;
        }
        Ok(())
    }

    /// Determines the target file path for a given file name
    fn determine_target_file_path(&self, base_path: &Path, file_name: &str) -> PathBuf {
        if file_name == *self.content().name() {
            // Use the original path for items that stay in the main file
            self.handle_original_file_path(base_path)
        } else {
            // Create new file path for split items
            self.create_split_file_path(base_path, file_name)
        }
    }

    /// Handles the original file path logic
    fn handle_original_file_path(&self, base_path: &Path) -> PathBuf {
        if base_path.is_dir() {
            // If output is a directory, place the original file in it
            base_path.join(self.content().name())
        } else {
            // If output is a file path, use it directly
            base_path.to_path_buf()
        }
    }

    /// Creates a path for split files
    fn create_split_file_path(&self, base_path: &Path, file_name: &str) -> PathBuf {
        if base_path.is_dir() {
            // If output is a directory, place the new file in it
            base_path.join(file_name)
        } else {
            // If output is a file path, replace the file name component
            base_path.with_file_name(file_name)
        }
    }

    /// Helper method to write a collection of items to a file
    fn write_items_to_file(&self, items: &[SourceItem], file_path: &Path) -> Result<()> {
        let content = self.build_file_content(items);
        self.write_content_to_file(&content, file_path)
    }

    /// Builds the complete file content from a collection of items
    fn build_file_content(&self, items: &[SourceItem]) -> String {
        let mut content = String::new();

        for (i, item) in items.iter().enumerate() {
            if i > 0 {
                let spacing = self.determine_item_spacing(&items[i - 1], item);
                content.push_str(&spacing);
            }

            let item_code = self.source_item_to_string(item);
            // Strip trailing newlines to have full control over spacing
            let trimmed_code = item_code.trim_end();
            content.push_str(trimmed_code);
        }

        // Ensure the file ends with a newline
        if !content.is_empty() {
            content.push('\n');
        }

        content
    }

    /// Updates or creates mod.rs file with module declarations for split files
    fn update_mod_file(
        &self,
        base_path: &Path,
        grouped_items: &IndexMap<String, Vec<SourceItem>>,
    ) -> Result<()> {
        let mod_file_path = if base_path.is_dir() {
            base_path.join(Self::MOD_RS)
        } else {
            base_path
                .parent()
                .unwrap_or(Path::new("."))
                .join(Self::MOD_RS)
        };

        let module_names = self.extract_module_names_from_groups(grouped_items);
        let existing_modules = self.read_existing_modules(&mod_file_path)?;
        let combined_modules = self.combine_module_lists(existing_modules, module_names);
        self.write_mod_file_content(&mod_file_path, combined_modules)?;

        Ok(())
    }

    /// Extracts module names from the grouped items, excluding the main file
    fn extract_module_names_from_groups(
        &self,
        grouped_items: &IndexMap<String, Vec<SourceItem>>,
    ) -> Vec<String> {
        grouped_items
            .keys()
            .filter(|&name| name != self.content().name())
            .map(|name| name.trim_end_matches(Self::RS_EXTENSION).to_string())
            .collect()
    }

    /// Reads existing module declarations from mod.rs if it exists
    fn read_existing_modules(&self, mod_file_path: &Path) -> Result<Vec<String>> {
        if mod_file_path.exists() {
            let content = std::fs::read_to_string(mod_file_path).map_err(|e| {
                Error::bail(format!(
                    "Failed to read mod.rs file {}: {}",
                    mod_file_path.display(),
                    e
                ))
            })?;
            Ok(self.parse_module_declarations(&content))
        } else {
            Ok(vec![])
        }
    }

    /// Parses mod declarations from file content
    fn parse_module_declarations(&self, content: &str) -> Vec<String> {
        content
            .lines()
            .filter_map(|line| {
                let trimmed = line.trim();
                if trimmed.starts_with(Self::MOD_PREFIX) && trimmed.ends_with(';') {
                    Some(trimmed[4..trimmed.len() - 1].to_string())
                } else {
                    None
                }
            })
            .collect()
    }

    /// Combines existing and new module lists, removing duplicates
    fn combine_module_lists(&self, existing: Vec<String>, new: Vec<String>) -> Vec<String> {
        let mut combined: Vec<String> = existing;
        for module in new {
            if !combined.contains(&module) {
                combined.push(module);
            }
        }
        combined.sort();
        combined
    }
}
