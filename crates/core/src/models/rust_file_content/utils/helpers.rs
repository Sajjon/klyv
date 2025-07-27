use crate::prelude::*;

impl RustFileContent {
    /// Formats a TokenStream using prettyplease or falls back to string conversion
    pub(super) fn format_token_stream(&self, token_stream: proc_macro2::TokenStream) -> String {
        // Parse the token stream back to a syn::File to format it properly
        if let Ok(file) = syn::parse2::<syn::File>(token_stream.clone()) {
            prettyplease::unparse(&file)
        } else {
            // Fallback to the original token stream if parsing fails
            token_stream.to_string()
        }
    }

    /// Converts a SourceItem to a TokenStream
    pub(super) fn convert_item_to_token_stream(
        &self,
        item: &SourceItem,
    ) -> proc_macro2::TokenStream {
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

    /// Collects all use statements from the items
    pub(super) fn collect_use_statements(&self, items: &[SourceItem]) -> Vec<SourceItem> {
        items
            .iter()
            .filter(|item| matches!(item, SourceItem::Use(_)))
            .cloned()
            .collect()
    }

    /// Groups type definitions (structs, enums, traits, etc.) with their use statements
    pub(super) fn group_type_definitions(
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
    pub(super) fn is_non_type_item(&self, item: &SourceItem) -> bool {
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
    pub(super) fn add_item_to_group(
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
    pub(super) fn assign_impl_blocks_to_types(
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
    pub(super) fn assign_single_impl_block(
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
    pub(super) fn try_add_impl_to_type_file(
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
    pub(super) fn extract_impl_target_type(&self, impl_block: &syn::ItemImpl) -> Option<String> {
        // Handle impl blocks like "impl SomeType" or "impl SomeTrait for SomeType"
        if let syn::Type::Path(type_path) = impl_block.self_ty.as_ref() {
            if let Some(segment) = type_path.path.segments.last() {
                return Some(segment.ident.to_string());
            }
        }
        None
    }

    /// Collects all impl blocks from the items
    pub(super) fn collect_impl_blocks(&self, items: &[SourceItem]) -> Vec<SourceItem> {
        items
            .iter()
            .filter(|item| matches!(item, SourceItem::Impl(_)))
            .cloned()
            .collect()
    }

    /// Removes empty original file entries from the groups
    pub(super) fn cleanup_empty_original_file_entry(
        &self,
        groups: &mut IndexMap<String, Vec<SourceItem>>,
    ) {
        if let Some(original_items) = groups.get(self.content().name()) {
            if original_items.is_empty() {
                groups.shift_remove(self.content().name());
            }
        }
    }

    /// Extracts the type name from various SourceItem types
    pub(super) fn extract_type_name_from_item(&self, item: &SourceItem) -> Option<String> {
        match item {
            SourceItem::Struct(s) => Some(s.ident.to_string()),
            SourceItem::Enum(e) => Some(e.ident.to_string()),
            SourceItem::Trait(t) => Some(t.ident.to_string()),
            SourceItem::Type(ty) => Some(ty.ident.to_string()),
            SourceItem::Union(u) => Some(u.ident.to_string()),
            _ => None,
        }
    }
}
