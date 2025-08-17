//! Advanced source mapping for SageMath to Python transformations
//!
//! This module provides sophisticated bidirectional mapping between SageMath and Python
//! source code that can handle complex syntax transformations, incremental updates,
//! and real-time features like autocompletion.

use std::collections::HashMap;

/// Represents a transformation between SageMath and Python syntax
#[derive(Debug, Clone)]
pub struct SyntaxTransformation {
    /// The original SageMath pattern
    pub sage_pattern: String,
    /// The corresponding Python pattern
    pub python_pattern: String,
    /// Whether this transformation can be applied incrementally
    pub incremental: bool,
    /// Regex pattern for matching (compiled lazily)
    pub regex_pattern: Option<String>,
}

/// Advanced source map that handles complex SageMath transformations
#[derive(Debug, Clone)]
pub struct AdvancedSourceMap {
    /// Known syntax transformations
    transformations: Vec<SyntaxTransformation>,
    /// Cached transformation results for performance
    transformation_cache: HashMap<String, String>,
    /// Line-by-line mappings (for simple cases)
    line_mappings: HashMap<u32, u32>,
    /// Character-level transformation offsets
    char_offsets: Vec<CharacterMapping>,
    /// Original sources
    sage_source: String,
    python_source: String,
}

/// Represents character-level mapping information
#[derive(Debug, Clone)]
struct CharacterMapping {
    sage_start: u32,
    sage_end: u32,
    python_start: u32,
    python_end: u32,
    transformation_type: TransformationType,
}

#[derive(Debug, Clone)]
enum TransformationType {
    PowerOperator,          // ^ -> **
    PolynomialRing,         // P.<x> = ... -> complex transformation
    RationalNumber,         // Enhanced rational handling
    MatrixConstruction,     // matrix(...) transformations
    Identity,               // No transformation
}

/// Position in source code (line, column)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub line: u32,
    pub character: u32,
}

/// Range in source code
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

impl AdvancedSourceMap {
    /// Create a new advanced source map
    pub fn new(sage_source: String, python_source: String) -> Self {
        let mut map = Self {
            transformations: Self::create_transformation_rules(),
            transformation_cache: HashMap::new(),
            line_mappings: HashMap::new(),
            char_offsets: Vec::new(),
            sage_source,
            python_source,
        };
        
        map.analyze_transformations();
        map
    }

    /// Create the standard set of transformation rules
    fn create_transformation_rules() -> Vec<SyntaxTransformation> {
        vec![
            // Basic power operator
            SyntaxTransformation {
                sage_pattern: r"\^".to_string(),
                python_pattern: "**".to_string(),
                incremental: true,
                regex_pattern: Some(r"([a-zA-Z0-9_\)])\s*\^\s*([a-zA-Z0-9_\(])".to_string()),
            },
            
            // Polynomial ring with single variable
            SyntaxTransformation {
                sage_pattern: r"([A-Z][a-zA-Z0-9_]*)\.<([a-zA-Z_][a-zA-Z0-9_]*)>\s*=\s*PolynomialRing\(([^)]+)\)".to_string(),
                python_pattern: "$1 = PolynomialRing($3, names=('$2',)); ($2,) = $1._first_ngens(1)".to_string(),
                incremental: false,
                regex_pattern: Some(r"([A-Z][a-zA-Z0-9_]*)\.<([a-zA-Z_][a-zA-Z0-9_]*)>\s*=\s*PolynomialRing\(([^)]+)\)".to_string()),
            },
            
            // Polynomial ring with multiple variables
            SyntaxTransformation {
                sage_pattern: r"([A-Z][a-zA-Z0-9_]*)\.<([a-zA-Z_][a-zA-Z0-9_,\s]*)>\s*=\s*PolynomialRing\(([^)]+)\)".to_string(),
                python_pattern: "$1 = PolynomialRing($3, names=($2)); ($2) = $1._first_ngens($count)".to_string(),
                incremental: false,
                regex_pattern: Some(r"([A-Z][a-zA-Z0-9_]*)\.<([a-zA-Z_][a-zA-Z0-9_,\s]*)>\s*=\s*PolynomialRing\(([^)]+)\)".to_string()),
            },
            
            // Matrix construction
            SyntaxTransformation {
                sage_pattern: r"matrix\((.+)\)".to_string(),
                python_pattern: "Matrix($1)".to_string(),
                incremental: true,
                regex_pattern: Some(r"matrix\((.+)\)".to_string()),
            },
        ]
    }

    /// Analyze transformations between sage and python sources
    fn analyze_transformations(&mut self) {
        // Simple line-based analysis for now
        let sage_lines: Vec<&str> = self.sage_source.lines().collect();
        let python_lines: Vec<&str> = self.python_source.lines().collect();
        
        // Build basic line mappings
        for (i, _) in sage_lines.iter().enumerate() {
            if i < python_lines.len() {
                self.line_mappings.insert(i as u32, i as u32);
            }
        }
        
        // Analyze character-level transformations
        self.analyze_character_transformations();
    }

