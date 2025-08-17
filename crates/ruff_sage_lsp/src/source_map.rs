//! Source mapping between SageMath and Python coordinates
//!
//! This module provides functionality to map locations between original SageMath (.sage)
//! source files and their converted Python equivalents. This is crucial for translating
//! diagnostics, hover information, and other LSP features back to the original source.

use ruff_text_size::{TextRange, TextSize};
use std::collections::HashMap;

/// Represents a mapping between SageMath and Python source coordinates
#[derive(Debug, Clone)]
pub struct SourceMap {
    /// Mapping from Python line numbers to SageMath line numbers
    python_to_sage_lines: HashMap<u32, u32>,
    /// Mapping from SageMath line numbers to Python line numbers
    sage_to_python_lines: HashMap<u32, u32>,
    /// The original SageMath source
    sage_source: String,
    /// The converted Python source
    python_source: String,
}

/// Represents a position in source code (line, column)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub line: u32,
    pub character: u32,
}

/// Represents a range in source code
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

impl SourceMap {
    /// Create a new source map from SageMath and Python sources
    pub fn new(sage_source: String, python_source: String) -> Self {
        let mut source_map = Self {
            python_to_sage_lines: HashMap::new(),
            sage_to_python_lines: HashMap::new(),
            sage_source,
            python_source,
        };

        source_map.build_line_mappings();
        source_map
    }

    /// Build line mappings between SageMath and Python sources
    fn build_line_mappings(&mut self) {
        // For now, we implement a simple 1:1 line mapping
        // In a more sophisticated implementation, we would parse the 
        // sage --preparse output to understand exactly how lines are transformed
        
        let sage_lines = self.sage_source.lines().count() as u32;
        let python_lines = self.python_source.lines().count() as u32;

        // Simple mapping: assume lines correspond 1:1 for basic cases
        // This is a starting point - real implementation would need to parse
        // the actual transformations made by sage --preparse
        for line in 0..sage_lines.min(python_lines) {
            self.sage_to_python_lines.insert(line, line);
            self.python_to_sage_lines.insert(line, line);
        }

        // Handle case where Python has more lines (imports, etc.)
        if python_lines > sage_lines {
            // Additional Python lines (likely imports) map to line 0 of sage
            for line in sage_lines..python_lines {
                self.python_to_sage_lines.insert(line, 0);
            }
        }
    }

    /// Convert a Python line number to a SageMath line number
    pub fn python_line_to_sage_line(&self, python_line: u32) -> Option<u32> {
        self.python_to_sage_lines.get(&python_line).copied()
    }

    /// Convert a SageMath line number to a Python line number
    pub fn sage_line_to_python_line(&self, sage_line: u32) -> Option<u32> {
        self.sage_to_python_lines.get(&sage_line).copied()
    }

    /// Convert a Python position to a SageMath position
    pub fn python_position_to_sage_position(&self, python_pos: Position) -> Option<Position> {
        let sage_line = self.python_line_to_sage_line(python_pos.line)?;
        
        // For character positions, we need to account for syntax transformations
        // This is a simplified implementation - real mapping would track character offsets
        let sage_character = self.map_python_character_to_sage(
            python_pos.line, 
            python_pos.character, 
            sage_line
        );

        Some(Position {
            line: sage_line,
            character: sage_character,
        })
    }

    /// Convert a SageMath position to a Python position
    pub fn sage_position_to_python_position(&self, sage_pos: Position) -> Option<Position> {
        let python_line = self.sage_line_to_python_line(sage_pos.line)?;
        
        let python_character = self.map_sage_character_to_python(
            sage_pos.line, 
            sage_pos.character, 
            python_line
        );

        Some(Position {
            line: python_line,
            character: python_character,
        })
    }

    /// Convert a Python range to a SageMath range
    pub fn python_range_to_sage_range(&self, python_range: Range) -> Option<Range> {
        let start = self.python_position_to_sage_position(python_range.start)?;
        let end = self.python_position_to_sage_position(python_range.end)?;

        Some(Range { start, end })
    }

    /// Convert a SageMath range to a Python range
    pub fn sage_range_to_python_range(&self, sage_range: Range) -> Option<Range> {
        let start = self.sage_position_to_python_position(sage_range.start)?;
        let end = self.sage_position_to_python_position(sage_range.end)?;

        Some(Range { start, end })
    }

    /// Convert a Python TextRange to a SageMath TextRange
    pub fn python_text_range_to_sage_text_range(&self, python_range: TextRange) -> Option<TextRange> {
        // Convert TextRange to line/character positions
        let python_start_pos = self.text_size_to_position(&self.python_source, python_range.start());
        let python_end_pos = self.text_size_to_position(&self.python_source, python_range.end());

        let python_range_pos = Range {
            start: python_start_pos,
            end: python_end_pos,
        };

        let sage_range_pos = self.python_range_to_sage_range(python_range_pos)?;

        // Convert back to TextRange
        let sage_start = self.position_to_text_size(&self.sage_source, sage_range_pos.start);
        let sage_end = self.position_to_text_size(&self.sage_source, sage_range_pos.end);

        Some(TextRange::new(sage_start, sage_end))
    }

