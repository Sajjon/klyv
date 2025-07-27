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

    /// Checks if this is a lib.rs file that should receive special treatment
    fn is_lib_rs_special_case(&self) -> bool {
        let file_name = self.content().name();
        file_name == Self::LIB_RS && self.has_multiple_types_or_functions()
    }

    /// Checks if this is a main.rs file that should receive special treatment
    fn is_main_rs_special_case(&self) -> bool {
        let file_name = self.content().name();
        file_name == Self::MAIN_RS && self.has_multiple_types_or_functions_for_main()
    }

    /// Checks if the file has multiple types or functions that warrant special organization
    fn has_multiple_types_or_functions(&self) -> bool {
        let items = self.content().items();
        let type_count = items.iter().filter(|item| self.is_type_item(item)).count();
        let function_count = items
            .iter()
            .filter(|item| matches!(item, SourceItem::Function(_)))
            .count();

        type_count > 0 || function_count > 0
    }

    /// Checks if the main.rs file has types or functions that warrant special organization
    /// For main.rs, we want to organize if there are types or non-main functions
    fn has_multiple_types_or_functions_for_main(&self) -> bool {
        let items = self.content().items();
        let type_count = items.iter().filter(|item| self.is_type_item(item)).count();
        let non_main_function_count = items
            .iter()
            .filter(|item| matches!(item, SourceItem::Function(f) if f.sig.ident != Self::MAIN_FUNCTION))
            .count();

        type_count > 0 || non_main_function_count > 0
    }

    /// Categorizes items into types, functions, and main function items (simple approach)
    fn categorize_main_rs_items_simple(
        &self,
        items: &[SourceItem],
    ) -> (Vec<SourceItem>, Vec<SourceItem>, Vec<SourceItem>) {
        let mut type_items = Vec::new();
        let mut function_items = Vec::new();
        let mut main_items = Vec::new();

        for item in items {
            match item {
                SourceItem::Function(f) if f.sig.ident == Self::MAIN_FUNCTION => {
                    main_items.push(item.clone());
                }
                SourceItem::Function(_) => {
                    function_items.push(item.clone());
                }
                _ if self.is_type_item(item) => {
                    type_items.push(item.clone());
                }
                _ => main_items.push(item.clone()),
            }
        }

        (type_items, function_items, main_items)
    }

    /// Handles the special main.rs case by organizing into simple flat structure
    fn handle_main_rs_special_case(&self, base_path: &Path) -> Result<()> {
        let items = self.content().items();
        let (type_items, function_items, main_items) = self.categorize_main_rs_items_simple(items);

        self.write_main_rs_type_items(&type_items, base_path)?;
        self.write_main_rs_function_items(&function_items, base_path)?;

        // Create the new main.rs with module declarations and main function
        self.create_simple_main_rs(
            base_path,
            !type_items.is_empty(),
            !function_items.is_empty(),
            &main_items,
        )?;

        self.create_main_mod_rs_if_needed(&type_items, !function_items.is_empty(), base_path)?;

        Ok(())
    }

    /// Writes type items to individual files for main.rs special case
    fn write_main_rs_type_items(&self, type_items: &[SourceItem], base_path: &Path) -> Result<()> {
        if !type_items.is_empty() {
            let grouped_items = self.group_items_by_target_file(type_items);
            for (file_name, group_items) in grouped_items {
                let target_file = base_path.join(&file_name);
                let content = self.build_organized_file_content(&group_items, Self::CATEGORY_MAIN);
                self.write_content_to_file(&content, &target_file)?;
            }
        }
        Ok(())
    }

    /// Writes function items to utils.rs for main.rs special case
    fn write_main_rs_function_items(
        &self,
        function_items: &[SourceItem],
        base_path: &Path,
    ) -> Result<()> {
        if !function_items.is_empty() {
            let target_file = base_path.join(Self::UTILS_RS);
            let content = self.build_organized_file_content(function_items, Self::CATEGORY_MAIN);
            self.write_content_to_file(&content, &target_file)?;
        }
        Ok(())
    }

    /// Creates mod.rs if needed for main.rs special case
    fn create_main_mod_rs_if_needed(
        &self,
        type_items: &[SourceItem],
        has_functions: bool,
        base_path: &Path,
    ) -> Result<()> {
        if !type_items.is_empty() || has_functions {
            self.create_main_mod_rs(base_path, type_items, has_functions)?;
        }
        Ok(())
    }

    /// Creates a simple main.rs file with module declarations and main function
    fn create_simple_main_rs(
        &self,
        base_path: &Path,
        _has_types: bool,
        has_functions: bool,
        main_items: &[SourceItem],
    ) -> Result<()> {
        let main_rs_path = if base_path.is_dir() {
            base_path.join(Self::MAIN_RS)
        } else {
            base_path.to_path_buf()
        };

        let mut content = String::new();

        // Don't create modules for types, just use individual files directly
        // Only add utils module if we have functions
        if has_functions {
            content.push_str("mod utils;\n");
            content.push('\n');
            content.push_str("use utils::*;\n");
        }

        // Add any remaining items (like use statements, main function)
        if !main_items.is_empty() {
            if has_functions {
                content.push('\n');
            }
            for (i, item) in main_items.iter().enumerate() {
                if i > 0 {
                    let spacing = self.determine_item_spacing(&main_items[i - 1], item);
                    content.push_str(&spacing);
                }
                content.push_str(&self.source_item_to_string(item));
                content.push('\n');
            }
        }

        self.write_content_to_file(&content, &main_rs_path)?;
        Ok(())
    }

    /// Creates mod.rs for main.rs special case
    fn create_main_mod_rs(
        &self,
        base_path: &Path,
        type_items: &[SourceItem],
        has_functions: bool,
    ) -> Result<()> {
        let mut module_names = Vec::new();

        // Add module for individual type files
        if !type_items.is_empty() {
            let grouped_items = self.group_items_by_target_file(type_items);
            let type_module_names = self.extract_module_names_for_organized_items(&grouped_items);
            module_names.extend(type_module_names);
        }

        // Add utils module if there are functions
        if has_functions {
            module_names.push(Self::UTILS_MODULE.to_string());
        }

        if !module_names.is_empty() {
            module_names.sort();
            let mod_rs_path = base_path.join(Self::MOD_RS);
            self.write_mod_file_content(&mod_rs_path, module_names)?;
        }

        Ok(())
    }
}