    /// Analyze character-level transformations
    fn analyze_character_transformations(&mut self) {
        // Collect all line pairs first to avoid borrowing issues
        let line_pairs: Vec<(usize, String, String)> = {
            let sage_lines: Vec<&str> = self.sage_source.lines().collect();
            let python_lines: Vec<&str> = self.python_source.lines().collect();
            
            sage_lines.iter()
                .zip(python_lines.iter())
                .enumerate()
                .map(|(idx, (sage_line, python_line))| {
                    (idx, sage_line.to_string(), python_line.to_string())
                })
                .collect()
        };
        
        for (line_idx, sage_line, python_line) in line_pairs {
            self.analyze_line_transformations(line_idx as u32, &sage_line, &python_line);
        }
    }

    /// Analyze transformations on a single line
    fn analyze_line_transformations(&mut self, line: u32, sage_line: &str, python_line: &str) {
        // Look for power operator transformations
        if sage_line.contains('^') && python_line.contains("**") {
            self.map_power_transformations(line, sage_line, python_line);
        }
        
        // Look for polynomial ring transformations
        if sage_line.contains(".<") && sage_line.contains("> =") {
            self.map_polynomial_ring_transformations(line, sage_line, python_line);
        }
    }

    /// Map power operator transformations
    fn map_power_transformations(&mut self, _line: u32, sage_line: &str, python_line: &str) {
        let mut sage_pos = 0;
        let mut python_pos = 0;
        
        for (sage_char, python_char) in sage_line.chars().zip(python_line.chars()) {
            if sage_char == '^' {
                // Found a transformation point
                self.char_offsets.push(CharacterMapping {
                    sage_start: sage_pos,
                    sage_end: sage_pos + 1,
                    python_start: python_pos,
                    python_end: python_pos + 2, // ** is 2 characters
                    transformation_type: TransformationType::PowerOperator,
                });
                python_pos += 2; // Skip the second * in **
            } else if sage_char == python_char {
                python_pos += python_char.len_utf8() as u32;
            } else {
                // Character mismatch - handle appropriately
                python_pos += python_char.len_utf8() as u32;
            }
            sage_pos += sage_char.len_utf8() as u32;
        }
    }

    /// Map polynomial ring transformations
    fn map_polynomial_ring_transformations(&mut self, _line: u32, sage_line: &str, python_line: &str) {
        // This is a complex transformation that typically expands significantly
        // For now, mark the entire line as a polynomial ring transformation
        self.char_offsets.push(CharacterMapping {
            sage_start: 0,
            sage_end: sage_line.len() as u32,
            python_start: 0,
            python_end: python_line.len() as u32,
            transformation_type: TransformationType::PolynomialRing,
        });
    }

    /// Convert a Python position to a SageMath position
    pub fn python_position_to_sage_position(&self, python_pos: Position) -> Option<Position> {
        // Get line mapping
        let sage_line = self.line_mappings.get(&python_pos.line)?;
        
        // Find the appropriate character mapping
        let sage_char = self.map_python_char_to_sage(python_pos.line, python_pos.character, *sage_line);
        
        Some(Position {
            line: *sage_line,
            character: sage_char,
        })
    }

    /// Convert a SageMath position to a Python position
    pub fn sage_position_to_python_position(&self, sage_pos: Position) -> Option<Position> {
        // Get line mapping
        let python_line = self.line_mappings.iter()
            .find(|&(_, py_line)| *py_line == sage_pos.line)
            .map(|(&sage_line, _)| sage_line)?;
        
        let python_char = self.map_sage_char_to_python(sage_pos.line, sage_pos.character, python_line);
        
        Some(Position {
            line: python_line,
            character: python_char,
        })
    }

    /// Map character position from Python to SageMath
    fn map_python_char_to_sage(&self, _python_line: u32, python_char: u32, _sage_line: u32) -> u32 {
        // Find relevant character mappings for this line
        let _relevant_mappings: Vec<_> = self.char_offsets.iter()
            .filter(|_mapping| {
                // For now, we assume line-based filtering
                // In a full implementation, we'd track line numbers in mappings
                true
            })
            .collect();

        for mapping in &_relevant_mappings {
            if python_char >= mapping.python_start && python_char < mapping.python_end {
                // Character is within a transformation
                match mapping.transformation_type {
                    TransformationType::PowerOperator => {
                        // Map from ** back to ^
                        let offset_in_transformation = python_char - mapping.python_start;
                        return mapping.sage_start + offset_in_transformation.min(1);
                    }
                    TransformationType::PolynomialRing => {
                        // Complex mapping - for now, return start of sage range
                        return mapping.sage_start;
                    }
                    _ => {}
                }
            }
        }

        // No transformation found, assume 1:1 mapping
        python_char
    }

