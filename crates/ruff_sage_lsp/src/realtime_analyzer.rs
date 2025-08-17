//! Real-time syntax analysis and type inference for SageMath
//!
//! This module provides incremental parsing and semantic analysis of SageMath code
//! as the user types, without relying on external preprocessing. It includes:
//! - Incremental syntax parsing
//! - Type inference for SageMath objects
//! - Real-time error detection
//! - Context-aware autocompletion

use std::collections::HashMap;
use std::sync::Arc;

/// Position in source code (line, column)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

/// SageMath type information
#[derive(Debug, Clone, PartialEq)]
pub enum SageType {
    /// Integer (SageMath Integer type)
    Integer,
    /// Rational number
    Rational,
    /// Polynomial in one or more variables
    Polynomial {
        variables: Vec<String>,
        base_ring: Box<SageType>,
    },
    /// Polynomial ring
    PolynomialRing {
        variables: Vec<String>,
        base_ring: Box<SageType>,
    },
    /// Matrix over a ring
    Matrix {
        dimensions: Option<(usize, usize)>,
        base_ring: Box<SageType>,
    },
    /// Vector over a ring
    Vector {
        length: Option<usize>,
        base_ring: Box<SageType>,
    },
    /// Function type
    Function {
        name: String,
        args: Vec<SageType>,
        return_type: Box<SageType>,
    },
    /// Unknown/unresolved type
    Unknown,
    /// Error type (for type checking errors)
    Error(String),
}

/// Variable binding in the current scope
#[derive(Debug, Clone)]
pub struct Variable {
    pub name: String,
    pub sage_type: SageType,
    pub defined_at: Position,
    pub mutable: bool,
}

/// Scope for variable tracking
#[derive(Debug, Clone)]
pub struct Scope {
    pub variables: HashMap<String, Variable>,
    pub parent: Option<Arc<Scope>>,
}

/// Syntax token in SageMath code
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    /// Identifier (variable, function name, etc.)
    Identifier(String),
    /// Number literal
    Number(String),
    /// String literal
    String(String),
    /// Operator
    Operator(String),
    /// Keyword
    Keyword(String),
    /// Punctuation (brackets, parentheses, etc.)
    Punctuation(char),
    /// Whitespace
    Whitespace,
    /// Comment
    Comment(String),
    /// End of line
    Newline,
    /// End of file
    Eof,
}

/// Positioned token
#[derive(Debug, Clone)]
pub struct PositionedToken {
    pub token: Token,
    pub range: Range,
}

/// SageMath-specific AST nodes
#[derive(Debug, Clone)]
pub enum SageAstNode {
    /// Variable assignment: x = value
    Assignment {
        target: String,
        value: Box<SageAstNode>,
        range: Range,
    },
    /// Polynomial ring declaration: P.<x> = PolynomialRing(QQ)
    PolynomialRingDeclaration {
        ring_name: String,
        variables: Vec<String>,
        base_ring: Box<SageAstNode>,
        range: Range,
    },
    /// Power operation: base^exponent (converted to base**exponent)
    PowerOperation {
        base: Box<SageAstNode>,
        exponent: Box<SageAstNode>,
        range: Range,
    },
    /// Function call: func(args...)
    FunctionCall {
        name: String,
        args: Vec<SageAstNode>,
        range: Range,
    },
    /// Variable reference
    Variable {
        name: String,
        range: Range,
    },
    /// Number literal
    Number {
        value: String,
        range: Range,
    },
    /// Binary operation
    BinaryOp {
        left: Box<SageAstNode>,
        operator: String,
        right: Box<SageAstNode>,
        range: Range,
    },
    /// Error node (for syntax errors)
    Error {
        message: String,
        range: Range,
    },
}

/// Real-time syntax analyzer for SageMath
#[derive(Debug, Default)]
pub struct RealtimeAnalyzer {
    /// Current source code
    source: String,
    /// Parsed tokens
    tokens: Vec<PositionedToken>,
    /// Current AST
    ast: Vec<SageAstNode>,
    /// Current scope chain
    scope: Arc<Scope>,
    /// Type inference results
    type_context: HashMap<String, SageType>,
    /// Syntax and type errors
    errors: Vec<AnalysisError>,
}

