use crate::prelude::*;

impl RustFileContent {
    /// Convert #[doc = "text"] attributes back to /// doc comment syntax
    pub fn convert_doc_attributes_to_comments(&self, code: String) -> String {
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
            let converted_text = format!("{}{}", Self::COMMENT_PREFIX, doc_content.content);
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
        if !slice.starts_with(Self::DOC_ATTR_PREFIX) {
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
}
