use crate::prelude::*;

impl RustFileContent {
    /// Handles the special lib.rs case by organizing into types and logic folders
    pub(super) fn handle_lib_rs_special_case(&self, base_path: &Path) -> Result<()> {
        let items = self.content().items();
        let (type_items, logic_items, other_items) = self.categorize_lib_rs_items(items);

        self.create_types_folder_if_needed(&type_items, base_path)?;
        self.create_logic_folder_if_needed(&logic_items, base_path)?;

        // Create the new lib.rs with prelude module
        self.create_lib_rs_with_prelude(
            base_path,
            !type_items.is_empty(),
            !logic_items.is_empty(),
            &other_items,
        )?;

        Ok(())
    }

    /// Categorizes items into types, logic (functions), and other items
    fn categorize_lib_rs_items(
        &self,
        items: &[SourceItem],
    ) -> (Vec<SourceItem>, Vec<SourceItem>, Vec<SourceItem>) {
        let mut type_items = Vec::new();
        let mut logic_items = Vec::new();
        let mut other_items = Vec::new();

        for item in items {
            match item {
                SourceItem::Function(_) => logic_items.push(item.clone()),
                _ if self.is_type_item(item) => type_items.push(item.clone()),
                _ => other_items.push(item.clone()),
            }
        }

        (type_items, logic_items, other_items)
    }

    /// Creates mod.rs for the types directory
    fn create_types_mod_rs(&self, types_dir: &Path, items: &[SourceItem]) -> Result<()> {
        let grouped_items = self.group_items_by_target_file(items);
        let module_names = self.extract_module_names_for_organized_items(&grouped_items);
        self.write_mod_file_content(&types_dir.join(Self::MOD_RS), module_names)?;
        Ok(())
    }

    /// Creates types folder and files if there are type items
    fn create_types_folder_if_needed(
        &self,
        type_items: &[SourceItem],
        base_path: &Path,
    ) -> Result<()> {
        if !type_items.is_empty() {
            let types_dir = base_path.join(Self::FOLDER_TYPES);
            std::fs::create_dir_all(&types_dir)
                .map_err(|e| Error::bail(format!("Failed to create types directory: {}", e)))?;
            self.write_organized_items(type_items, &types_dir, Self::CATEGORY_TYPES)?;
            self.create_types_mod_rs(&types_dir, type_items)?;
        }
        Ok(())
    }

    /// Creates mod.rs for the logic directory  
    fn create_logic_mod_rs(&self, logic_dir: &Path, items: &[SourceItem]) -> Result<()> {
        if !items.is_empty() {
            // For logic items, we always create functions.rs
            let module_names = vec![Self::FUNCTIONS_MODULE.to_string()];
            self.write_mod_file_content(&logic_dir.join(Self::MOD_RS), module_names)?;
        }
        Ok(())
    }

    /// Creates logic folder and files if there are function items
    fn create_logic_folder_if_needed(
        &self,
        logic_items: &[SourceItem],
        base_path: &Path,
    ) -> Result<()> {
        if !logic_items.is_empty() {
            let logic_dir = base_path.join(Self::FOLDER_LOGIC);
            std::fs::create_dir_all(&logic_dir)
                .map_err(|e| Error::bail(format!("Failed to create logic directory: {}", e)))?;
            self.write_organized_items(logic_items, &logic_dir, Self::CATEGORY_LOGIC)?;
            self.create_logic_mod_rs(&logic_dir, logic_items)?;
        }
        Ok(())
    }

    /// Determines the path for the lib.rs file
    fn determine_lib_rs_path(&self, base_path: &Path) -> PathBuf {
        if base_path.is_dir() {
            base_path.join(Self::LIB_RS)
        } else {
            base_path.to_path_buf()
        }
    }

    /// Adds module declarations to lib.rs content
    fn add_module_declarations(&self, content: &mut String, has_logic: bool, has_types: bool) {
        if has_logic {
            content.push_str("mod logic;\n");
        }
        if has_types {
            content.push_str("mod types;\n");
        }
    }

    /// Adds remaining items (like use statements) to lib.rs content
    fn add_remaining_items(&self, content: &mut String, other_items: &[SourceItem]) {
        if !other_items.is_empty() {
            content.push('\n');
            for (i, item) in other_items.iter().enumerate() {
                if i > 0 {
                    let spacing = self.determine_item_spacing(&other_items[i - 1], item);
                    content.push_str(&spacing);
                }
                content.push_str(&self.source_item_to_string(item));
                content.push('\n');
            }
        }
    }

    /// Builds the content for lib.rs file
    fn build_lib_rs_content(
        &self,
        has_types: bool,
        has_logic: bool,
        other_items: &[SourceItem],
    ) -> String {
        let mut content = String::new();

        self.add_module_declarations(&mut content, has_logic, has_types);
        self.add_remaining_items(&mut content, other_items);
        self.add_prelude_module(&mut content, has_logic, has_types, other_items);

        content
    }

