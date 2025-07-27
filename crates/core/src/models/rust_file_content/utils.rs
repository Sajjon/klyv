use super::*;
use crate::prelude::*;

impl RustFileContent {
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

    /// Convert a SourceItem back to its string representation
    pub(super) fn source_item_to_string(&self, item: &SourceItem) -> String {
        let token_stream = self.convert_item_to_token_stream(item);
        let formatted_code = self.format_token_stream(token_stream);

        // Convert #[doc = "..."] attributes back to /// doc comments
        self.convert_doc_attributes_to_comments(formatted_code)
    }

    /// Formats a TokenStream using prettyplease or falls back to string conversion
    fn format_token_stream(&self, token_stream: proc_macro2::TokenStream) -> String {
        // Parse the token stream back to a syn::File to format it properly
        if let Ok(file) = syn::parse2::<syn::File>(token_stream.clone()) {
            prettyplease::unparse(&file)
        } else {
            // Fallback to the original token stream if parsing fails
            token_stream.to_string()
        }
    }

    /// Converts a SourceItem to a TokenStream
    fn convert_item_to_token_stream(&self, item: &SourceItem) -> proc_macro2::TokenStream {
        use quote::ToTokens;

        match item {
            SourceItem::Struct(s) => s.to_token_stream(),
            SourceItem::Enum(e) => e.to_token_stream(),
            SourceItem::Trait(t) => t.to_token_stream(),
            SourceItem::Type(ty) => ty.to_token_stream(),
            SourceItem::Union(u) => u.to_token_stream(),
            SourceItem::Function(f) => f.to_token_stream(),
            SourceItem::MacroRules(m) => m.to_token_stream(),
            SourceItem::Impl(i) => i.to_token_stream(),
            SourceItem::Use(u) => u.to_token_stream(),
            SourceItem::Unsplittable(item) => item.to_token_stream(),
            SourceItem::Verbatim(tokens) => tokens.clone(),
        }
    }

