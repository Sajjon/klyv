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
    pub const FOLDER_MODELS: &'static str = "models";
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
            return self.handle_lib_rs_special_case(base_path);
        }

        if self.is_main_rs_special_case() {
            debug!("Detected main.rs special case");
            return self.handle_main_rs_special_case(base_path);
        }
        debug!("Non main.rs or lib.rs case, using standard file splitting");
        // Use standard file splitting for regular files
        self.handle_standard_file_splitting(base_path)
    }

    /// Handles standard file splitting logic for regular files
    fn handle_standard_file_splitting(&self, base_path: &Path) -> Result<()> {
        // Apply the same categorization logic as special cases
        let items = self.content().items();
        let (type_items, logic_items, other_items) = self.categorize_regular_file_items(items);

        debug!(
            "Found {} type items, {} logic items, {} other items",
            type_items.len(),
            logic_items.len(),
            other_items.len()
        );

        // Determine the directory where files should be written
        let output_dir = self.determine_output_directory(base_path);

        // If we have both types and logic, organize like special cases
        if !type_items.is_empty() && !logic_items.is_empty() {
            // Create organized structure with categorized items
            self.create_organized_structure_for_regular_file(
                &output_dir,
                &type_items,
                &logic_items,
                &other_items,
            )?;
        } else if !type_items.is_empty() || !logic_items.is_empty() || !other_items.is_empty() {
            // Fall back to traditional grouping for simpler cases
            let grouped_items = self.group_items_by_target_file(items);
            self.write_grouped_items_to_directory(&output_dir, &grouped_items)?;

            // Update mod.rs if multiple files were created
            if grouped_items.len() > 1 {
                debug!("Multiple files created, updating mod.rs");
                self.update_mod_file(base_path, &grouped_items)?;
            }
        }

        Ok(())
    }

    /// Categorizes items for regular files using the same logic as special cases
    fn categorize_regular_file_items(
        &self,
        items: &[SourceItem],
    ) -> (Vec<SourceItem>, Vec<SourceItem>, Vec<SourceItem>) {
        let mut type_items = Vec::new();
        let mut logic_items = Vec::new();
        let mut other_items = Vec::new();

        for item in items {
            match item {
                SourceItem::Function(_) | SourceItem::MacroRules(_) => {
                    logic_items.push(item.clone())
                }
                _ if self.is_type_item(item) => type_items.push(item.clone()),
                _ => other_items.push(item.clone()),
            }
        }

        (type_items, logic_items, other_items)
    }

    /// Creates organized structure for regular files that have both types and logic
    fn create_organized_structure_for_regular_file(
        &self,
        output_dir: &Path,
        type_items: &[SourceItem],
        logic_items: &[SourceItem],
        other_items: &[SourceItem],
    ) -> Result<()> {
        // Write type items using the same logic as special cases
        if !type_items.is_empty() {
            let grouped_type_items = self.group_items_by_target_file(type_items);
            self.write_grouped_items_to_directory(output_dir, &grouped_type_items)?;
        }

        // Write logic items (functions to functions.rs, macros to individual files)
        if !logic_items.is_empty() {
            self.write_logic_items_shared(logic_items, output_dir)?;
        }

        // Write other items to a separate file in output directory if they exist
        if !other_items.is_empty() {
            let other_file_path = output_dir.join("other.rs");
            let content = self.build_organized_file_content(other_items);
            self.write_content_to_file(&content, &other_file_path)?;
        }

        // Create mod.rs with all the modules
        self.create_comprehensive_mod_file(output_dir, type_items, logic_items)?;

        Ok(())
    }

    /// Creates a comprehensive mod.rs file that includes both type and logic modules
    fn create_comprehensive_mod_file(
        &self,
        output_dir: &Path,
        type_items: &[SourceItem],
        logic_items: &[SourceItem],
    ) -> Result<()> {
        let mut module_names = Vec::new();

        // Add type module names
        if !type_items.is_empty() {
            let grouped_type_items = self.group_items_by_target_file(type_items);
            let type_module_names =
                self.extract_module_names_for_organized_items(&grouped_type_items);
            module_names.extend(type_module_names);
        }

        // Add logic module names
        if !logic_items.is_empty() {
            // Check if we have functions (they go into functions.rs)
            let has_functions = logic_items
                .iter()
                .any(|item| matches!(item, SourceItem::Function(_)));
            if has_functions {
                module_names.push(Self::FUNCTIONS_MODULE.to_string());
            }

            // Add module names for each macro (they get individual files)
            for item in logic_items {
                let SourceItem::MacroRules(macro_item) = item else {
                    continue;
                };

                let Some(ident) = &macro_item.ident else {
                    continue;
                };

                module_names.push(ident.to_string());
            }
        }

        if !module_names.is_empty() {
            module_names.sort();
            self.write_mod_file_content(&output_dir.join(Self::MOD_RS), module_names)?;
        }

        Ok(())
    }

    /// Write grouped items to a directory
    fn write_grouped_items_to_directory(
        &self,
        output_dir: &Path,
        grouped_items: &IndexMap<String, Vec<SourceItem>>,
    ) -> Result<()> {
        // Ensure the output directory exists
        std::fs::create_dir_all(output_dir).map_err(|e| {
            Error::bail(format!(
                "Failed to create output directory {}: {}",
                output_dir.display(),
                e
            ))
        })?;

        debug!(
            "Writing {} groups to directory: {}",
            grouped_items.len(),
            output_dir.display()
        );
        for (file_name, group_items) in grouped_items {
            let target_file = output_dir.join(file_name);
            debug!(
                "Writing file: {} with {} items",
                target_file.display(),
                group_items.len()
            );
            let content = self.build_organized_file_content(group_items);
            self.write_content_to_file(&content, &target_file)?;
        }
        Ok(())
    }

    /// Creates the output directory if it doesn't exist
    fn ensure_output_directory_exists(&self, base_path: &Path) -> Result<()> {
        let Some(parent) = base_path.parent() else {
            // No parent directory, nothing to create
            return Ok(());
        };

        std::fs::create_dir_all(parent)
            .map_err(|e| Error::bail(format!("Failed to create directory: {}", e)))
    }

    /// Determines the output directory based on the base path
    /// If base_path is a file (has .rs extension), use its parent directory; if it's a directory, use it directly
    pub(super) fn determine_output_directory(&self, base_path: &Path) -> PathBuf {
        if base_path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
            // It's a .rs file, use the parent directory
            base_path.parent().unwrap_or(Path::new(".")).to_path_buf()
        } else {
            // It's a directory (or no extension), use it directly
            base_path.to_path_buf()
        }
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
        if !mod_file_path.exists() {
            // No existing mod.rs file to read from
            return Ok(vec![]);
        }

        let content = std::fs::read_to_string(mod_file_path).map_err(|e| {
            Error::bail(format!(
                "Failed to read mod.rs file {}: {}",
                mod_file_path.display(),
                e
            ))
        })?;
        Ok(self.parse_module_declarations(&content))
    }

    /// Parses mod declarations from file content
    fn parse_module_declarations(&self, content: &str) -> Vec<String> {
        content
            .lines()
            .filter_map(|line| {
                let trimmed = line.trim();
                if !trimmed.starts_with(Self::MOD_PREFIX) || !trimmed.ends_with(';') {
                    // Skip lines that aren't mod declarations
                    return None;
                }

                Some(trimmed[4..trimmed.len() - 1].to_string())
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
