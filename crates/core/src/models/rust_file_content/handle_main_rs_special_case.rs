use crate::prelude::*;

impl RustFileContent {
    /// Checks if this is a main.rs file that should receive special treatment
    pub(super) fn is_main_rs_special_case(&self) -> bool {
        let file_name = self.content().name();
        file_name == Self::MAIN_RS && self.has_multiple_types_or_functions_for_main()
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

    /// Handles the special main.rs case by organizing into models and logic folders
    pub(super) fn handle_main_rs_special_case(&self, base_path: &Path) -> Result<()> {
        let items = self.content().items();
        let (type_items, logic_items, main_items) = self.categorize_main_rs_items(items);

        // Create models and logic folders like lib.rs does
        self.create_types_folder_if_needed(&type_items, base_path)?;
        self.create_main_logic_folder_if_needed(&logic_items, base_path)?;

        // Create the new main.rs with module declarations and main function
        self.create_main_rs_with_prelude(
            base_path,
            !type_items.is_empty(),
            !logic_items.is_empty(),
            &main_items,
        )?;

        Ok(())
    }

    /// Categorizes items into types, logic (functions and macros), and main function items
    fn categorize_main_rs_items(
        &self,
        items: &[SourceItem],
    ) -> (Vec<SourceItem>, Vec<SourceItem>, Vec<SourceItem>) {
        let mut type_items = Vec::new();
        let mut logic_items = Vec::new();
        let mut main_items = Vec::new();

        for item in items {
            match item {
                SourceItem::Function(f) if f.sig.ident == Self::MAIN_FUNCTION => {
                    main_items.push(item.clone());
                }
                SourceItem::Function(_) | SourceItem::MacroRules(_) => {
                    logic_items.push(item.clone());
                }
                _ if self.is_type_item(item) => {
                    type_items.push(item.clone());
                }
                _ => main_items.push(item.clone()),
            }
        }

        (type_items, logic_items, main_items)
    }

    /// Creates types folder and files if there are type items
    fn create_types_folder_if_needed(
        &self,
        type_items: &[SourceItem],
        base_path: &Path,
    ) -> Result<()> {
        if !type_items.is_empty() {
            let output_dir = self.determine_output_directory(base_path);
            let types_dir = output_dir.join(Self::FOLDER_TYPES);
            std::fs::create_dir_all(&types_dir)
                .map_err(|e| Error::bail(format!("Failed to create types directory: {}", e)))?;

            // Group items by target file and write each group
            let grouped_items = self.group_items_by_target_file(type_items);
            for (file_name, group_items) in grouped_items {
                let target_file = types_dir.join(&file_name);
                let content = self.build_organized_file_content(&group_items, Self::CATEGORY_TYPES);
                self.write_content_to_file(&content, &target_file)?;
            }

            self.create_main_models_mod_rs(&types_dir, type_items)?;
        }
        Ok(())
    }

    /// Creates logic folder and files if there are function/macro items
    fn create_main_logic_folder_if_needed(
        &self,
        logic_items: &[SourceItem],
        base_path: &Path,
    ) -> Result<()> {
        if !logic_items.is_empty() {
            let output_dir = self.determine_output_directory(base_path);
            let logic_dir = output_dir.join(Self::FOLDER_LOGIC);
            std::fs::create_dir_all(&logic_dir)
                .map_err(|e| Error::bail(format!("Failed to create logic directory: {}", e)))?;

            // Write logic items to individual files (functions.rs, macro_name.rs, etc.)
            self.write_main_logic_items(logic_items, &logic_dir)?;
            self.create_main_logic_mod_rs(&logic_dir, logic_items)?;
        }
        Ok(())
    }

    /// Creates mod.rs for the models directory
    fn create_main_models_mod_rs(&self, models_dir: &Path, items: &[SourceItem]) -> Result<()> {
        let grouped_items = self.group_items_by_target_file(items);
        let module_names = self.extract_module_names_for_organized_items(&grouped_items);
        self.write_mod_file_content(&models_dir.join(Self::MOD_RS), module_names)?;
        Ok(())
    }

    /// Creates mod.rs for the logic directory
    fn create_main_logic_mod_rs(&self, logic_dir: &Path, items: &[SourceItem]) -> Result<()> {
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

    /// Creates the new main.rs file with prelude module structure
    fn create_main_rs_with_prelude(
        &self,
        base_path: &Path,
        has_types: bool,
        has_logic: bool,
        main_items: &[SourceItem],
    ) -> Result<()> {
        let main_rs_path = if base_path.is_file() {
            base_path.to_path_buf()
        } else {
            base_path.join(Self::MAIN_RS)
        };

        let mut content = String::new();

        // Add module declarations
        if has_types {
            content.push_str("mod models;\n");
        }
        if has_logic {
            content.push_str("mod logic;\n");
        }

        // Add prelude module
        if has_types || has_logic {
            content.push_str("\npub mod prelude {\n");
            if has_types {
                content.push_str("    pub use crate::models::*;\n");
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

    /// Writes logic items (functions and macros) to individual files
    fn write_main_logic_items(&self, logic_items: &[SourceItem], logic_dir: &Path) -> Result<()> {
        // Separate functions and macros
        let mut functions = Vec::new();
        let mut macros = Vec::new();

        for item in logic_items {
            match item {
                SourceItem::Function(_) => functions.push(item.clone()),
                SourceItem::MacroRules(_) => {
                    // Add #[macro_export] attribute and place in individual file
                    macros.push(item.clone());
                }
                _ => functions.push(item.clone()), // fallback for other logic items
            }
        }

        // Write functions to functions.rs if any exist
        if !functions.is_empty() {
            let functions_file = logic_dir.join(Self::FUNCTIONS_RS);
            let content = self.build_organized_file_content(&functions, Self::CATEGORY_LOGIC);
            self.write_content_to_file(&content, &functions_file)?;
        }

        // Write each macro to its own file with #[macro_export]
        for item in &macros {
            if let SourceItem::MacroRules(macro_item) = item {
                if let Some(ident) = &macro_item.ident {
                    let macro_name = ident.to_string();
                    let macro_file = logic_dir.join(format!("{}.rs", macro_name));

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
}