    /// Determines the appropriate spacing between two consecutive items
    pub(super) fn determine_item_spacing(
        &self,
        prev_item: &SourceItem,
        current_item: &SourceItem,
    ) -> String {
        match (prev_item, current_item) {
            // Use statements should have single newlines between them
            (SourceItem::Use(_), SourceItem::Use(_)) => "\n".to_string(),
            // Everything else gets double newlines for better separation
            _ => "\n\n".to_string(),
        }
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

    /// Collects all use statements from the items
    fn collect_use_statements(&self, items: &[SourceItem]) -> Vec<SourceItem> {
        items
            .iter()
            .filter(|item| matches!(item, SourceItem::Use(_)))
            .cloned()
            .collect()
    }

    /// Groups type definitions (structs, enums, traits, etc.) with their use statements
    fn group_type_definitions(
        &self,
        groups: &mut IndexMap<String, Vec<SourceItem>>,
        items: &[SourceItem],
        use_statements: &[SourceItem],
    ) {
        for item in items {
            if let Some(type_name) = self.extract_type_name_from_item(item) {
                let file_name = format!("{}{}", self.to_snake_case(&type_name), Self::RS_EXTENSION);
                self.add_item_to_group(groups, file_name, use_statements, item);
            } else if self.is_non_type_item(item) {
                // Functions, macros, etc. go to the original file
                self.add_item_to_original_file(groups, item);
            }
        }
    }

    /// Checks if an item is a non-type item (functions, macros, etc.)
    fn is_non_type_item(&self, item: &SourceItem) -> bool {
        !matches!(
            item,
            SourceItem::Struct(_)
                | SourceItem::Enum(_)
                | SourceItem::Trait(_)
                | SourceItem::Type(_)
                | SourceItem::Union(_)
                | SourceItem::Impl(_)
                | SourceItem::Use(_)
        )
    }

    /// Adds an item and its use statements to a specific group
    fn add_item_to_group(
        &self,
        groups: &mut IndexMap<String, Vec<SourceItem>>,
        file_name: String,
        use_statements: &[SourceItem],
        item: &SourceItem,
    ) {
        let group = groups.entry(file_name).or_default();
        // Add use statements first, then the item
        group.extend(use_statements.iter().cloned());
        group.push(item.clone());
    }

    /// Assigns impl blocks to their corresponding type files
    fn assign_impl_blocks_to_types(
        &self,
        groups: &mut IndexMap<String, Vec<SourceItem>>,
        items: &[SourceItem],
    ) {
        let impl_blocks = self.collect_impl_blocks(items);

        for impl_item in impl_blocks {
            self.assign_single_impl_block(groups, &impl_item);
        }
    }

    /// Assigns a single impl block to its target type file
    fn assign_single_impl_block(
        &self,
        groups: &mut IndexMap<String, Vec<SourceItem>>,
        impl_item: &SourceItem,
    ) {
        if let SourceItem::Impl(impl_block) = impl_item {
            let target_type = self.extract_impl_target_type(impl_block);

            if let Some(type_name) = target_type {
                let file_name = format!("{}{}", self.to_snake_case(&type_name), Self::RS_EXTENSION);
                self.try_add_impl_to_type_file(groups, &file_name, impl_item);
            } else {
                // Can't determine target type, put in original file
                self.add_item_to_original_file(groups, impl_item);
            }
        }
    }

    /// Tries to add an impl block to its type file, falls back to original file
    fn try_add_impl_to_type_file(
        &self,
        groups: &mut IndexMap<String, Vec<SourceItem>>,
        file_name: &str,
        impl_item: &SourceItem,
    ) {
        if groups.contains_key(file_name) {
            groups.get_mut(file_name).unwrap().push(impl_item.clone());
        } else {
            // Type file doesn't exist, put in original file
            self.add_item_to_original_file(groups, impl_item);
        }
    }

    /// Extract the target type name from an impl block
    fn extract_impl_target_type(&self, impl_block: &syn::ItemImpl) -> Option<String> {
        // Handle impl blocks like "impl SomeType" or "impl SomeTrait for SomeType"
        if let syn::Type::Path(type_path) = impl_block.self_ty.as_ref() {
            if let Some(segment) = type_path.path.segments.last() {
                return Some(segment.ident.to_string());
            }
        }
        None
    }

    /// Collects all impl blocks from the items
    fn collect_impl_blocks(&self, items: &[SourceItem]) -> Vec<SourceItem> {
        items
            .iter()
            .filter(|item| matches!(item, SourceItem::Impl(_)))
            .cloned()
            .collect()
    }

    /// Removes empty original file entries from the groups
    fn cleanup_empty_original_file_entry(&self, groups: &mut IndexMap<String, Vec<SourceItem>>) {
        if let Some(original_items) = groups.get(self.content().name()) {
            if original_items.is_empty() {
                groups.shift_remove(self.content().name());
            }
        }
    }

    /// Extracts the type name from various SourceItem types
    fn extract_type_name_from_item(&self, item: &SourceItem) -> Option<String> {
        match item {
            SourceItem::Struct(s) => Some(s.ident.to_string()),
            SourceItem::Enum(e) => Some(e.ident.to_string()),
            SourceItem::Trait(t) => Some(t.ident.to_string()),
            SourceItem::Type(ty) => Some(ty.ident.to_string()),
            SourceItem::Union(u) => Some(u.ident.to_string()),
            _ => None,
        }
    }

    /// Convert PascalCase to snake_case
    pub(super) fn to_snake_case(&self, input: &str) -> String {
        let mut result = String::new();
        let mut prev_was_lowercase = false;

        for ch in input.chars() {
            self.process_snake_case_character(ch, &mut result, &mut prev_was_lowercase);
        }

        result
    }

    /// Processes a single character for snake_case conversion
    fn process_snake_case_character(
        &self,
        ch: char,
        result: &mut String,
        prev_was_lowercase: &mut bool,
    ) {
        if ch.is_uppercase() {
            self.handle_uppercase_character(ch, result, *prev_was_lowercase);
            *prev_was_lowercase = false;
        } else {
            result.push(ch);
            *prev_was_lowercase = ch.is_lowercase();
        }
    }

    /// Handles uppercase characters in snake_case conversion
    fn handle_uppercase_character(&self, ch: char, result: &mut String, prev_was_lowercase: bool) {
        if prev_was_lowercase && !result.is_empty() {
            result.push('_'); // Add underscore before uppercase letter
        }
        result.push(ch.to_lowercase().next().unwrap());
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

    /// Builds file content for organized items with proper prelude import
    pub(super) fn build_organized_file_content(
        &self,
        items: &[SourceItem],
        _category: &str,
    ) -> String {
        let mut content = String::new();

        // Add prelude import at the top
        content.push_str(Self::PRELUDE_IMPORT);

        // Add items with proper spacing
        for (i, item) in items.iter().enumerate() {
            if i > 0 {
                let spacing = self.determine_item_spacing(&items[i - 1], item);
                content.push_str(&spacing);
            }
            content.push_str(&self.source_item_to_string(item));
            content.push('\n');
        }

        content
    }
}
