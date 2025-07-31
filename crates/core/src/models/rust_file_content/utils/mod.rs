mod convert_doc_attributes_to_comments;
mod helpers;
mod to_snake_case;

use crate::prelude::*;

impl RustFileContent {
    /// Builds file content for organized items with proper prelude import
    pub(super) fn build_organized_file_content(&self, items: &[SourceItem]) -> String {
        let mut content = String::new();

        // Add prelude import at the top
        content.push_str(Self::PRELUDE_IMPORT);

        // Add items with proper spacing
        for item in items {
            content.push_str(&self.source_item_to_string(item));
            content.push('\n');
        }

        content
    }

    /// Writes content string to the specified file path
    pub(super) fn write_content_to_file(&self, content: &str, file_path: &Path) -> Result<()> {
        std::fs::write(file_path, content).map_err(|e| {
            Error::bail(format!(
                "Failed to write file {}: {}",
                file_path.display(),
                e
            ))
        })
    }

    /// Adds an item to the original file group
    pub(super) fn add_item_to_original_file(
        &self,
        groups: &mut IndexMap<String, Vec<SourceItem>>,
        item: &SourceItem,
    ) {
        groups
            .entry(self.content().name().clone())
            .or_default()
            .push(item.clone());
    }

    /// Group items by their target file name
    /// Returns a map where keys are file names and values are vectors of items for that file
    pub(super) fn group_items_by_target_file(
        &self,
        items: &[SourceItem],
    ) -> IndexMap<String, Vec<SourceItem>> {
        let mut groups: IndexMap<String, Vec<SourceItem>> = IndexMap::new();
        let use_statements = self.collect_use_statements(items);

        // Group type definitions with their use statements
        self.group_type_definitions(&mut groups, items, &use_statements);

        // Assign impl blocks to their corresponding types
        self.assign_impl_blocks_to_types(&mut groups, items);

        // Clean up empty original file entries
        self.cleanup_empty_original_file_entry(&mut groups);

        groups
    }

    /// Extracts module names for organized items (doesn't filter out main file name)
    pub(super) fn extract_module_names_for_organized_items(
        &self,
        grouped_items: &IndexMap<String, Vec<SourceItem>>,
    ) -> Vec<String> {
        let mut module_names: Vec<String> = grouped_items
            .keys()
            .map(|name| name.trim_end_matches(Self::RS_EXTENSION).to_string())
            .collect();
        module_names.sort();
        module_names
    }

    /// Writes the mod.rs file content with module declarations and re-exports
    pub(super) fn write_mod_file_content(
        &self,
        mod_file_path: &Path,
        modules: Vec<String>,
    ) -> Result<()> {
        let mut content = String::new();

        // Add mod declarations
        for module in &modules {
            content.push_str(&format!("mod {};\n", module));
        }

        content.push('\n');

        // Add pub use re-exports
        for module in &modules {
            content.push_str(&format!("pub use {}::*;\n", module));
        }

        std::fs::write(mod_file_path, content).map_err(|e| {
            Error::bail(format!(
                "Failed to write mod.rs file {}: {}",
                mod_file_path.display(),
                e
            ))
        })
    }

    /// Convert a SourceItem back to its string representation
    pub(super) fn source_item_to_string(&self, item: &SourceItem) -> String {
        let token_stream = self.convert_item_to_token_stream(item);
        let formatted_code = self.format_token_stream(token_stream);

        // Convert #[doc = "..."] attributes back to /// doc comments
        self.convert_doc_attributes_to_comments(formatted_code)
    }

    /// Checks if an item is a type (struct, enum, trait, type alias, union, impl)
    pub(super) fn is_type_item(&self, item: &SourceItem) -> bool {
        matches!(
            item,
            SourceItem::Struct(_)
                | SourceItem::Enum(_)
                | SourceItem::Trait(_)
                | SourceItem::Type(_)
                | SourceItem::Union(_)
                | SourceItem::Impl(_)
        )
    }
}
