use std::path::PathBuf;

use crate::prelude::*;

/// Represents the content of a node in the file system
#[derive(Clone, Debug, Getters, Builder)]
pub struct NodeContent<C> {
    /// The name of the directory or file
    #[getset(get = "pub")]
    name: String,
    /// The path to the directory or file
    #[getset(get = "pub")]
    path: PathBuf,
    /// The content of the node, which can be a directory or a Rust file
    #[getset(get = "pub")]
    content: C,
}

impl FileWritable for DirectoryContent {
    fn write_to(&self, path: impl AsRef<Path>) -> Result<()> {
        self.content()
            .iter()
            .map(|node| node.write_to(path.as_ref().join(node.name())))
            .collect::<Result<Vec<()>>>()?;
        Ok(())
    }
}

impl FileWritable for RustFileContent {
    /// Writes Rust file content to one or more files based on item types.
    ///
    /// Types are split into separate files with their implementations,
    /// while keeping use statements and global items appropriately distributed.
    fn write_to(&self, path: impl AsRef<Path>) -> Result<()> {
        let base_path = path.as_ref();

        // Ensure output directory exists
        self.ensure_output_directory_exists(base_path)?;

        // Group and write items to their target files
        self.process_and_write_grouped_items(base_path)
    }
}

/// Helper struct for doc attribute conversion results
struct DocConversion {
    converted_text: String,
    next_position: usize,
}

/// Helper struct for doc attribute content parsing
struct DocContent {
    content: String,
    end_position: usize,
}

