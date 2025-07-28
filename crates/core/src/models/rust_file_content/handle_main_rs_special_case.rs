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

    /// Handles the special main.rs case by organizing into simple flat structure
    pub(super) fn handle_main_rs_special_case(&self, base_path: &Path) -> Result<()> {
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