    /// Adds prelude module to lib.rs content
    fn add_prelude_module(
        &self,
        content: &mut String,
        has_logic: bool,
        has_types: bool,
        other_items: &[SourceItem],
    ) {
        content.push_str("\npub mod prelude {\n");

        if has_logic {
            content.push_str("    pub use crate::logic::*;\n");
        }
        if has_types {
            content.push_str("    pub use crate::types::*;\n");
        }

        if self.should_add_common_imports(other_items) {
            content.push('\n');
            content.push_str("    pub use std::{\n");
            content.push_str("        collections::IndexMap,\n");
            content.push_str("        path::{Path, PathBuf},\n");
            content.push_str("    };\n");
        }

        content.push_str("}\n");
    }

    /// Checks if we should add common imports (when there are minimal existing imports)
    fn should_add_common_imports(&self, other_items: &[SourceItem]) -> bool {
        // Add common imports if there are few or no existing use statements
        let use_count = other_items
            .iter()
            .filter(|item| matches!(item, SourceItem::Use(_)))
            .count();
        use_count <= 2 // Only if there are 2 or fewer existing use statements
    }

    /// Creates the new lib.rs file with prelude module structure
    fn create_lib_rs_with_prelude(
        &self,
        base_path: &Path,
        has_types: bool,
        has_logic: bool,
        other_items: &[SourceItem],
    ) -> Result<()> {
        let lib_rs_path = self.determine_lib_rs_path(base_path);
        let content = self.build_lib_rs_content(has_types, has_logic, other_items);
        self.write_content_to_file(&content, &lib_rs_path)?;
        Ok(())
    }

    /// Writes organized items to separate files in the given directory
    fn write_organized_items(
        &self,
        items: &[SourceItem],
        dir: &Path,
        category: &str,
    ) -> Result<()> {
        match category {
            Self::CATEGORY_LOGIC => {
                self.write_logic_items(items, dir, category)?;
            }
            Self::CATEGORY_CLI | Self::CATEGORY_CORE => {
                self.write_cli_core_items(items, dir, category)?;
            }
            _ => {
                self.write_standard_grouped_items(items, dir, category)?;
            }
        }

        Ok(())
    }

    /// Writes items using standard grouping
    fn write_standard_grouped_items(
        &self,
        items: &[SourceItem],
        dir: &Path,
        category: &str,
    ) -> Result<()> {
        let grouped_items = self.group_items_by_target_file(items);
        for (file_name, group_items) in grouped_items {
            let target_file = dir.join(&file_name);
            let content = self.build_organized_file_content(&group_items, category);
            self.write_content_to_file(&content, &target_file)?;
        }
        Ok(())
    }

    /// Writes logic items (functions) to functions.rs
    fn write_logic_items(&self, items: &[SourceItem], dir: &Path, category: &str) -> Result<()> {
        let target_file = dir.join(Self::FUNCTIONS_RS);
        let content = self.build_organized_file_content(items, category);
        self.write_content_to_file(&content, &target_file)?;
        Ok(())
    }

    /// Writes CLI and core items, separating types and functions
    fn write_cli_core_items(&self, items: &[SourceItem], dir: &Path, category: &str) -> Result<()> {
        let (type_items, function_items) = self.separate_types_and_functions(items);

        self.write_separated_type_items(&type_items, dir, category)?;
        self.write_separated_function_items(&function_items, dir, category)?;

        Ok(())
    }

    /// Writes separated function items to functions.rs
    fn write_separated_function_items(
        &self,
        function_items: &[SourceItem],
        dir: &Path,
        category: &str,
    ) -> Result<()> {
        if !function_items.is_empty() {
            let target_file = dir.join(Self::FUNCTIONS_RS);
            let content = self.build_organized_file_content(function_items, category);
            self.write_content_to_file(&content, &target_file)?;
        }
        Ok(())
    }

    /// Writes separated type items to individual files
    fn write_separated_type_items(
        &self,
        type_items: &[SourceItem],
        dir: &Path,
        category: &str,
    ) -> Result<()> {
        if !type_items.is_empty() {
            let grouped_items = self.group_items_by_target_file(type_items);
            for (file_name, group_items) in grouped_items {
                let target_file = dir.join(&file_name);
                let content = self.build_organized_file_content(&group_items, category);
                self.write_content_to_file(&content, &target_file)?;
            }
        }
        Ok(())
    }

    /// Separates items into types and functions
    fn separate_types_and_functions(
        &self,
        items: &[SourceItem],
    ) -> (Vec<SourceItem>, Vec<SourceItem>) {
        let mut type_items = Vec::new();
        let mut function_items = Vec::new();

        for item in items {
            if self.is_type_item(item) {
                type_items.push(item.clone());
            } else if matches!(item, SourceItem::Function(_)) {
                function_items.push(item.clone());
            }
        }

        (type_items, function_items)
    }
}