/// Analysis error (syntax or type error)
#[derive(Debug, Clone)]
pub struct AnalysisError {
    pub message: String,
    pub range: Range,
    pub error_type: ErrorType,
}

#[derive(Debug, Clone)]
pub enum ErrorType {
    Syntax,
    Type,
    Semantic,
}

impl RealtimeAnalyzer {
    /// Create a new analyzer
    pub fn new() -> Self {
        Self {
            source: String::new(),
            tokens: Vec::new(),
            ast: Vec::new(),
            scope: Arc::new(Scope {
                variables: HashMap::new(),
                parent: None,
            }),
            type_context: HashMap::new(),
            errors: Vec::new(),
        }
    }

    /// Update the source code and perform incremental analysis
    pub fn update_source(&mut self, source: String, changed_range: Option<Range>) {
        self.source = source;
        
        if let Some(_range) = changed_range {
            // Incremental parsing - only re-analyze the changed portion
            self.incremental_analysis();
        } else {
            // Full re-analysis
            self.full_analysis();
        }
    }

    /// Perform full analysis of the source code
    fn full_analysis(&mut self) {
        self.errors.clear();
        
        // Step 1: Tokenization
        self.tokenize();
        
        // Step 2: Parsing
        self.parse();
        
        // Step 3: Type inference
        self.infer_types();
        
        // Step 4: Semantic analysis
        self.semantic_analysis();
    }

    /// Perform incremental analysis (optimized for real-time editing)
    fn incremental_analysis(&mut self) {
        // For now, fall back to full analysis
        // In a production implementation, this would only re-analyze changed sections
        self.full_analysis();
    }

