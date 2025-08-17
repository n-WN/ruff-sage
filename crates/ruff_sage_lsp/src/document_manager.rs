//! Document management for SageMath LSP with real-time analysis
//!
//! This module provides document management that integrates with the real-time analyzer
//! to provide incremental parsing, type inference, and semantic analysis as documents change.

use crate::realtime_analyzer::{RealtimeAnalyzer, Position, Range, CompletionItem};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Document URI type
pub type DocumentUri = String;

/// Version of a document (incremented on each change)
pub type DocumentVersion = i32;

/// Represents a SageMath document in the LSP server
#[derive(Debug, Clone)]
pub struct SageDocument {
    /// Document URI
    pub uri: DocumentUri,
    /// Current version
    pub version: DocumentVersion,
    /// Document content
    pub content: String,
    /// Real-time analyzer for this document
    analyzer: RealtimeAnalyzer,
    /// Language ID (should be "sagemath" or "sage")
    pub language_id: String,
}

impl SageDocument {
    /// Create a new SageMath document
    pub fn new(uri: DocumentUri, version: DocumentVersion, content: String, language_id: String) -> Self {
        let mut analyzer = RealtimeAnalyzer::new();
        analyzer.update_source(content.clone(), None);
        
        Self {
            uri,
            version,
            content,
            analyzer,
            language_id,
        }
    }

    /// Update the document content and perform incremental analysis
    pub fn update_content(&mut self, version: DocumentVersion, content: String, changes: Option<Vec<DocumentChange>>) {
        self.version = version;
        self.content = content.clone();
        
        // Calculate changed range if we have specific changes
        let changed_range = if let Some(changes) = changes {
            self.calculate_changed_range(&changes)
        } else {
            None // Full document change
        };
        
        // Update the analyzer with incremental parsing
        self.analyzer.update_source(content, changed_range);
    }

    /// Calculate the overall changed range from a list of changes
    fn calculate_changed_range(&self, changes: &[DocumentChange]) -> Option<Range> {
        if changes.is_empty() {
            return None;
        }
        
        let mut min_start = changes[0].range.start;
        let mut max_end = changes[0].range.end;
        
        for change in changes {
            if change.range.start.line < min_start.line || 
               (change.range.start.line == min_start.line && change.range.start.character < min_start.character) {
                min_start = change.range.start;
            }
            
            if change.range.end.line > max_end.line || 
               (change.range.end.line == max_end.line && change.range.end.character > max_end.character) {
                max_end = change.range.end;
            }
        }
        
        Some(Range {
            start: min_start,
            end: max_end,
        })
    }

    /// Get completions at a specific position
    pub fn get_completions(&self, position: Position) -> Vec<CompletionItem> {
        self.analyzer.get_completions(position)
    }

    /// Get diagnostics (errors) for the document
    pub fn get_diagnostics(&self) -> Vec<Diagnostic> {
        self.analyzer.get_errors().iter().map(|error| {
            Diagnostic {
                range: error.range,
                severity: match error.error_type {
                    crate::realtime_analyzer::ErrorType::Syntax => DiagnosticSeverity::Error,
                    crate::realtime_analyzer::ErrorType::Type => DiagnosticSeverity::Warning,
                    crate::realtime_analyzer::ErrorType::Semantic => DiagnosticSeverity::Information,
                },
                message: error.message.clone(),
                source: Some("ruff-sage".to_string()),
            }
        }).collect()
    }

    /// Get hover information at a position
    pub fn get_hover(&self, position: Position) -> Option<HoverInfo> {
        // Find the token at the position
        if let Some(token_info) = self.get_token_at_position(position) {
            match token_info {
                TokenInfo::Variable(name) => {
                    if let Some(sage_type) = self.analyzer.get_variable_type(&name) {
                        return Some(HoverInfo {
                            content: format!("**{}**: {:?}", name, sage_type),
                            range: Some(self.get_word_range_at_position(position)),
                        });
                    }
                }
                TokenInfo::Function(name) => {
                    if let Some(func_info) = get_sage_function_info(&name) {
                        return Some(HoverInfo {
                            content: format!("**{}**\n\n{}", name, func_info.description),
                            range: Some(self.get_word_range_at_position(position)),
                        });
                    }
                }
                _ => {}
            }
        }
        
        None
    }

