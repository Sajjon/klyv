use crate::prelude::*;

impl RustFileContent {
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
}