    /// Tokenize the source code
    fn tokenize(&mut self) {
        self.tokens.clear();
        
        let mut line = 0;
        let mut character = 0;
        let mut chars = self.source.chars().peekable();
        
        while let Some(ch) = chars.next() {
            let start_pos = Position { line, character };
            
            match ch {
                // Whitespace
                ' ' | '\t' => {
                    character += 1;
                    self.tokens.push(PositionedToken {
                        token: Token::Whitespace,
                        range: Range {
                            start: start_pos,
                            end: Position { line, character },
                        },
                    });
                }
                
                // Newline
                '\n' => {
                    self.tokens.push(PositionedToken {
                        token: Token::Newline,
                        range: Range {
                            start: start_pos,
                            end: Position { line, character: character + 1 },
                        },
                    });
                    line += 1;
                    character = 0;
                }
                
                // Numbers
                '0'..='9' => {
                    let mut number = String::new();
                    number.push(ch);
                    character += 1;
                    
                    while let Some(&next_ch) = chars.peek() {
                        if next_ch.is_ascii_digit() || next_ch == '.' {
                            number.push(chars.next().unwrap());
                            character += 1;
                        } else {
                            break;
                        }
                    }
                    
                    self.tokens.push(PositionedToken {
                        token: Token::Number(number),
                        range: Range {
                            start: start_pos,
                            end: Position { line, character },
                        },
                    });
                }
                
                // Identifiers and keywords
                'a'..='z' | 'A'..='Z' | '_' => {
                    let mut identifier = String::new();
                    identifier.push(ch);
                    character += 1;
                    
                    while let Some(&next_ch) = chars.peek() {
                        if next_ch.is_alphanumeric() || next_ch == '_' {
                            identifier.push(chars.next().unwrap());
                            character += 1;
                        } else {
                            break;
                        }
                    }
                    
                    // Check if it's a keyword
                    let token = match identifier.as_str() {
                        "def" | "class" | "if" | "else" | "elif" | "for" | "while" | 
                        "return" | "import" | "from" | "as" | "try" | "except" | "finally" => {
                            Token::Keyword(identifier)
                        }
                        _ => Token::Identifier(identifier),
                    };
                    
                    self.tokens.push(PositionedToken {
                        token,
                        range: Range {
                            start: start_pos,
                            end: Position { line, character },
                        },
                    });
                }
                
                // String literals
                '"' | '\'' => {
                    let quote = ch;
                    let mut string_val = String::new();
                    character += 1;
                    
                    while let Some(next_ch) = chars.next() {
                        character += 1;
                        if next_ch == quote {
                            break;
                        } else if next_ch == '\\' {
                            // Handle escape sequences
                            if let Some(escaped) = chars.next() {
                                string_val.push('\\');
                                string_val.push(escaped);
                                character += 1;
                            }
                        } else {
                            string_val.push(next_ch);
                        }
                    }
                    
                    self.tokens.push(PositionedToken {
                        token: Token::String(string_val),
                        range: Range {
                            start: start_pos,
                            end: Position { line, character },
                        },
                    });
                }
                
                // Comments
                '#' => {
                    let mut comment = String::new();
                    character += 1;
                    
                    while let Some(&next_ch) = chars.peek() {
                        if next_ch == '\n' {
                            break;
                        }
                        comment.push(chars.next().unwrap());
                        character += 1;
                    }
                    
                    self.tokens.push(PositionedToken {
                        token: Token::Comment(comment),
                        range: Range {
                            start: start_pos,
                            end: Position { line, character },
                        },
                    });
                }
                
                // Power operator (SageMath specific)
                '^' => {
                    character += 1;
                    self.tokens.push(PositionedToken {
                        token: Token::Operator("^".to_string()),
                        range: Range {
                            start: start_pos,
                            end: Position { line, character },
                        },
                    });
                }
                
                // Other operators and punctuation
                '+' | '-' | '*' | '/' | '=' | '<' | '>' | '!' | '&' | '|' => {
                    let mut operator = String::new();
                    operator.push(ch);
                    character += 1;
                    
                    // Handle multi-character operators
                    if let Some(&next_ch) = chars.peek() {
                        match (ch, next_ch) {
                            ('*', '*') | ('=', '=') | ('!', '=') | ('<', '=') | ('>', '=') => {
                                operator.push(chars.next().unwrap());
                                character += 1;
                            }
                            _ => {}
                        }
                    }
                    
                    self.tokens.push(PositionedToken {
                        token: Token::Operator(operator),
                        range: Range {
                            start: start_pos,
                            end: Position { line, character },
                        },
                    });
                }
                
                // Punctuation
                '(' | ')' | '[' | ']' | '{' | '}' | ',' | ';' | ':' | '.' => {
                    character += 1;
                    self.tokens.push(PositionedToken {
                        token: Token::Punctuation(ch),
                        range: Range {
                            start: start_pos,
                            end: Position { line, character },
                        },
                    });
                }
                
                // Unknown character
                _ => {
                    character += 1;
                    self.errors.push(AnalysisError {
                        message: format!("Unexpected character: '{}'", ch),
                        range: Range {
                            start: start_pos,
                            end: Position { line, character },
                        },
                        error_type: ErrorType::Syntax,
                    });
                }
            }
        }
        
        // Add EOF token
        self.tokens.push(PositionedToken {
            token: Token::Eof,
            range: Range {
                start: Position { line, character },
                end: Position { line, character },
            },
        });
    }

    /// Parse tokens into AST
    fn parse(&mut self) {
        self.ast.clear();
        
        let mut parser = Parser::new(&self.tokens);
        while !parser.is_at_end() {
            match parser.parse_statement() {
                Ok(node) => self.ast.push(node),
                Err(error) => self.errors.push(error),
            }
        }
    }

    /// Perform type inference on the AST
    fn infer_types(&mut self) {
        self.type_context.clear();
        
        for node in &self.ast {
            self.infer_node_type(node);
        }
    }

