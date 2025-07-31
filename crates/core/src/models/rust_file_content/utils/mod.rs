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

        // Add items with proper spacing between different items
        for item in items {
            content.push_str(&self.source_item_to_string(item));
            content.push('\n');
            if item.is_impl() {
                content.push('\n');
            }
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
        let doc_converted = self.convert_doc_attributes_to_comments(formatted_code);

        // For impl blocks, ensure proper spacing between methods
        if matches!(item, SourceItem::Impl(_)) {
            self.fix_impl_method_spacing(doc_converted)
        } else {
            doc_converted
        }
    }

    /// Fixes spacing between methods in impl blocks by adding blank lines where needed
    fn fix_impl_method_spacing(&self, impl_code: String) -> String {
        let lines: Vec<&str> = impl_code.lines().collect();
        let mut output = Vec::new();
        let mut brace_depth = 0;
        let mut prev_line_was_method_end = false;

        for line in lines {
            let trimmed = line.trim();

            // Count braces to track depth
            for ch in line.chars() {
                match ch {
                    '{' => brace_depth += 1,
                    '}' => brace_depth -= 1,
                    _ => {}
                }
            }

            // If we're at impl level (depth 1) and the previous line was a method end
            // and this line starts a new method, add a blank line
            if brace_depth == 1
                && prev_line_was_method_end
                && (trimmed.starts_with("pub fn ")
                    || trimmed.starts_with("fn ")
                    || trimmed.starts_with("pub async fn ")
                    || trimmed.starts_with("async fn ")
                    || trimmed.starts_with("/// ")
                    || trimmed.starts_with("#["))
            {
                output.push(String::new()); // Add blank line
            }

            output.push(line.to_string());

            // Check if this line ends a method (closing brace at impl level)
            prev_line_was_method_end = brace_depth == 1 && trimmed == "}";
        }

        output.join("\n")
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
