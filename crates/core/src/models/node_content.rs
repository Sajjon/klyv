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
    /// This is a complex method.
    ///
    /// The content is a NamedSourceItems, which contains the items of the Rust file
    /// and contains `items: Vec<SourceItem>`.
    ///
    /// This method might write to many files, depending on the items.
    ///
    /// Here are some examples of how it might write:
    /// - If `items` contains a single struct and many impls of that struct, we
    ///   write to a single file with the struct and all impls.
    /// - If `items` contains multiple structs, we write to a file for each struct
    ///   and its impls.
    /// - If `items` contains a mix of structs, enums, and (global) functions, we
    ///   write to a file for each struct and enum, and a separate file for the
    ///   global functions.
    fn write_to(&self, path: impl AsRef<Path>) -> Result<()> {
        let items = self.content().items();
        let base_path = path.as_ref();

        // Create the output directory if it doesn't exist
        if let Some(parent) = base_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| Error::bail(format!("Failed to create directory: {}", e)))?;
        }

        // Group items by their target file
        let grouped_items = self.group_items_by_target_file(items);

        // Write each group to its corresponding file
        for (file_name, group_items) in grouped_items {
            let target_file = if file_name == *self.content().name() {
                // Use the original path for items that stay in the main file
                base_path.to_path_buf()
            } else {
                // Create new file path for split items
                base_path.with_file_name(&file_name)
            };

            // For now, don't optimize use statements - just copy them all
            // TODO: Implement smarter use statement optimization
            self.write_items_to_file(&group_items, &target_file)?;
        }

        Ok(())
    }
}

impl RustFileContent {
    /// Group items by their target file name
    /// Returns a map where keys are file names and values are vectors of items for that file
    fn group_items_by_target_file(
        &self,
        items: &[SourceItem],
    ) -> std::collections::HashMap<String, Vec<SourceItem>> {
        use std::collections::HashMap;

        let mut groups: HashMap<String, Vec<SourceItem>> = HashMap::new();
        let mut impl_blocks: Vec<SourceItem> = Vec::new();
        let mut use_statements: Vec<SourceItem> = Vec::new();

        // Collect use statements separately
        for item in items {
            if matches!(item, SourceItem::Use(_)) {
                use_statements.push(item.clone());
            }
        }

        // First pass: group main types and collect impl blocks
        for item in items {
            match item {
                SourceItem::Struct(s) => {
                    let type_name = s.ident.to_string();
                    let file_name = format!("{}.rs", self.to_snake_case(&type_name));
                    let group = groups.entry(file_name).or_default();
                    // Add use statements first, then the struct
                    group.extend(use_statements.clone());
                    group.push(item.clone());
                }
                SourceItem::Enum(e) => {
                    let type_name = e.ident.to_string();
                    let file_name = format!("{}.rs", self.to_snake_case(&type_name));
                    let group = groups.entry(file_name).or_default();
                    // Add use statements first, then the enum
                    group.extend(use_statements.clone());
                    group.push(item.clone());
                }
                SourceItem::Trait(t) => {
                    let type_name = t.ident.to_string();
                    let file_name = format!("{}.rs", self.to_snake_case(&type_name));
                    let group = groups.entry(file_name).or_default();
                    // Add use statements first, then the trait
                    group.extend(use_statements.clone());
                    group.push(item.clone());
                }
                SourceItem::Type(ty) => {
                    let type_name = ty.ident.to_string();
                    let file_name = format!("{}.rs", self.to_snake_case(&type_name));
                    let group = groups.entry(file_name).or_default();
                    // Add use statements first, then the type
                    group.extend(use_statements.clone());
                    group.push(item.clone());
                }
                SourceItem::Union(u) => {
                    let type_name = u.ident.to_string();
                    let file_name = format!("{}.rs", self.to_snake_case(&type_name));
                    let group = groups.entry(file_name).or_default();
                    // Add use statements first, then the union
                    group.extend(use_statements.clone());
                    group.push(item.clone());
                }
                SourceItem::Impl(_) => {
                    // Collect impl blocks for second pass
                    impl_blocks.push(item.clone());
                }
                SourceItem::Use(_) => {
                    // Already handled above
                }
                _ => {
                    // Functions, macros, etc. go to the original file
                    groups
                        .entry(self.content().name().clone())
                        .or_default()
                        .push(item.clone());
                }
            }
        }

        // Second pass: assign impl blocks to their target types
        for impl_item in impl_blocks {
            if let SourceItem::Impl(impl_block) = &impl_item {
                let target_type = self.extract_impl_target_type(impl_block);

                if let Some(type_name) = target_type {
                    let file_name = format!("{}.rs", self.to_snake_case(&type_name));

                    // If we have a file for this type, add the impl there
                    if groups.contains_key(&file_name) {
                        groups.get_mut(&file_name).unwrap().push(impl_item);
                    } else {
                        // Otherwise, put it in the original file
                        groups
                            .entry(self.content().name().clone())
                            .or_default()
                            .push(impl_item);
                    }
                } else {
                    // Can't determine target type, put in original file
                    groups
                        .entry(self.content().name().clone())
                        .or_default()
                        .push(impl_item);
                }
            }
        }

        // Remove empty original file entry if it exists and is empty
        if let Some(original_items) = groups.get(self.content().name()) {
            if original_items.is_empty() {
                groups.remove(self.content().name());
            }
        }

        groups
    }

