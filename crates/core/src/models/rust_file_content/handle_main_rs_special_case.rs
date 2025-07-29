use super::special_case_utils::SpecialCaseConfig;
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
        let config = SpecialCaseConfig::main_rs();

        // Create models and logic folders using shared utilities
        self.create_types_folder_with_config(&type_items, base_path, &config)?;
        self.create_logic_folder_with_config(&logic_items, base_path, &config)?;

        // Create the new main.rs with module declarations and main function
        self.create_main_file_with_prelude(
            base_path,
            Self::MAIN_RS,
            !type_items.is_empty(),
            !logic_items.is_empty(),
            &main_items,
            &config,
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
}
