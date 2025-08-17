//! Incremental completion handler for SageMath syntax
//!
//! This module provides real-time autocompletion and syntax assistance for SageMath
//! code, handling the specific scenarios mentioned in the issue: incomplete power
//! operators and polynomial ring declarations.

use crate::advanced_source_map::Position;
use std::collections::HashMap;

/// Represents a completion suggestion with context
#[derive(Debug, Clone)]
pub struct CompletionItem {
    /// The text to insert
    pub insert_text: String,
    /// Human-readable label for the completion
    pub label: String,
    /// Detailed description of what this completion does
    pub detail: Option<String>,
    /// The kind of completion (function, variable, etc.)
    pub kind: CompletionKind,
    /// Whether this completion requires additional setup (like imports)
    pub requires_setup: bool,
}

#[derive(Debug, Clone)]
pub enum CompletionKind {
    Operator,
    Function,
    Constructor,
    Variable,
    Keyword,
}

/// Handles incremental completion for SageMath syntax
pub struct IncrementalCompletionHandler {
    /// Known SageMath functions and their signatures
    sage_functions: HashMap<String, Vec<CompletionItem>>,
    /// Common SageMath patterns and their completions
    pattern_completions: HashMap<String, Vec<CompletionItem>>,
}

impl Default for IncrementalCompletionHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl IncrementalCompletionHandler {
    /// Create a new completion handler with standard SageMath completions
    pub fn new() -> Self {
        let mut handler = Self {
            sage_functions: HashMap::new(),
            pattern_completions: HashMap::new(),
        };
        
        handler.initialize_completions();
        handler
    }

    /// Initialize the standard SageMath completions
    fn initialize_completions(&mut self) {
        // Power operator completion
        self.pattern_completions.insert(
            "*".to_string(),
            vec![CompletionItem {
                insert_text: "**".to_string(),
                label: "** (power operator)".to_string(),
                detail: Some("SageMath/Python exponentiation operator".to_string()),
                kind: CompletionKind::Operator,
                requires_setup: false,
            }]
        );

        // Polynomial ring completions
        self.pattern_completions.insert(
            "P.<x> = Pol".to_string(),
            vec![
                CompletionItem {
                    insert_text: "PolynomialRing(QQ)".to_string(),
                    label: "PolynomialRing(QQ)".to_string(),
                    detail: Some("Create polynomial ring over rationals".to_string()),
                    kind: CompletionKind::Constructor,
                    requires_setup: false,
                },
                CompletionItem {
                    insert_text: "PolynomialRing(ZZ)".to_string(),
                    label: "PolynomialRing(ZZ)".to_string(),
                    detail: Some("Create polynomial ring over integers".to_string()),
                    kind: CompletionKind::Constructor,
                    requires_setup: false,
                },
                CompletionItem {
                    insert_text: "PolynomialRing(GF(2))".to_string(),
                    label: "PolynomialRing(GF(2))".to_string(),
                    detail: Some("Create polynomial ring over finite field".to_string()),
                    kind: CompletionKind::Constructor,
                    requires_setup: false,
                },
            ]
        );

        // Variable declaration patterns
        self.pattern_completions.insert(
            ".<".to_string(),
            vec![
                CompletionItem {
                    insert_text: "<x> = PolynomialRing(QQ)".to_string(),
                    label: "<x> = PolynomialRing(QQ)".to_string(),
                    detail: Some("Single variable polynomial ring".to_string()),
                    kind: CompletionKind::Constructor,
                    requires_setup: false,
                },
                CompletionItem {
                    insert_text: "<x,y> = PolynomialRing(QQ)".to_string(),
                    label: "<x,y> = PolynomialRing(QQ)".to_string(),
                    detail: Some("Two variable polynomial ring".to_string()),
                    kind: CompletionKind::Constructor,
                    requires_setup: false,
                },
            ]
        );

        // Common SageMath functions
        let functions = vec![
            ("factor", "factor(n)", "Factor an integer or polynomial"),
            ("gcd", "gcd(a, b)", "Greatest common divisor"),
            ("lcm", "lcm(a, b)", "Least common multiple"),
            ("is_prime", "is_prime(n)", "Test if number is prime"),
            ("matrix", "matrix([[]])", "Create a matrix"),
            ("vector", "vector([])", "Create a vector"),
            ("Matrix", "Matrix([])", "Create a matrix (alternative)"),
            ("Vector", "Vector([])", "Create a vector (alternative)"),
        ];

        for (name, signature, description) in functions {
            self.sage_functions.insert(
                name.to_string(),
                vec![CompletionItem {
                    insert_text: signature.to_string(),
                    label: signature.to_string(),
                    detail: Some(description.to_string()),
                    kind: CompletionKind::Function,
                    requires_setup: false,
                }]
            );
        }
    }

    /// Get completion suggestions for a given input and cursor position
    pub fn get_completions(&self, input: &str, cursor_pos: Position) -> Vec<CompletionItem> {
        let mut completions = Vec::new();
        
        let lines: Vec<&str> = input.lines().collect();
        if cursor_pos.line as usize >= lines.len() {
            return completions;
        }
        
        let current_line = lines[cursor_pos.line as usize];
        let cursor_char = cursor_pos.character as usize;
        
        if cursor_char > current_line.len() {
            return completions;
        }
        
        let before_cursor = &current_line[..cursor_char];
        
        // Check for pattern-based completions
        completions.extend(self.get_pattern_completions(before_cursor));
        
        // Check for function-based completions
        completions.extend(self.get_function_completions(before_cursor));
        
        // Check for context-specific completions
        completions.extend(self.get_context_completions(before_cursor, current_line));
        
        completions
    }