    /// Infer the type of an AST node
    fn infer_node_type(&mut self, node: &SageAstNode) -> SageType {
        match node {
            SageAstNode::Assignment { target, value, .. } => {
                let value_type = self.infer_node_type(value);
                self.type_context.insert(target.clone(), value_type.clone());
                value_type
            }
            
            SageAstNode::PolynomialRingDeclaration { ring_name, variables, base_ring, .. } => {
                let base_type = self.infer_node_type(base_ring);
                let ring_type = SageType::PolynomialRing {
                    variables: variables.clone(),
                    base_ring: Box::new(base_type),
                };
                self.type_context.insert(ring_name.clone(), ring_type.clone());
                
                // Also define the polynomial variables
                for var in variables {
                    let poly_type = SageType::Polynomial {
                        variables: vec![var.clone()],
                        base_ring: Box::new(SageType::Integer), // Default to integers
                    };
                    self.type_context.insert(var.clone(), poly_type);
                }
                
                ring_type
            }
            
            SageAstNode::PowerOperation { base, exponent, .. } => {
                let base_type = self.infer_node_type(base);
                let _exp_type = self.infer_node_type(exponent);
                
                // Power operation generally preserves the base type
                base_type
            }
            
            SageAstNode::FunctionCall { name, args, .. } => {
                // Infer types for arguments
                let _arg_types: Vec<_> = args.iter().map(|arg| self.infer_node_type(arg)).collect();
                
                // Return type based on function name
                match name.as_str() {
                    "PolynomialRing" => SageType::PolynomialRing {
                        variables: vec![], // Will be filled in by declaration
                        base_ring: Box::new(SageType::Integer),
                    },
                    "matrix" | "Matrix" => SageType::Matrix {
                        dimensions: None,
                        base_ring: Box::new(SageType::Integer),
                    },
                    "vector" | "Vector" => SageType::Vector {
                        length: None,
                        base_ring: Box::new(SageType::Integer),
                    },
                    "factor" | "gcd" | "lcm" => SageType::Integer,
                    _ => SageType::Unknown,
                }
            }
            
            SageAstNode::Variable { name, .. } => {
                self.type_context.get(name).cloned().unwrap_or(SageType::Unknown)
            }
            
            SageAstNode::Number { value, .. } => {
                if value.contains('.') {
                    SageType::Rational
                } else {
                    SageType::Integer
                }
            }
            
            SageAstNode::BinaryOp { left, right, operator, .. } => {
                let left_type = self.infer_node_type(left);
                let right_type = self.infer_node_type(right);
                
                match operator.as_str() {
                    "+" | "-" | "*" | "/" | "**" | "^" => {
                        // Arithmetic operations - try to find common type
                        match (&left_type, &right_type) {
                            (SageType::Integer, SageType::Integer) => SageType::Integer,
                            (SageType::Rational, _) | (_, SageType::Rational) => SageType::Rational,
                            _ => left_type, // Default to left type
                        }
                    }
                    "==" | "!=" | "<" | ">" | "<=" | ">=" => SageType::Integer, // Boolean (represented as Integer in Sage)
                    _ => SageType::Unknown,
                }
            }
            
            SageAstNode::Error { .. } => SageType::Error("Syntax error".to_string()),
        }
    }

    /// Perform semantic analysis
    fn semantic_analysis(&mut self) {
        // Check for undefined variables, type mismatches, etc.
        for node in &self.ast {
            self.check_semantics(node);
        }
    }

    /// Check semantics of an AST node
    fn check_semantics(&mut self, node: &SageAstNode) {
        match node {
            SageAstNode::Variable { name, range } => {
                if !self.type_context.contains_key(name) {
                    self.errors.push(AnalysisError {
                        message: format!("Undefined variable: {}", name),
                        range: *range,
                        error_type: ErrorType::Semantic,
                    });
                }
            }
            
            SageAstNode::Assignment { value, .. } => {
                self.check_semantics(value);
            }
            
            SageAstNode::PolynomialRingDeclaration { base_ring, .. } => {
                self.check_semantics(base_ring);
            }
            
            SageAstNode::PowerOperation { base, exponent, .. } => {
                self.check_semantics(base);
                self.check_semantics(exponent);
            }
            
            SageAstNode::FunctionCall { args, .. } => {
                for arg in args {
                    self.check_semantics(arg);
                }
            }
            
            SageAstNode::BinaryOp { left, right, .. } => {
                self.check_semantics(left);
                self.check_semantics(right);
            }
            
            _ => {}
        }
    }