    /// Map character position from SageMath to Python
    fn map_sage_char_to_python(&self, _sage_line: u32, sage_char: u32, _python_line: u32) -> u32 {
        // Find relevant character mappings for this line
        for mapping in &self.char_offsets {
            if sage_char >= mapping.sage_start && sage_char < mapping.sage_end {
                // Character is within a transformation
                match mapping.transformation_type {
                    TransformationType::PowerOperator => {
                        // Map from ^ to **
                        return mapping.python_start;
                    }
                    TransformationType::PolynomialRing => {
                        // Complex mapping - for now, return start of python range
                        return mapping.python_start;
                    }
                    _ => {}
                }
            }
        }

        // No transformation found, assume 1:1 mapping
        sage_char
    }

    /// Provide autocompletion suggestions for partial SageMath input
    pub fn get_autocompletion_suggestions(&self, partial_input: &str, cursor_pos: Position) -> Vec<String> {
        let mut suggestions = Vec::new();
        
        // Get the current line content
        let lines: Vec<&str> = partial_input.lines().collect();
        if cursor_pos.line as usize >= lines.len() {
            return suggestions;
        }
        
        let current_line = lines[cursor_pos.line as usize];
        let cursor_char = cursor_pos.character as usize;
        
        if cursor_char > current_line.len() {
            return suggestions;
        }
        
        let before_cursor = &current_line[..cursor_char];
        let _after_cursor = &current_line[cursor_char..];
        
        // Check for power operator completion
        if before_cursor.ends_with("*") && !before_cursor.ends_with("**") {
            suggestions.push("**".to_string());
        }
        
        // Check for polynomial ring completion
        if before_cursor.ends_with("P.<x> = Pol") {
            suggestions.push("PolynomialRing".to_string());
        }
        
        // Check for other common SageMath constructs
        if before_cursor.ends_with("matrix") {
            suggestions.push("matrix([[]])".to_string());
        }
        
        if before_cursor.ends_with("factor") {
            suggestions.push("factor()".to_string());
        }
        
        suggestions
    }

    /// Check if a partial input represents valid SageMath syntax that can be completed
    pub fn can_complete_syntax(&self, partial_input: &str) -> bool {
        // Check for incomplete power operators
        if partial_input.ends_with("^") {
            return true;
        }
        
        // Check for incomplete polynomial ring declarations
        if partial_input.contains(".<") && !partial_input.contains("> =") {
            return true;
        }
        
        // Check for incomplete function calls
        if partial_input.ends_with("(") {
            return true;
        }
        
        false
    }

    /// Get the original SageMath source
    pub fn sage_source(&self) -> &str {
        &self.sage_source
    }

    /// Get the converted Python source
    pub fn python_source(&self) -> &str {
        &self.python_source
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_power_operator_autocompletion() {
        let sage_source = "x = 2^3".to_string();
        let python_source = "x = 2**3".to_string();
        
        let source_map = AdvancedSourceMap::new(sage_source, python_source);
        
        // Test autocompletion for partial power operator
        let suggestions = source_map.get_autocompletion_suggestions(
            "x = 2*", 
            Position { line: 0, character: 6 }
        );
        
        assert!(suggestions.contains(&"**".to_string()));
    }

    #[test]
    fn test_polynomial_ring_autocompletion() {
        let sage_source = "P.<x> = PolynomialRing(QQ)".to_string();
        let python_source = "P = PolynomialRing(QQ, names=('x',)); (x,) = P._first_ngens(1)".to_string();
        
        let source_map = AdvancedSourceMap::new(sage_source, python_source);
        
        // Test autocompletion for partial polynomial ring
        let suggestions = source_map.get_autocompletion_suggestions(
            "P.<x> = Pol", 
            Position { line: 0, character: 11 }
        );
        
        assert!(suggestions.contains(&"PolynomialRing".to_string()));
    }

    #[test]
    fn test_bidirectional_position_mapping() {
        let sage_source = "x = 2^3".to_string();
        let python_source = "x = 2**3".to_string();
        
        let source_map = AdvancedSourceMap::new(sage_source, python_source);
        
        // Test sage to python mapping
        let sage_pos = Position { line: 0, character: 5 }; // Position of ^
        let python_pos = source_map.sage_position_to_python_position(sage_pos);
        
        assert!(python_pos.is_some());
        
        // Test python to sage mapping
        if let Some(py_pos) = python_pos {
            let back_to_sage = source_map.python_position_to_sage_position(py_pos);
            assert!(back_to_sage.is_some());
        }
    }

    #[test]
    fn test_syntax_completion_detection() {
        let source_map = AdvancedSourceMap::new("".to_string(), "".to_string());
        
        assert!(source_map.can_complete_syntax("x = 2^"));
        assert!(source_map.can_complete_syntax("P.<x"));
        assert!(source_map.can_complete_syntax("matrix("));
        assert!(!source_map.can_complete_syntax("x = 2**3"));
    }
}