impl RustFileContent {
    /// Creates the output directory if it doesn't exist
    fn ensure_output_directory_exists(&self, base_path: &Path) -> Result<()> {
        if let Some(parent) = base_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| Error::bail(format!("Failed to create directory: {}", e)))?;
        }
        Ok(())
    }

    /// Groups items by target file and writes each group to its file
    fn process_and_write_grouped_items(&self, base_path: &Path) -> Result<()> {
        let items = self.content().items();
        let grouped_items = self.group_items_by_target_file(items);

        // Write each group to its corresponding file
        for (file_name, group_items) in grouped_items {
            let target_file = self.determine_target_file_path(base_path, &file_name);
            self.write_items_to_file(&group_items, &target_file)?;
        }
        Ok(())
    }

    /// Determines the target file path for a given file name
    fn determine_target_file_path(&self, base_path: &Path, file_name: &str) -> PathBuf {
        if file_name == *self.content().name() {
            // Use the original path for items that stay in the main file
            base_path.to_path_buf()
        } else {
            // Create new file path for split items
            base_path.with_file_name(file_name)
        }
    }

    /// Group items by their target file name
    /// Returns a map where keys are file names and values are vectors of items for that file
    fn group_items_by_target_file(
        &self,
        items: &[SourceItem],
    ) -> std::collections::HashMap<String, Vec<SourceItem>> {
        use std::collections::HashMap;

        let mut groups: HashMap<String, Vec<SourceItem>> = HashMap::new();
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
        groups: &mut std::collections::HashMap<String, Vec<SourceItem>>,
        items: &[SourceItem],
        use_statements: &[SourceItem],
    ) {
        for item in items {
            if let Some(type_name) = self.extract_type_name_from_item(item) {
                let file_name = format!("{}.rs", self.to_snake_case(&type_name));
                self.add_item_to_group(groups, file_name, use_statements, item);
            } else if self.is_non_type_item(item) {
                // Functions, macros, etc. go to the original file
                self.add_item_to_original_file(groups, item);
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
        groups: &mut std::collections::HashMap<String, Vec<SourceItem>>,
        file_name: String,
        use_statements: &[SourceItem],
        item: &SourceItem,
    ) {
        let group = groups.entry(file_name).or_default();
        // Add use statements first, then the item
        group.extend(use_statements.iter().cloned());
        group.push(item.clone());
    }

    /// Adds an item to the original file group
    fn add_item_to_original_file(
        &self,
        groups: &mut std::collections::HashMap<String, Vec<SourceItem>>,
        item: &SourceItem,
    ) {
        groups
            .entry(self.content().name().clone())
            .or_default()
            .push(item.clone());
    }

    /// Assigns impl blocks to their corresponding type files
    fn assign_impl_blocks_to_types(
        &self,
        groups: &mut std::collections::HashMap<String, Vec<SourceItem>>,
        items: &[SourceItem],
    ) {
        let impl_blocks = self.collect_impl_blocks(items);

        for impl_item in impl_blocks {
            self.assign_single_impl_block(groups, &impl_item);
        }
    }

    /// Collects all impl blocks from the items
    fn collect_impl_blocks(&self, items: &[SourceItem]) -> Vec<SourceItem> {
        items
            .iter()
            .filter(|item| matches!(item, SourceItem::Impl(_)))
            .cloned()
            .collect()
    }

    /// Assigns a single impl block to its target type file
    fn assign_single_impl_block(
        &self,
        groups: &mut std::collections::HashMap<String, Vec<SourceItem>>,
        impl_item: &SourceItem,
    ) {
        if let SourceItem::Impl(impl_block) = impl_item {
            let target_type = self.extract_impl_target_type(impl_block);

            if let Some(type_name) = target_type {
                let file_name = format!("{}.rs", self.to_snake_case(&type_name));
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
        groups: &mut std::collections::HashMap<String, Vec<SourceItem>>,
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

    /// Removes empty original file entries from the groups
    fn cleanup_empty_original_file_entry(
        &self,
        groups: &mut std::collections::HashMap<String, Vec<SourceItem>>,
    ) {
        if let Some(original_items) = groups.get(self.content().name()) {
            if original_items.is_empty() {
                groups.remove(self.content().name());
            }
        }
    }

    /// Convert PascalCase to snake_case
    fn to_snake_case(&self, input: &str) -> String {
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

    /// Helper method to write a collection of items to a file
    fn write_items_to_file(&self, items: &[SourceItem], file_path: &Path) -> Result<()> {
        let content = self.build_file_content(items);
        self.write_content_to_file(&content, file_path)
    }

    /// Builds the complete file content from a collection of items
    fn build_file_content(&self, items: &[SourceItem]) -> String {
        let mut content = String::new();

        for (i, item) in items.iter().enumerate() {
            if i > 0 {
                let spacing = self.determine_item_spacing(&items[i - 1], item);
                content.push_str(&spacing);
            }

            let item_code = self.source_item_to_string(item);
            // Strip trailing newlines to have full control over spacing
            let trimmed_code = item_code.trim_end();
            content.push_str(trimmed_code);
        }

        // Ensure the file ends with a newline
        if !content.is_empty() {
            content.push('\n');
        }

        content
    }

    /// Determines the appropriate spacing between two consecutive items
    fn determine_item_spacing(&self, prev_item: &SourceItem, current_item: &SourceItem) -> String {
        match (prev_item, current_item) {
            // Use statements should have single newlines between them
            (SourceItem::Use(_), SourceItem::Use(_)) => "\n".to_string(),
            // Everything else gets double newlines for better separation
            _ => "\n\n".to_string(),
        }
    }
    /// Writes content string to the specified file path
    fn write_content_to_file(&self, content: &str, file_path: &Path) -> Result<()> {
        std::fs::write(file_path, content).map_err(|e| {
            Error::bail(format!(
                "Failed to write file {}: {}",
                file_path.display(),
                e
            ))
        })
    }

    /// Convert a SourceItem back to its string representation
    fn source_item_to_string(&self, item: &SourceItem) -> String {
        let token_stream = self.convert_item_to_token_stream(item);
        let formatted_code = self.format_token_stream(token_stream);

        // Convert #[doc = "..."] attributes back to /// doc comments
        self.convert_doc_attributes_to_comments(formatted_code)
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

    /// Convert #[doc = "text"] attributes back to /// doc comment syntax
    fn convert_doc_attributes_to_comments(&self, code: String) -> String {
        let mut result = String::new();
        let chars = code.chars().collect::<Vec<_>>();

        self.process_all_characters(&chars, &mut result);
        result
    }

    /// Processes all characters in the code for doc attribute conversion
    fn process_all_characters(&self, chars: &[char], result: &mut String) {
        let mut i = 0;

        while i < chars.len() {
            i = self.process_single_character_position(chars, result, i);
        }
    }

    /// Processes a single character position and returns the next position
    fn process_single_character_position(
        &self,
        chars: &[char],
        result: &mut String,
        i: usize,
    ) -> usize {
        if let Some(doc_conversion) = self.try_convert_doc_attribute(chars, i) {
            // Successfully converted a doc attribute
            result.push_str(&doc_conversion.converted_text);
            doc_conversion.next_position
        } else {
            // Not a doc attribute, add the character as-is
            result.push(chars[i]);
            i + 1
        }
    }

    /// Attempts to convert a doc attribute at the given position
    fn try_convert_doc_attribute(&self, chars: &[char], start_pos: usize) -> Option<DocConversion> {
        if !self.starts_with_hash(chars, start_pos) {
            return None;
        }

        let pos_after_hash = start_pos + 1;
        let pos_after_whitespace = self.skip_whitespace_except_newline(chars, pos_after_hash);

        self.try_parse_doc_attribute(chars, pos_after_whitespace)
    }

    /// Checks if the character at the given position is a hash
    fn starts_with_hash(&self, chars: &[char], pos: usize) -> bool {
        chars[pos] == '#'
    }

    /// Attempts to parse a doc attribute and create a DocConversion
    fn try_parse_doc_attribute(&self, chars: &[char], pos: usize) -> Option<DocConversion> {
        if let Some(doc_content) = self.extract_doc_attribute_content(chars, pos) {
            let converted_text = format!("///{}", doc_content.content);
            Some(DocConversion {
                converted_text,
                next_position: doc_content.end_position,
            })
        } else {
            None
        }
    }

    /// Skips whitespace characters except newlines
    fn skip_whitespace_except_newline(&self, chars: &[char], mut pos: usize) -> usize {
        while pos < chars.len() && chars[pos].is_whitespace() && chars[pos] != '\n' {
            pos += 1;
        }
        pos
    }

    /// Extracts content from a doc attribute if found
    fn extract_doc_attribute_content(&self, chars: &[char], pos: usize) -> Option<DocContent> {
        if pos + 8 >= chars.len() || chars[pos] != '[' {
            return None;
        }

        // Check for exact pattern "[doc = \""
        let slice: String = chars[pos..std::cmp::min(pos + 8, chars.len())]
            .iter()
            .collect();
        if !slice.starts_with("[doc = \"") {
            return None;
        }

        self.parse_doc_attribute_content(chars, pos + 8)
    }

    /// Parses the content inside a doc attribute
    fn parse_doc_attribute_content(&self, chars: &[char], start_pos: usize) -> Option<DocContent> {
        let (content, pos_after_quote) = self.extract_content_until_quote(chars, start_pos)?;
        let pos_after_whitespace = self.skip_whitespace_except_newline(chars, pos_after_quote);

        self.validate_closing_bracket(chars, pos_after_whitespace, content)
    }

    /// Extracts content until the closing quote
    fn extract_content_until_quote(
        &self,
        chars: &[char],
        start_pos: usize,
    ) -> Option<(String, usize)> {
        let mut pos = start_pos;
        let mut content = String::new();

        // Find the closing quote
        while pos < chars.len() && chars[pos] != '"' {
            content.push(chars[pos]);
            pos += 1;
        }

        if pos >= chars.len() || chars[pos] != '"' {
            None // No closing quote found
        } else {
            Some((content, pos + 1)) // Return content and position after quote
        }
    }

    /// Validates the closing bracket and creates DocContent if valid
    fn validate_closing_bracket(
        &self,
        chars: &[char],
        pos: usize,
        content: String,
    ) -> Option<DocContent> {
        if pos < chars.len() && chars[pos] == ']' {
            Some(DocContent {
                content,
                end_position: pos + 1,
            })
        } else {
            None // No closing bracket found
        }
    }
}

/// Type alias for a directory content, which is a NodeContent containing a vector of FileSystemNode
pub type DirectoryContent = NodeContent<Vec<FileSystemNode>>;

/// Type alias for a Rust file content, which is a NodeContent containing NamedSourceItems
pub type RustFileContent = NodeContent<NamedSourceItems>;