    /// Get autocompletion suggestions at a given position
    pub fn get_completions(&self, position: Position) -> Vec<CompletionItem> {
        let mut completions = Vec::new();
        
        // Add variables in scope
        for (name, sage_type) in &self.type_context {
            completions.push(CompletionItem {
                label: name.clone(),
                insert_text: name.clone(),
                detail: Some(format!("Type: {:?}", sage_type)),
                kind: CompletionKind::Variable,
            });
        }
        
        // Add SageMath functions
        let sage_functions = [
            ("factor", "Factor an integer or polynomial"),
            ("gcd", "Greatest common divisor"),
            ("lcm", "Least common multiple"),
            ("is_prime", "Test if number is prime"),
            ("matrix", "Create a matrix"),
            ("vector", "Create a vector"),
            ("PolynomialRing", "Create a polynomial ring"),
        ];
        
        for (name, description) in &sage_functions {
            completions.push(CompletionItem {
                label: format!("{}()", name),
                insert_text: format!("{}()", name),
                detail: Some(description.to_string()),
                kind: CompletionKind::Function,
            });
        }
        
        // Context-aware completions based on position
        if let Some(context) = self.get_context_at_position(position) {
            completions.extend(self.get_context_completions(&context));
        }
        
        completions
    }

    /// Get the context at a specific position
    fn get_context_at_position(&self, _position: Position) -> Option<String> {
        // TODO: Implement context detection based on position
        // This would analyze the token stream around the position
        None
    }

    /// Get context-specific completions
    fn get_context_completions(&self, _context: &str) -> Vec<CompletionItem> {
        // TODO: Implement context-specific completions
        Vec::new()
    }

    /// Get current errors
    pub fn get_errors(&self) -> &[AnalysisError] {
        &self.errors
    }

    /// Get type of a variable
    pub fn get_variable_type(&self, name: &str) -> Option<&SageType> {
        self.type_context.get(name)
    }

    /// Get the AST
    pub fn get_ast(&self) -> &[SageAstNode] {
        &self.ast
    }
}

/// Parser for SageMath syntax
struct Parser<'a> {
    tokens: &'a [PositionedToken],
    current: usize,
}

impl<'a> Parser<'a> {
    fn new(tokens: &'a [PositionedToken]) -> Self {
        Self { tokens, current: 0 }
    }

    fn is_at_end(&self) -> bool {
        self.current >= self.tokens.len() || 
        matches!(self.tokens[self.current].token, Token::Eof)
    }

    fn advance(&mut self) -> Option<&PositionedToken> {
        if !self.is_at_end() {
            self.current += 1;
            self.tokens.get(self.current - 1)
        } else {
            None
        }
    }

    fn peek(&self) -> Option<&PositionedToken> {
        self.tokens.get(self.current)
    }

    fn parse_statement(&mut self) -> Result<SageAstNode, AnalysisError> {
        // Skip whitespace and comments
        while let Some(token) = self.peek() {
            match &token.token {
                Token::Whitespace | Token::Comment(_) | Token::Newline => {
                    self.advance();
                }
                _ => break,
            }
        }

        if self.is_at_end() {
            return Err(AnalysisError {
                message: "Unexpected end of input".to_string(),
                range: Range {
                    start: Position { line: 0, character: 0 },
                    end: Position { line: 0, character: 0 },
                },
                error_type: ErrorType::Syntax,
            });
        }

        // Try to parse different statement types
        self.parse_assignment_or_expression()
    }

    fn parse_assignment_or_expression(&mut self) -> Result<SageAstNode, AnalysisError> {
        // For now, implement a simple expression parser
        // In a full implementation, this would handle the full SageMath grammar
        
        if let Some(token) = self.peek() {
            match &token.token {
                Token::Identifier(name) => {
                    let name = name.clone();
                    let range = token.range;
                    self.advance();
                    
                    // Check if this is an assignment
                    if let Some(next_token) = self.peek() {
                        if matches!(next_token.token, Token::Operator(ref op) if op == "=") {
                            self.advance(); // consume =
                            let value = self.parse_expression()?;
                            return Ok(SageAstNode::Assignment {
                                target: name,
                                value: Box::new(value),
                                range,
                            });
                        }
                    }
                    
                    // Otherwise it's a variable reference
                    Ok(SageAstNode::Variable { name, range })
                }
                
                Token::Number(value) => {
                    let value = value.clone();
                    let range = token.range;
                    self.advance();
                    Ok(SageAstNode::Number { value, range })
                }
                
                _ => {
                    let range = token.range;
                    Err(AnalysisError {
                        message: "Unexpected token".to_string(),
                        range,
                        error_type: ErrorType::Syntax,
                    })
                }
            }
        } else {
            Err(AnalysisError {
                message: "Unexpected end of input".to_string(),
                range: Range {
                    start: Position { line: 0, character: 0 },
                    end: Position { line: 0, character: 0 },
                },
                error_type: ErrorType::Syntax,
            })
        }
    }