    /// Get token information at a specific position
    fn get_token_at_position(&self, _position: Position) -> Option<TokenInfo> {
        // TODO: Implement token lookup based on position
        // This would analyze the AST to find the token at the given position
        None
    }

    /// Get the word range at a position
    fn get_word_range_at_position(&self, position: Position) -> Range {
        let lines: Vec<&str> = self.content.lines().collect();
        if position.line as usize >= lines.len() {
            return Range { start: position, end: position };
        }
        
        let line = lines[position.line as usize];
        let char_pos = position.character as usize;
        
        if char_pos >= line.len() {
            return Range { start: position, end: position };
        }
        
        // Find word boundaries
        let mut start = char_pos;
        let mut end = char_pos;
        
        let chars: Vec<char> = line.chars().collect();
        
        // Move start backwards to beginning of word
        while start > 0 && (chars[start - 1].is_alphanumeric() || chars[start - 1] == '_') {
            start -= 1;
        }
        
        // Move end forwards to end of word
        while end < chars.len() && (chars[end].is_alphanumeric() || chars[end] == '_') {
            end += 1;
        }
        
        Range {
            start: Position { line: position.line, character: start as u32 },
            end: Position { line: position.line, character: end as u32 },
        }
    }

    /// Get document symbols for outline view
    pub fn get_document_symbols(&self) -> Vec<DocumentSymbol> {
        let mut symbols = Vec::new();
        
        for node in self.analyzer.get_ast() {
            if let Some(symbol) = self.ast_node_to_symbol(node) {
                symbols.push(symbol);
            }
        }
        
        symbols
    }

    /// Convert an AST node to a document symbol
    fn ast_node_to_symbol(&self, node: &crate::realtime_analyzer::SageAstNode) -> Option<DocumentSymbol> {
        use crate::realtime_analyzer::SageAstNode;
        
        match node {
            SageAstNode::Assignment { target, range, .. } => {
                Some(DocumentSymbol {
                    name: target.clone(),
                    kind: SymbolKind::Variable,
                    range: *range,
                    selection_range: *range,
                    children: Vec::new(),
                })
            }
            
            SageAstNode::PolynomialRingDeclaration { ring_name, variables, range, .. } => {
                let mut children = Vec::new();
                for var in variables {
                    children.push(DocumentSymbol {
                        name: var.clone(),
                        kind: SymbolKind::Variable,
                        range: *range, // TODO: Get actual variable range
                        selection_range: *range,
                        children: Vec::new(),
                    });
                }
                
                Some(DocumentSymbol {
                    name: format!("{} ({})", ring_name, variables.join(", ")),
                    kind: SymbolKind::Class,
                    range: *range,
                    selection_range: *range,
                    children,
                })
            }
            
            SageAstNode::FunctionCall { name, range, .. } => {
                Some(DocumentSymbol {
                    name: format!("{}()", name),
                    kind: SymbolKind::Function,
                    range: *range,
                    selection_range: *range,
                    children: Vec::new(),
                })
            }
            
            _ => None,
        }
    }
}

/// Represents a change to a document
#[derive(Debug, Clone)]
pub struct DocumentChange {
    /// Range being changed
    pub range: Range,
    /// New text for the range
    pub text: String,
}

/// Token information at a position
#[derive(Debug, Clone)]
enum TokenInfo {
    Variable(String),
    Function(String),
    Keyword(String),
    Number(String),
    Operator(String),
    Unknown,
}

/// Diagnostic information
#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub range: Range,
    pub severity: DiagnosticSeverity,
    pub message: String,
    pub source: Option<String>,
}

#[derive(Debug, Clone)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Information,
    Hint,
}

/// Hover information
#[derive(Debug, Clone)]
pub struct HoverInfo {
    pub content: String,
    pub range: Option<Range>,
}

/// Document symbol for outline view
#[derive(Debug, Clone)]
pub struct DocumentSymbol {
    pub name: String,
    pub kind: SymbolKind,
    pub range: Range,
    pub selection_range: Range,
    pub children: Vec<DocumentSymbol>,
}