    /// Convert PascalCase to snake_case
    fn to_snake_case(&self, input: &str) -> String {
        let mut result = String::new();
        let mut prev_was_lowercase = false;

        for ch in input.chars() {
            if ch.is_uppercase() {
                if prev_was_lowercase && !result.is_empty() {
                    result.push('_');
                }
                result.push(ch.to_lowercase().next().unwrap());
                prev_was_lowercase = false;
            } else {
                result.push(ch);
                prev_was_lowercase = ch.is_lowercase();
            }
        }

        result
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
        let mut content = String::new();

        for (i, item) in items.iter().enumerate() {
            if i > 0 {
                content.push_str("\n\n");
            }

            // Convert the SourceItem back to Rust code
            let item_code = self.source_item_to_string(item);
            content.push_str(&item_code);
        }

        // Write to file
        std::fs::write(file_path, content).map_err(|e| {
            Error::bail(format!(
                "Failed to write file {}: {}",
                file_path.display(),
                e
            ))
        })?;
        Ok(())
    }

    /// Convert a SourceItem back to its string representation
    fn source_item_to_string(&self, item: &SourceItem) -> String {
        use quote::ToTokens;

        let token_stream = match item {
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
        };

        // Parse the token stream back to a syn::File to format it properly
        let formatted_code = if let Ok(file) = syn::parse2::<syn::File>(token_stream.clone()) {
            prettyplease::unparse(&file)
        } else {
            // Fallback to the original token stream if parsing fails
            token_stream.to_string()
        };

        // Convert #[doc = "..."] attributes back to /// doc comments
        self.convert_doc_attributes_to_comments(formatted_code)
    }

    /// Convert #[doc = "text"] attributes back to /// doc comment syntax
    fn convert_doc_attributes_to_comments(&self, code: String) -> String {
        let mut result = String::new();
        let chars = code.chars().collect::<Vec<_>>();
        let mut i = 0;

        while i < chars.len() {
            if chars[i] == '#' {
                // Look for the pattern # [doc = " or #[doc = "
                let mut j = i + 1;

                // Skip optional whitespace after #
                while j < chars.len() && chars[j].is_whitespace() && chars[j] != '\n' {
                    j += 1;
                }

                // Check for [doc = "
                if j + 8 < chars.len() && chars[j] == '[' {
                    let slice: String =
                        chars[j..std::cmp::min(j + 8, chars.len())].iter().collect();
                    if slice.starts_with("[doc = \"") {
                        // Found doc attribute start, find the closing quote and bracket
                        j += 8; // Start after [doc = "
                        let mut doc_content = String::new();

                        // Find the closing quote
                        while j < chars.len() && chars[j] != '"' {
                            doc_content.push(chars[j]);
                            j += 1;
                        }

                        if j < chars.len() && chars[j] == '"' {
                            j += 1; // Skip the quote

                            // Skip whitespace and find the closing bracket
                            while j < chars.len() && chars[j].is_whitespace() && chars[j] != '\n' {
                                j += 1;
                            }

                            if j < chars.len() && chars[j] == ']' {
                                // Successfully found complete doc attribute
                                result.push_str(&format!("///{}", doc_content));
                                i = j + 1; // Continue after the ]
                                continue;
                            }
                        }
                    }
                }
            }

            // Not a doc attribute or couldn't parse it, add the character as-is
            result.push(chars[i]);
            i += 1;
        }

        result
    }
}

/// Type alias for a directory content, which is a NodeContent containing a vector of FileSystemNode
pub type DirectoryContent = NodeContent<Vec<FileSystemNode>>;

/// Type alias for a Rust file content, which is a NodeContent containing NamedSourceItems
pub type RustFileContent = NodeContent<NamedSourceItems>;