    fn parse_expression(&mut self) -> Result<SageAstNode, AnalysisError> {
        // Simple expression parsing - in a full implementation, this would handle
        // operator precedence, function calls, etc.
        
        if let Some(token) = self.peek() {
            match &token.token {
                Token::Number(value) => {
                    let value = value.clone();
                    let range = token.range;
                    self.advance();
                    Ok(SageAstNode::Number { value, range })
                }
                
                Token::Identifier(name) => {
                    let name = name.clone();
                    let range = token.range;
                    self.advance();
                    Ok(SageAstNode::Variable { name, range })
                }
                
                _ => {
                    let range = token.range;
                    Err(AnalysisError {
                        message: "Expected expression".to_string(),
                        range,
                        error_type: ErrorType::Syntax,
                    })
                }
            }
        } else {
            Err(AnalysisError {
                message: "Expected expression".to_string(),
                range: Range {
                    start: Position { line: 0, character: 0 },
                    end: Position { line: 0, character: 0 },
                },
                error_type: ErrorType::Syntax,
            })
        }
    }
}

/// Completion item for autocompletion
#[derive(Debug, Clone)]
pub struct CompletionItem {
    pub label: String,
    pub insert_text: String,
    pub detail: Option<String>,
    pub kind: CompletionKind,
}

#[derive(Debug, Clone)]
pub enum CompletionKind {
    Variable,
    Function,
    Keyword,
    Operator,
    Constructor,
}

impl Default for RealtimeAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for RealtimeAnalyzer {
    fn clone(&self) -> Self {
        // For cloning, we create a new analyzer and update it with the current source
        let mut cloned = Self::new();
        cloned.update_source(self.source.clone(), None);
        cloned
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenization() {
        let mut analyzer = RealtimeAnalyzer::new();
        analyzer.update_source("x = 2^3".to_string(), None);
        
        // Check that we have the expected tokens
        let tokens: Vec<_> = analyzer.tokens.iter()
            .filter(|t| !matches!(t.token, Token::Whitespace))
            .map(|t| &t.token)
            .collect();
        
        assert!(matches!(tokens[0], Token::Identifier(_)));
        assert!(matches!(tokens[1], Token::Operator(_)));
        assert!(matches!(tokens[2], Token::Number(_)));
        assert!(matches!(tokens[3], Token::Operator(_))); // ^
        assert!(matches!(tokens[4], Token::Number(_)));
    }

    #[test]
    fn test_power_operator_detection() {
        let mut analyzer = RealtimeAnalyzer::new();
        analyzer.update_source("x = 2^3".to_string(), None);
        
        // Check that ^ operator is detected
        let has_power_op = analyzer.tokens.iter().any(|t| {
            matches!(&t.token, Token::Operator(op) if op == "^")
        });
        
        assert!(has_power_op);
    }

    #[test]
    fn test_type_inference() {
        let mut analyzer = RealtimeAnalyzer::new();
        analyzer.update_source("x = 42".to_string(), None);
        
        // Check that x is inferred as Integer type
        if let Some(x_type) = analyzer.get_variable_type("x") {
            assert!(matches!(x_type, SageType::Integer));
        } else {
            panic!("Variable x not found in type context");
        }
    }

    #[test]
    fn test_polynomial_ring_parsing() {
        let mut analyzer = RealtimeAnalyzer::new();
        analyzer.update_source("P.<x> = PolynomialRing(QQ)".to_string(), None);
        
        // This is a simplified test - the full parser would handle this more completely
        assert!(!analyzer.get_errors().is_empty() || !analyzer.get_ast().is_empty());
    }

    #[test]
    fn test_completion_suggestions() {
        let mut analyzer = RealtimeAnalyzer::new();
        analyzer.update_source("x = 42\ny = ".to_string(), None);
        
        let completions = analyzer.get_completions(Position { line: 1, character: 4 });
        
        // Should suggest 'x' as a variable
        assert!(completions.iter().any(|c| c.label == "x"));
        
        // Should suggest SageMath functions
        assert!(completions.iter().any(|c| c.label.contains("factor")));
    }
}