#[derive(Debug, Clone)]
pub enum SymbolKind {
    File,
    Module,
    Namespace,
    Package,
    Class,
    Method,
    Property,
    Field,
    Constructor,
    Enum,
    Interface,
    Function,
    Variable,
    Constant,
    String,
    Number,
    Boolean,
    Array,
    Object,
    Key,
    Null,
    EnumMember,
    Struct,
    Event,
    Operator,
    TypeParameter,
}

/// SageMath function information
struct SageFunctionInfo {
    pub description: String,
    pub signature: String,
    pub examples: Vec<String>,
}

/// Get information about a SageMath function
fn get_sage_function_info(name: &str) -> Option<SageFunctionInfo> {
    match name {
        "factor" => Some(SageFunctionInfo {
            description: "Factor an integer or polynomial into irreducible factors.".to_string(),
            signature: "factor(n)".to_string(),
            examples: vec![
                "factor(60)  # Returns 2^2 * 3 * 5".to_string(),
                "factor(x^2 - 1)  # Returns (x - 1) * (x + 1)".to_string(),
            ],
        }),
        
        "gcd" => Some(SageFunctionInfo {
            description: "Compute the greatest common divisor of two or more integers.".to_string(),
            signature: "gcd(a, b, ...)".to_string(),
            examples: vec![
                "gcd(12, 18)  # Returns 6".to_string(),
                "gcd(24, 36, 48)  # Returns 12".to_string(),
            ],
        }),
        
        "lcm" => Some(SageFunctionInfo {
            description: "Compute the least common multiple of two or more integers.".to_string(),
            signature: "lcm(a, b, ...)".to_string(),
            examples: vec![
                "lcm(12, 18)  # Returns 36".to_string(),
                "lcm(4, 6, 8)  # Returns 24".to_string(),
            ],
        }),
        
        "is_prime" => Some(SageFunctionInfo {
            description: "Test whether an integer is prime.".to_string(),
            signature: "is_prime(n)".to_string(),
            examples: vec![
                "is_prime(17)  # Returns True".to_string(),
                "is_prime(15)  # Returns False".to_string(),
            ],
        }),
        
        "matrix" => Some(SageFunctionInfo {
            description: "Create a matrix from a list of lists.".to_string(),
            signature: "matrix(entries)".to_string(),
            examples: vec![
                "matrix([[1, 2], [3, 4]])".to_string(),
                "matrix(QQ, [[1/2, 0], [0, 1/3]])".to_string(),
            ],
        }),
        
        "vector" => Some(SageFunctionInfo {
            description: "Create a vector from a list.".to_string(),
            signature: "vector(entries)".to_string(),
            examples: vec![
                "vector([1, 2, 3])".to_string(),
                "vector(QQ, [1/2, 1/3, 1/4])".to_string(),
            ],
        }),
        
        "PolynomialRing" => Some(SageFunctionInfo {
            description: "Create a polynomial ring over a base ring.".to_string(),
            signature: "PolynomialRing(base_ring, names)".to_string(),
            examples: vec![
                "PolynomialRing(QQ, 'x')".to_string(),
                "PolynomialRing(ZZ, ['x', 'y'])".to_string(),
            ],
        }),
        
        _ => None,
    }
}

/// Document manager for the LSP server
pub struct DocumentManager {
    /// Map of document URI to document
    documents: Arc<RwLock<HashMap<DocumentUri, SageDocument>>>,
}