    /// Get completions based on known patterns
    fn get_pattern_completions(&self, before_cursor: &str) -> Vec<CompletionItem> {
        let mut completions = Vec::new();
        
        for (pattern, items) in &self.pattern_completions {
            if before_cursor.ends_with(pattern) {
                completions.extend(items.clone());
            }
        }
        
        completions
    }

    /// Get completions for function names
    fn get_function_completions(&self, before_cursor: &str) -> Vec<CompletionItem> {
        let mut completions = Vec::new();
        
        // Find the last word before cursor
        let words: Vec<&str> = before_cursor.split_whitespace().collect();
        if let Some(last_word) = words.last() {
            // Check if it's a partial function name
            for (func_name, items) in &self.sage_functions {
                if func_name.starts_with(last_word) && func_name != *last_word {
                    completions.extend(items.clone());
                }
            }
        }
        
        completions
    }

    /// Get context-specific completions
    fn get_context_completions(&self, before_cursor: &str, current_line: &str) -> Vec<CompletionItem> {
        let mut completions = Vec::new();
        
        // Special case: completing after incomplete assignments
        if before_cursor.contains('=') && !before_cursor.contains("==") {
            // Check if this looks like a polynomial ring assignment
            if before_cursor.contains(".<") && !before_cursor.contains("> =") {
                completions.push(CompletionItem {
                    insert_text: "> = PolynomialRing(QQ)".to_string(),
                    label: "> = PolynomialRing(QQ)".to_string(),
                    detail: Some("Complete polynomial ring declaration".to_string()),
                    kind: CompletionKind::Constructor,
                    requires_setup: false,
                });
            }
        }
        
        // Rational number context
        if before_cursor.contains('/') && !current_line.contains("//") {
            completions.push(CompletionItem {
                insert_text: "".to_string(), // No insert needed, just explanation
                label: "Rational number".to_string(),
                detail: Some("SageMath automatically handles rational arithmetic".to_string()),
                kind: CompletionKind::Keyword,
                requires_setup: false,
            });
        }
        
        completions
    }

    /// Check if the current input can be automatically completed
    pub fn can_auto_complete(&self, input: &str, cursor_pos: Position) -> bool {
        let lines: Vec<&str> = input.lines().collect();
        if cursor_pos.line as usize >= lines.len() {
            return false;
        }
        
        let current_line = lines[cursor_pos.line as usize];
        let cursor_char = cursor_pos.character as usize;
        
        if cursor_char > current_line.len() {
            return false;
        }
        
        let before_cursor = &current_line[..cursor_char];
        
        // Auto-complete single * to **
        if before_cursor.ends_with("*") && !before_cursor.ends_with("**") {
            return true;
        }
        
        // Auto-complete polynomial ring patterns
        if before_cursor.ends_with("P.<x> = Pol") {
            return true;
        }
        
        false
    }

    /// Get the auto-completion for common patterns
    pub fn get_auto_completion(&self, input: &str, cursor_pos: Position) -> Option<String> {
        let lines: Vec<&str> = input.lines().collect();
        if cursor_pos.line as usize >= lines.len() {
            return None;
        }
        
        let current_line = lines[cursor_pos.line as usize];
        let cursor_char = cursor_pos.character as usize;
        
        if cursor_char > current_line.len() {
            return None;
        }
        
        let before_cursor = &current_line[..cursor_char];
        
        // Auto-complete * to **
        if before_cursor.ends_with("*") && !before_cursor.ends_with("**") {
            return Some("*".to_string()); // Add second *
        }
        
        // Auto-complete polynomial ring
        if before_cursor.ends_with("P.<x> = Pol") {
            return Some("ynomialRing(QQ)".to_string()); // Complete the rest
        }
        
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_power_operator_completion() {
        let handler = IncrementalCompletionHandler::new();
        
        let completions = handler.get_completions(
            "x = 2*",
            Position { line: 0, character: 6 }
        );
        
        assert!(!completions.is_empty());
        assert!(completions.iter().any(|c| c.insert_text == "**"));
    }

    #[test]
    fn test_polynomial_ring_completion() {
        let handler = IncrementalCompletionHandler::new();
        
        let completions = handler.get_completions(
            "P.<x> = Pol",
            Position { line: 0, character: 11 }
        );
        
        assert!(!completions.is_empty());
        assert!(completions.iter().any(|c| c.insert_text.contains("PolynomialRing")));
    }

    #[test]
    fn test_auto_completion_detection() {
        let handler = IncrementalCompletionHandler::new();
        
        assert!(handler.can_auto_complete(
            "x = 2*",
            Position { line: 0, character: 6 }
        ));
        
        assert!(handler.can_auto_complete(
            "P.<x> = Pol",
            Position { line: 0, character: 11 }
        ));
        
        assert!(!handler.can_auto_complete(
            "x = 2**3",
            Position { line: 0, character: 8 }
        ));
    }

    #[test]
    fn test_auto_completion_text() {
        let handler = IncrementalCompletionHandler::new();
        
        let completion = handler.get_auto_completion(
            "x = 2*",
            Position { line: 0, character: 6 }
        );
        
        assert_eq!(completion, Some("*".to_string()));
        
        let completion = handler.get_auto_completion(
            "P.<x> = Pol",
            Position { line: 0, character: 11 }
        );
        
        assert_eq!(completion, Some("ynomialRing(QQ)".to_string()));
    }

    #[test]
    fn test_function_name_completion() {
        let handler = IncrementalCompletionHandler::new();
        
        let completions = handler.get_completions(
            "fac",
            Position { line: 0, character: 3 }
        );
        
        assert!(completions.iter().any(|c| c.insert_text.contains("factor")));
    }
}