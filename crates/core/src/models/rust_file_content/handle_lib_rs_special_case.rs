use super::special_case_utils::SpecialCaseConfig;
use crate::prelude::*;

impl RustFileContent {
    /// Checks if this is a lib.rs file that should receive special treatment
    pub(super) fn is_lib_rs_special_case(&self) -> bool {
        let file_name = self.content().name();
        file_name == Self::LIB_RS && self.has_multiple_types_or_functions()
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

    /// Handles the special lib.rs case by organizing into types and logic folders
    pub(super) fn handle_lib_rs_special_case(&self, base_path: &Path) -> Result<()> {
        let items = self.content().items();
        let (type_items, logic_items, other_items) = self.categorize_lib_rs_items(items);
        let config = SpecialCaseConfig::lib_rs();

        // Create types and logic folders using shared utilities
        self.create_types_folder_with_config(&type_items, base_path, &config)?;
        self.create_logic_folder_with_config(&logic_items, base_path, &config)?;

        // Create the new lib.rs with module declarations
        self.create_main_file_with_prelude(
            base_path,
            Self::LIB_RS,
            !type_items.is_empty(),
            !logic_items.is_empty(),
            &other_items,
            &config,
        )?;

        Ok(())
    }

    /// Categorizes items into types, logic (functions and macros), and other items
    fn categorize_lib_rs_items(
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
}
