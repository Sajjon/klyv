use crate::prelude::*;

/// Configuration for special case handling
pub struct SpecialCaseConfig {
    pub types_folder: &'static str,
    pub logic_folder: &'static str,
    pub types_category: &'static str,
    pub logic_category: &'static str,
    pub module_name: &'static str,
}

impl SpecialCaseConfig {
    /// Configuration for lib.rs special case
    pub fn lib_rs() -> Self {
        Self {
            types_folder: RustFileContent::FOLDER_TYPES,
            logic_folder: RustFileContent::FOLDER_LOGIC,
            types_category: RustFileContent::CATEGORY_TYPES,
            logic_category: RustFileContent::CATEGORY_LOGIC,
            module_name: "types",
        }
    }

    /// Configuration for main.rs special case (uses types folder but models module name)
    pub fn main_rs() -> Self {
        Self {
            types_folder: RustFileContent::FOLDER_TYPES,
            logic_folder: RustFileContent::FOLDER_LOGIC,
            types_category: RustFileContent::CATEGORY_TYPES,
            logic_category: RustFileContent::CATEGORY_LOGIC,
            module_name: "models",
        }
    }
}

impl RustFileContent {
    /// Shared logic for writing types folder and files
    pub(super) fn create_types_folder_with_config(
        &self,
        type_items: &[SourceItem],
        base_path: &Path,
        config: &SpecialCaseConfig,
    ) -> Result<()> {
        if !type_items.is_empty() {
            let output_dir = self.determine_output_directory(base_path);
            let types_dir = output_dir.join(config.types_folder);
            std::fs::create_dir_all(&types_dir)
                .map_err(|e| Error::bail(format!("Failed to create types directory: {}", e)))?;

            // Group items by target file and write each group
            let grouped_items = self.group_items_by_target_file(type_items);
            for (file_name, group_items) in grouped_items {
                let target_file = types_dir.join(&file_name);
                let content =
                    self.build_organized_file_content(&group_items, config.types_category);
                self.write_content_to_file(&content, &target_file)?;
            }

            self.create_types_mod_rs_shared(&types_dir, type_items)?;
        }
        Ok(())
    }

    /// Shared logic for creating logic folder and files
    pub(super) fn create_logic_folder_with_config(
        &self,
        logic_items: &[SourceItem],
        base_path: &Path,
        config: &SpecialCaseConfig,
    ) -> Result<()> {
        if !logic_items.is_empty() {
            let output_dir = self.determine_output_directory(base_path);
            let logic_dir = output_dir.join(config.logic_folder);
            std::fs::create_dir_all(&logic_dir)
                .map_err(|e| Error::bail(format!("Failed to create logic directory: {}", e)))?;

            // Write logic items to individual files (functions.rs, macro_name.rs, etc.)
            self.write_logic_items_shared(logic_items, &logic_dir, config.logic_category)?;
            self.create_logic_mod_rs_shared(&logic_dir, logic_items)?;
        }
        Ok(())
    }

    /// Shared logic for writing logic items (functions and macros) to individual files
    pub(super) fn write_logic_items_shared(
        &self,
        items: &[SourceItem],
        dir: &Path,
        category: &str,
    ) -> Result<()> {
        // Separate functions and macros
        let mut functions = Vec::new();
        let mut macros = Vec::new();

        for item in items {
            match item {
                SourceItem::Function(_) => functions.push(item.clone()),
                SourceItem::MacroRules(_) => {
                    macros.push(item.clone());
                }
                _ => functions.push(item.clone()), // fallback for other logic items
            }
        }

        // Write functions to functions.rs if any exist
        if !functions.is_empty() {
            let functions_file = dir.join(Self::FUNCTIONS_RS);
            let content = self.build_organized_file_content(&functions, category);
            self.write_content_to_file(&content, &functions_file)?;
        }

        // Write each macro to its own file with #[macro_export]
        for item in &macros {
            if let SourceItem::MacroRules(macro_item) = item {
                if let Some(ident) = &macro_item.ident {
                    let macro_name = ident.to_string();
                    let macro_file = dir.join(format!("{}.rs", macro_name));

                    // Build content with #[macro_export] attribute
                    let mut content = String::new();
                    content.push_str(Self::PRELUDE_IMPORT);
                    content.push_str("#[macro_export]\n");
                    content.push_str(&self.source_item_to_string(item));
                    content.push('\n');

                    self.write_content_to_file(&content, &macro_file)?;
                }
            }
        }

        Ok(())
    }

    /// Shared logic for creating types mod.rs
    pub(super) fn create_types_mod_rs_shared(
        &self,
        types_dir: &Path,
        items: &[SourceItem],
    ) -> Result<()> {
        let grouped_items = self.group_items_by_target_file(items);
        let module_names = self.extract_module_names_for_organized_items(&grouped_items);
        self.write_mod_file_content(&types_dir.join(Self::MOD_RS), module_names)?;
        Ok(())
    }

    /// Shared logic for creating logic mod.rs
    pub(super) fn create_logic_mod_rs_shared(
        &self,
        logic_dir: &Path,
        items: &[SourceItem],
    ) -> Result<()> {
        let mut module_names = Vec::new();

        // Check if we have functions (they go into functions.rs)
        let has_functions = items
            .iter()
            .any(|item| matches!(item, SourceItem::Function(_)));
        if has_functions {
            module_names.push(Self::FUNCTIONS_MODULE.to_string());
        }

        // Add module names for each macro (they get individual files)
        for item in items {
            if let SourceItem::MacroRules(macro_item) = item {
                if let Some(ident) = &macro_item.ident {
                    module_names.push(ident.to_string());
                }
            }
        }

        if !module_names.is_empty() {
            module_names.sort();
            self.write_mod_file_content(&logic_dir.join(Self::MOD_RS), module_names)?;
        }
        Ok(())
    }

    /// Shared logic for creating main file with prelude module structure
    pub(super) fn create_main_file_with_prelude(
        &self,
        base_path: &Path,
        file_name: &str,
        has_types: bool,
        has_logic: bool,
        main_items: &[SourceItem],
        config: &SpecialCaseConfig,
    ) -> Result<()> {
        let main_file_path = if base_path.is_file() {
            base_path.to_path_buf()
        } else {
            base_path.join(file_name)
        };

        let mut content = String::new();

        // Add module declarations
        if has_types {
            content.push_str(&format!("mod {};\n", config.module_name));
        }
        if has_logic {
            content.push_str("mod logic;\n");
        }

        // Add prelude module
        if has_types || has_logic {
            content.push_str("\npub mod prelude {\n");
            if has_types {
                content.push_str(&format!("    pub use crate::{}::*;\n", config.module_name));
            }
            if has_logic {
                content.push_str("    pub use crate::logic::*;\n");
            }
            content.push_str("}\n\n");
            content.push_str("use prelude::*;\n");
        }

        // Add remaining items (use statements, main function, etc.)
        if !main_items.is_empty() {
            if has_types || has_logic {
                content.push('\n');
            }
            for (i, item) in main_items.iter().enumerate() {
                if i > 0 {
                    content.push('\n');
                }
                content.push_str(&self.source_item_to_string(item));
                content.push('\n');
            }
        }

        self.write_content_to_file(&content, &main_file_path)?;
        Ok(())
    }
}