    /// Map a character position from Python to SageMath, accounting for syntax changes
    fn map_python_character_to_sage(&self, python_line: u32, python_char: u32, sage_line: u32) -> u32 {
        // This is a simplified implementation
        // Real implementation would track specific syntax transformations like ^ -> **
        
        // Get the actual line content to check for transformations
        let python_line_content = self.get_line(&self.python_source, python_line);
        let sage_line_content = self.get_line(&self.sage_source, sage_line);

        // Handle the ** -> ^ transformation
        if let (Some(py_line), Some(sage_line)) = (python_line_content, sage_line_content) {
            if py_line.contains("**") && sage_line.contains("^") {
                // Adjust character position for ** -> ^ transformation
                let char_pos = python_char as usize;
                if char_pos > 0 && char_pos < py_line.len() {
                    let before_char = &py_line[..char_pos];
                    let stars_count = before_char.matches("**").count();
                    // Each ** in Python corresponds to one ^ in Sage, so subtract the difference
                    return python_char.saturating_sub(stars_count as u32);
                }
            }
        }

        python_char
    }

    /// Map a character position from SageMath to Python, accounting for syntax changes
    fn map_sage_character_to_python(&self, sage_line: u32, sage_char: u32, python_line: u32) -> u32 {
        // This is a simplified implementation
        // Real implementation would track specific syntax transformations like ^ -> **
        
        let sage_line_content = self.get_line(&self.sage_source, sage_line);
        let python_line_content = self.get_line(&self.python_source, python_line);

        // Handle the ^ -> ** transformation
        if let (Some(sage_line), Some(py_line)) = (sage_line_content, python_line_content) {
            if sage_line.contains("^") && py_line.contains("**") {
                // Adjust character position for ^ -> ** transformation
                let char_pos = sage_char as usize;
                if char_pos > 0 && char_pos < sage_line.len() {
                    let before_char = &sage_line[..char_pos];
                    let caret_count = before_char.matches('^').count();
                    // Each ^ in Sage corresponds to ** in Python, so add the difference
                    return sage_char + caret_count as u32;
                }
            }
        }

        sage_char
    }

    /// Get a specific line from source code
    fn get_line<'a>(&self, source: &'a str, line: u32) -> Option<&'a str> {
        source.lines().nth(line as usize)
    }

    /// Convert a TextSize to a Position in the given source
    fn text_size_to_position(&self, source: &str, offset: TextSize) -> Position {
        let offset = offset.to_usize();
        let mut line = 0;
        let mut character = 0;
        let mut _current_offset = 0;

        for (idx, ch) in source.char_indices() {
            if idx >= offset {
                break;
            }
            if ch == '\n' {
                line += 1;
                character = 0;
            } else {
                character += 1;
            }
            _current_offset = idx + ch.len_utf8();
        }

        Position { line, character }
    }

    /// Convert a Position to a TextSize in the given source
    fn position_to_text_size(&self, source: &str, position: Position) -> TextSize {
        let mut current_line = 0;
        let mut current_character = 0;

        for (idx, ch) in source.char_indices() {
            if current_line == position.line && current_character == position.character {
                return TextSize::try_from(idx).unwrap_or(TextSize::default());
            }

            if ch == '\n' {
                current_line += 1;
                current_character = 0;
            } else {
                current_character += 1;
            }
        }

        TextSize::try_from(source.len()).unwrap_or(TextSize::default())
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
    fn test_basic_source_mapping() {
        let sage_source = "x = 2^3\ny = 1/2\nprint(x * y)".to_string();
        let python_source = "x = 2**3\ny = 1/2\nprint(x * y)".to_string();

        let source_map = SourceMap::new(sage_source, python_source);

        // Test line mappings
        assert_eq!(source_map.sage_line_to_python_line(0), Some(0));
        assert_eq!(source_map.sage_line_to_python_line(1), Some(1));
        assert_eq!(source_map.python_line_to_sage_line(0), Some(0));

        // Test position mappings
        let sage_pos = Position { line: 0, character: 5 }; // Position of '^'
        let python_pos = source_map.sage_position_to_python_position(sage_pos);
        
        // The character position should be adjusted for ^ -> ** transformation
        if let Some(pos) = python_pos {
            println!("Mapped position: line {}, char {}", pos.line, pos.character);
        }
    }

    #[test]
    fn test_position_conversion() {
        let source = "line 1\nline 2\nline 3";
        let source_map = SourceMap::new(source.to_string(), source.to_string());

        let pos = Position { line: 1, character: 2 };
        let text_size = source_map.position_to_text_size(source, pos);
        let back_to_pos = source_map.text_size_to_position(source, text_size);

        assert_eq!(pos, back_to_pos);
    }
}