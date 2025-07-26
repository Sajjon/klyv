use std::collections::HashMap;
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

        // Check if this is a special lib.rs case
        if self.is_lib_rs_special_case() {
            println!("DEBUG: Detected lib.rs special case");
            self.handle_lib_rs_special_case(base_path)?;
        } else if self.is_main_rs_special_case() {
            println!("DEBUG: Detected main.rs special case");
            self.handle_main_rs_special_case(base_path)?;
        } else {
            // Standard file splitting logic
            let items = self.content().items();
            let grouped_items = self.group_items_by_target_file(items);

            println!("DEBUG: Found {} item groups", grouped_items.len());

            self.process_and_write_grouped_items(base_path)?;

            // Update mod.rs if multiple files were created
            if grouped_items.len() > 1 {
                println!("DEBUG: Multiple files created, updating mod.rs");
                self.update_mod_file(base_path, &grouped_items)?;
            } else {
                println!("DEBUG: Only one file, skipping mod.rs update");
            }
        }

        Ok(())
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
            self.handle_original_file_path(base_path)
        } else {
            // Create new file path for split items
            self.create_split_file_path(base_path, file_name)
        }
    }

    /// Handles the original file path logic
    fn handle_original_file_path(&self, base_path: &Path) -> PathBuf {
        if base_path.is_dir() {
            // If output is a directory, place the original file in it
            base_path.join(self.content().name())
        } else {
            // If output is a file path, use it directly
            base_path.to_path_buf()
        }
    }

    /// Creates a path for split files
    fn create_split_file_path(&self, base_path: &Path, file_name: &str) -> PathBuf {
        if base_path.is_dir() {
            // If output is a directory, place the new file in it
            base_path.join(file_name)
        } else {
            // If output is a file path, replace the file name component
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

    /// Updates or creates mod.rs file with module declarations for split files
    fn update_mod_file(
        &self,
        base_path: &Path,
        grouped_items: &HashMap<String, Vec<SourceItem>>,
    ) -> Result<()> {
        let mod_file_path = if base_path.is_dir() {
            base_path.join("mod.rs")
        } else {
            base_path.parent().unwrap_or(Path::new(".")).join("mod.rs")
        };

        let module_names = self.extract_module_names_from_groups(grouped_items);
        let existing_modules = self.read_existing_modules(&mod_file_path)?;
        let combined_modules = self.combine_module_lists(existing_modules, module_names);
        self.write_mod_file_content(&mod_file_path, combined_modules)?;

        Ok(())
    }

    /// Extracts module names from the grouped items, excluding the main file
    fn extract_module_names_from_groups(
        &self,
        grouped_items: &HashMap<String, Vec<SourceItem>>,
    ) -> Vec<String> {
        grouped_items
            .keys()
            .filter(|&name| name != self.content().name())
            .map(|name| name.trim_end_matches(".rs").to_string())
            .collect()
    }

    /// Reads existing module declarations from mod.rs if it exists
    fn read_existing_modules(&self, mod_file_path: &Path) -> Result<Vec<String>> {
        if mod_file_path.exists() {
            let content = std::fs::read_to_string(mod_file_path).map_err(|e| {
                Error::bail(format!(
                    "Failed to read mod.rs file {}: {}",
                    mod_file_path.display(),
                    e
                ))
            })?;
            Ok(self.parse_module_declarations(&content))
        } else {
            Ok(vec![])
        }
    }

    /// Parses mod declarations from file content
    fn parse_module_declarations(&self, content: &str) -> Vec<String> {
        content
            .lines()
            .filter_map(|line| {
                let trimmed = line.trim();
                if trimmed.starts_with("mod ") && trimmed.ends_with(';') {
                    Some(trimmed[4..trimmed.len() - 1].to_string())
                } else {
                    None
                }
            })
            .collect()
    }

    /// Combines existing and new module lists, removing duplicates
    fn combine_module_lists(&self, existing: Vec<String>, new: Vec<String>) -> Vec<String> {
        let mut combined: Vec<String> = existing;
        for module in new {
            if !combined.contains(&module) {
                combined.push(module);
            }
        }
        combined.sort();
        combined
    }

    /// Writes the mod.rs file content with module declarations and re-exports
    fn write_mod_file_content(&self, mod_file_path: &Path, modules: Vec<String>) -> Result<()> {
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

    /// Checks if this is a lib.rs file that should receive special treatment
    fn is_lib_rs_special_case(&self) -> bool {
        let file_name = self.content().name();
        file_name == "lib.rs" && self.has_multiple_types_or_functions()
    }

    /// Checks if this is a main.rs file that should receive special treatment
    fn is_main_rs_special_case(&self) -> bool {
        let file_name = self.content().name();
        file_name == "main.rs" && self.has_multiple_types_or_functions_for_main()
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

    /// Checks if the main.rs file has types or functions that warrant special organization
    /// For main.rs, we want to organize if there are types or non-main functions
    fn has_multiple_types_or_functions_for_main(&self) -> bool {
        let items = self.content().items();
        let type_count = items.iter().filter(|item| self.is_type_item(item)).count();
        let non_main_function_count = items
            .iter()
            .filter(|item| matches!(item, SourceItem::Function(f) if f.sig.ident != "main"))
            .count();

        type_count > 0 || non_main_function_count > 0
    }

    /// Handles the special lib.rs case by organizing into types and logic folders
    fn handle_lib_rs_special_case(&self, base_path: &Path) -> Result<()> {
        let items = self.content().items();
        let (type_items, logic_items, other_items) = self.categorize_lib_rs_items(items);

        // Create types folder and files if there are type items
        if !type_items.is_empty() {
            let types_dir = base_path.join("types");
            std::fs::create_dir_all(&types_dir)
                .map_err(|e| Error::bail(format!("Failed to create types directory: {}", e)))?;
            self.write_organized_items(&type_items, &types_dir, "types")?;
            self.create_types_mod_rs(&types_dir, &type_items)?;
        }

        // Create logic folder and files if there are function items
        if !logic_items.is_empty() {
            let logic_dir = base_path.join("logic");
            std::fs::create_dir_all(&logic_dir)
                .map_err(|e| Error::bail(format!("Failed to create logic directory: {}", e)))?;
            self.write_organized_items(&logic_items, &logic_dir, "logic")?;
            self.create_logic_mod_rs(&logic_dir, &logic_items)?;
        }

        // Create the new lib.rs with prelude module
        self.create_lib_rs_with_prelude(
            base_path,
            !type_items.is_empty(),
            !logic_items.is_empty(),
            &other_items,
        )?;

        Ok(())
    }

    /// Categorizes items into types, logic (functions), and other items
    fn categorize_lib_rs_items(
        &self,
        items: &[SourceItem],
    ) -> (Vec<SourceItem>, Vec<SourceItem>, Vec<SourceItem>) {
        let mut type_items = Vec::new();
        let mut logic_items = Vec::new();
        let mut other_items = Vec::new();

        for item in items {
            match item {
                SourceItem::Function(_) => logic_items.push(item.clone()),
                _ if self.is_type_item(item) => type_items.push(item.clone()),
                _ => other_items.push(item.clone()),
            }
        }

        (type_items, logic_items, other_items)
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
                SourceItem::Function(f) if f.sig.ident == "main" => {
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

    /// Checks if an item is a type (struct, enum, trait, type alias, union, impl)
    fn is_type_item(&self, item: &SourceItem) -> bool {
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

    /// Handles the special main.rs case by organizing into simple flat structure
    fn handle_main_rs_special_case(&self, base_path: &Path) -> Result<()> {
        let items = self.content().items();
        let (type_items, function_items, main_items) = self.categorize_main_rs_items_simple(items);

        // Write type items to individual files (similar to standard behavior)
        if !type_items.is_empty() {
            let grouped_items = self.group_items_by_target_file(&type_items);
            for (file_name, group_items) in grouped_items {
                let target_file = base_path.join(&file_name);
                let content = self.build_organized_file_content(&group_items, "main");
                self.write_content_to_file(&content, &target_file)?;
            }
        }

        // Write functions to utils.rs (or similar)
        if !function_items.is_empty() {
            let target_file = base_path.join("utils.rs");
            let content = self.build_organized_file_content(&function_items, "main");
            self.write_content_to_file(&content, &target_file)?;
        }

        // Create the new main.rs with module declarations and main function
        self.create_simple_main_rs(
            base_path,
            !type_items.is_empty(),
            !function_items.is_empty(),
            &main_items,
        )?;

        // Create mod.rs if we have multiple files
        if !type_items.is_empty() || !function_items.is_empty() {
            self.create_main_mod_rs(base_path, &type_items, !function_items.is_empty())?;
        }

        Ok(())
    }

    /// Writes organized items to separate files in the given directory
    fn write_organized_items(
        &self,
        items: &[SourceItem],
        dir: &Path,
        category: &str,
    ) -> Result<()> {
        match category {
            "logic" => {
                // For logic items (functions), group them all into functions.rs
                let target_file = dir.join("functions.rs");
                let content = self.build_organized_file_content(items, category);
                self.write_content_to_file(&content, &target_file)?;
            }
            "cli" | "core" => {
                // For CLI and core items, separate types and functions
                let mut type_items = Vec::new();
                let mut function_items = Vec::new();

                for item in items {
                    if self.is_type_item(item) {
                        type_items.push(item.clone());
                    } else if matches!(item, SourceItem::Function(_)) {
                        function_items.push(item.clone());
                    }
                }

                // Write types to individual files
                if !type_items.is_empty() {
                    let grouped_items = self.group_items_by_target_file(&type_items);
                    for (file_name, group_items) in grouped_items {
                        let target_file = dir.join(&file_name);
                        let content = self.build_organized_file_content(&group_items, category);
                        self.write_content_to_file(&content, &target_file)?;
                    }
                }

                // Write functions to functions.rs
                if !function_items.is_empty() {
                    let target_file = dir.join("functions.rs");
                    let content = self.build_organized_file_content(&function_items, category);
                    self.write_content_to_file(&content, &target_file)?;
                }
            }
            _ => {
                // For other categories (like "types"), use the standard grouping
                let grouped_items = self.group_items_by_target_file(items);
                for (file_name, group_items) in grouped_items {
                    let target_file = dir.join(&file_name);
                    let content = self.build_organized_file_content(&group_items, category);
                    self.write_content_to_file(&content, &target_file)?;
                }
            }
        }

        Ok(())
    }

    /// Builds file content for organized items with proper prelude import
    fn build_organized_file_content(&self, items: &[SourceItem], _category: &str) -> String {
        let mut content = String::new();

        // Add prelude import at the top
        content.push_str("use crate::prelude::*;\n\n");

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

    /// Creates mod.rs for the types directory
    fn create_types_mod_rs(&self, types_dir: &Path, items: &[SourceItem]) -> Result<()> {
        let grouped_items = self.group_items_by_target_file(items);
        let module_names = self.extract_module_names_for_organized_items(&grouped_items);
        self.write_mod_file_content(&types_dir.join("mod.rs"), module_names)?;
        Ok(())
    }

    /// Creates mod.rs for the logic directory  
    fn create_logic_mod_rs(&self, logic_dir: &Path, items: &[SourceItem]) -> Result<()> {
        if !items.is_empty() {
            // For logic items, we always create functions.rs
            let module_names = vec!["functions".to_string()];
            self.write_mod_file_content(&logic_dir.join("mod.rs"), module_names)?;
        }
        Ok(())
    }

    /// Extracts module names for organized items (doesn't filter out main file name)
    fn extract_module_names_for_organized_items(
        &self,
        grouped_items: &HashMap<String, Vec<SourceItem>>,
    ) -> Vec<String> {
        let mut module_names: Vec<String> = grouped_items
            .keys()
            .map(|name| name.trim_end_matches(".rs").to_string())
            .collect();
        module_names.sort();
        module_names
    }

    /// Creates the new lib.rs file with prelude module structure
    fn create_lib_rs_with_prelude(
        &self,
        base_path: &Path,
        has_types: bool,
        has_logic: bool,
        other_items: &[SourceItem],
    ) -> Result<()> {
        let lib_rs_path = if base_path.is_dir() {
            base_path.join("lib.rs")
        } else {
            base_path.to_path_buf()
        };

        let mut content = String::new();

        // Add module declarations
        if has_logic {
            content.push_str("mod logic;\n");
        }
        if has_types {
            content.push_str("mod types;\n");
        }

        // Add any remaining items (like use statements)
        if !other_items.is_empty() {
            content.push('\n');
            for (i, item) in other_items.iter().enumerate() {
                if i > 0 {
                    let spacing = self.determine_item_spacing(&other_items[i - 1], item);
                    content.push_str(&spacing);
                }
                content.push_str(&self.source_item_to_string(item));
                content.push('\n');
            }
        }

        // Add prelude module
        content.push_str("\npub mod prelude {\n");
        if has_logic {
            content.push_str("    pub use crate::logic::*;\n");
        }
        if has_types {
            content.push_str("    pub use crate::types::*;\n");
        }

        // Add common external crates if we detect this is a fresh lib.rs
        if self.should_add_common_imports(other_items) {
            content.push('\n');
            content.push_str("    pub use std::{\n");
            content.push_str("        collections::HashMap,\n");
            content.push_str("        path::{Path, PathBuf},\n");
            content.push_str("    };\n");
        }

        content.push_str("}\n");

        self.write_content_to_file(&content, &lib_rs_path)?;
        Ok(())
    }

    /// Checks if we should add common imports (when there are minimal existing imports)
    fn should_add_common_imports(&self, other_items: &[SourceItem]) -> bool {
        // Add common imports if there are few or no existing use statements
        let use_count = other_items
            .iter()
            .filter(|item| matches!(item, SourceItem::Use(_)))
            .count();
        use_count <= 2 // Only if there are 2 or fewer existing use statements
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
            base_path.join("main.rs")
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
            module_names.push("utils".to_string());
        }

        if !module_names.is_empty() {
            module_names.sort();
            let mod_rs_path = base_path.join("mod.rs");
            self.write_mod_file_content(&mod_rs_path, module_names)?;
        }

        Ok(())
    }
}

/// Type alias for a directory content, which is a NodeContent containing a vector of FileSystemNode
pub type DirectoryContent = NodeContent<Vec<FileSystemNode>>;

/// Type alias for a Rust file content, which is a NodeContent containing NamedSourceItems
pub type RustFileContent = NodeContent<NamedSourceItems>;