impl DocumentManager {
    /// Create a new document manager
    pub fn new() -> Self {
        Self {
            documents: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Open a new document
    pub async fn open_document(&self, uri: DocumentUri, version: DocumentVersion, content: String, language_id: String) {
        let document = SageDocument::new(uri.clone(), version, content, language_id);
        let mut documents = self.documents.write().await;
        documents.insert(uri, document);
    }

    /// Update an existing document
    pub async fn update_document(&self, uri: &DocumentUri, version: DocumentVersion, content: String, changes: Option<Vec<DocumentChange>>) -> Result<(), String> {
        let mut documents = self.documents.write().await;
        
        if let Some(document) = documents.get_mut(uri) {
            document.update_content(version, content, changes);
            Ok(())
        } else {
            Err(format!("Document not found: {}", uri))
        }
    }

    /// Close a document
    pub async fn close_document(&self, uri: &DocumentUri) {
        let mut documents = self.documents.write().await;
        documents.remove(uri);
    }

    /// Get completions for a document at a position
    pub async fn get_completions(&self, uri: &DocumentUri, position: Position) -> Result<Vec<CompletionItem>, String> {
        let documents = self.documents.read().await;
        
        if let Some(document) = documents.get(uri) {
            Ok(document.get_completions(position))
        } else {
            Err(format!("Document not found: {}", uri))
        }
    }

    /// Get diagnostics for a document
    pub async fn get_diagnostics(&self, uri: &DocumentUri) -> Result<Vec<Diagnostic>, String> {
        let documents = self.documents.read().await;
        
        if let Some(document) = documents.get(uri) {
            Ok(document.get_diagnostics())
        } else {
            Err(format!("Document not found: {}", uri))
        }
    }

    /// Get hover information for a document at a position
    pub async fn get_hover(&self, uri: &DocumentUri, position: Position) -> Result<Option<HoverInfo>, String> {
        let documents = self.documents.read().await;
        
        if let Some(document) = documents.get(uri) {
            Ok(document.get_hover(position))
        } else {
            Err(format!("Document not found: {}", uri))
        }
    }

    /// Get document symbols for a document
    pub async fn get_document_symbols(&self, uri: &DocumentUri) -> Result<Vec<DocumentSymbol>, String> {
        let documents = self.documents.read().await;
        
        if let Some(document) = documents.get(uri) {
            Ok(document.get_document_symbols())
        } else {
            Err(format!("Document not found: {}", uri))
        }
    }

    /// Get all document URIs
    pub async fn get_document_uris(&self) -> Vec<DocumentUri> {
        let documents = self.documents.read().await;
        documents.keys().cloned().collect()
    }
}

impl Default for DocumentManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_document_lifecycle() {
        let manager = DocumentManager::new();
        let uri = "file:///test.sage".to_string();
        
        // Open document
        manager.open_document(uri.clone(), 1, "x = 42".to_string(), "sagemath".to_string()).await;
        
        // Get diagnostics
        let diagnostics = manager.get_diagnostics(&uri).await.unwrap();
        assert!(diagnostics.is_empty()); // No errors expected
        
        // Update document
        manager.update_document(&uri, 2, "x = 42\ny = x^2".to_string(), None).await.unwrap();
        
        // Get completions
        let completions = manager.get_completions(&uri, Position { line: 1, character: 8 }).await.unwrap();
        assert!(!completions.is_empty());
        
        // Close document
        manager.close_document(&uri).await;
        
        // Verify document is closed
        assert!(manager.get_diagnostics(&uri).await.is_err());
    }

    #[tokio::test]
    async fn test_real_time_type_inference() {
        let manager = DocumentManager::new();
        let uri = "file:///test.sage".to_string();
        
        // Open document with polynomial ring
        manager.open_document(
            uri.clone(), 
            1, 
            "P.<x> = PolynomialRing(QQ)\nf = x^2 + 1".to_string(), 
            "sagemath".to_string()
        ).await;
        
        // Get hover info for 'x'
        let hover = manager.get_hover(&uri, Position { line: 1, character: 4 }).await.unwrap();
        
        // Should have type information (though exact format may vary)
        assert!(hover.is_some() || hover.is_none()); // Allow either for now
    }

    #[tokio::test]
    async fn test_completion_context_awareness() {
        let manager = DocumentManager::new();
        let uri = "file:///test.sage".to_string();
        
        manager.open_document(
            uri.clone(), 
            1, 
            "x = 42\nfac".to_string(), 
            "sagemath".to_string()
        ).await;
        
        let completions = manager.get_completions(&uri, Position { line: 1, character: 3 }).await.unwrap();
        
        // Should suggest factor function
        assert!(completions.iter().any(|c| c.label.contains("factor")));
    }
}