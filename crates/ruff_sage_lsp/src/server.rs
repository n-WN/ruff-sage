//! Server implementation for Ruff SageMath LSP with real-time analysis
//!
//! This module implements a full LSP server for SageMath that provides:
//! - Real-time syntax analysis and type inference
//! - Incremental parsing without external preprocessing
//! - Context-aware autocompletion
//! - Semantic error detection and reporting

use std::num::NonZeroUsize;
use std::sync::Arc;

use lsp_server::{Connection, IoThreads, Message, Notification, Request, RequestId, Response};
use lsp_types::{
    request::{GotoDefinition, HoverRequest, Completion, DocumentSymbolRequest, Request as RequestTrait},
    notification::{DidOpenTextDocument, DidChangeTextDocument, DidCloseTextDocument, Notification as NotificationTrait},
    InitializeParams, InitializeResult, ServerCapabilities, TextDocumentSyncCapability,
    TextDocumentSyncKind, CompletionOptions, HoverProviderCapability,
    OneOf,
};
use serde_json::Value;

use crate::document_manager::DocumentManager;

pub struct Server {
    worker_threads: NonZeroUsize,
    connection: Connection,
    preview: Option<bool>,
}

pub struct ConnectionInitializer;

pub struct ConnectionSender;
pub struct MainLoopSender;

impl ConnectionInitializer {
    pub fn stdio() -> (Connection, IoThreads) {
        Connection::stdio()
    }
}

impl Server {
    pub fn new(
        worker_threads: NonZeroUsize,
        connection: Connection,
        preview: Option<bool>,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            worker_threads,
            connection,
            preview,
            document_manager: Arc::new(DocumentManager::new()),
        })
    }

    pub fn run(mut self) -> anyhow::Result<()> {
        tracing::info!("Starting Ruff-Sage LSP server");
        tracing::info!("Worker threads: {}", self.worker_threads);
        tracing::info!("Preview mode: {:?}", self.preview);
        
        // Initialize the server
        let initialization_params = self.initialize()?;
        tracing::info!("Server initialized with params: {:?}", initialization_params.client_info);
        
        // Run the main loop
        self.main_loop()?;
        
        Ok(())
    }

    /// Handle the LSP initialization
    fn initialize(&mut self) -> anyhow::Result<InitializeParams> {
        let (initialize_id, initialize_params) = self.connection.initialize_start()?;
        
        let initialize_params: InitializeParams = serde_json::from_value(initialize_params)?;
        
        let server_capabilities = ServerCapabilities {
            text_document_sync: Some(TextDocumentSyncCapability::Kind(
                TextDocumentSyncKind::INCREMENTAL,
            )),
            completion_provider: Some(CompletionOptions {
                trigger_characters: Some(vec![
                    ".".to_string(),
                    "^".to_string(),
                    "*".to_string(),
                    "<".to_string(),
                ]),
                ..Default::default()
            }),
            hover_provider: Some(HoverProviderCapability::Simple(true)),
            document_symbol_provider: Some(OneOf::Left(true)),
            ..Default::default()
        };

        let initialize_result = InitializeResult {
            capabilities: server_capabilities,
            server_info: Some(lsp_types::ServerInfo {
                name: "ruff-sage-lsp".to_string(),
                version: Some(crate::version().to_string()),
            }),
        };

        self.connection.initialize_finish(initialize_id, serde_json::to_value(initialize_result)?)?;
        
        Ok(initialize_params)
    }

    /// Main message processing loop
    fn main_loop(&mut self) -> anyhow::Result<()> {
        for msg in &self.connection.receiver {
            match msg {
                Message::Request(req) => {
                    if self.connection.handle_shutdown(&req)? {
                        break;
                    }
                    self.handle_request(req)?;
                }
                Message::Notification(not) => {
                    self.handle_notification(not)?;
                }
                Message::Response(_) => {
                    // We don't expect responses from the client in this server
                }
            }
        }
        
        Ok(())
    }

    /// Handle LSP requests
    fn handle_request(&mut self, req: Request) -> anyhow::Result<()> {
        let req_id = req.id.clone();
        
        match req.method.as_str() {
            Completion::METHOD => {
                self.handle_completion_request(req_id, req.params)?;
            }
            HoverRequest::METHOD => {
                self.handle_hover_request(req_id, req.params)?;
            }
            DocumentSymbolRequest::METHOD => {
                self.handle_document_symbol_request(req_id, req.params)?;
            }
            GotoDefinition::METHOD => {
                self.handle_goto_definition_request(req_id, req.params)?;
            }
            _ => {
                // Send method not found error
                let error = lsp_server::ErrorCode::MethodNotFound as i32;
                let response = Response::new_err(req_id, error, "Method not found".to_string());
                self.connection.sender.send(Message::Response(response))?;
            }
        }
        
        Ok(())
    }

    /// Handle LSP notifications
    fn handle_notification(&mut self, not: Notification) -> anyhow::Result<()> {
        match not.method.as_str() {
            <DidOpenTextDocument as NotificationTrait>::METHOD => {
                self.handle_did_open(not.params)?;
            }
            <DidChangeTextDocument as NotificationTrait>::METHOD => {
                self.handle_did_change(not.params)?;
            }
            <DidCloseTextDocument as NotificationTrait>::METHOD => {
                self.handle_did_close(not.params)?;
            }
            _ => {
                tracing::debug!("Unhandled notification: {}", not.method);
            }
        }
        
        Ok(())
    }

    /// Handle document open notification
    fn handle_did_open(&mut self, params: Value) -> anyhow::Result<()> {
        let params: lsp_types::DidOpenTextDocumentParams = serde_json::from_value(params)?;
        
        let uri = params.text_document.uri.to_string();
        let version = params.text_document.version;
        let content = params.text_document.text;
        let language_id = params.text_document.language_id;
        
        // Open the document in our manager
        let document_manager = self.document_manager.clone();
        tokio::spawn(async move {
            document_manager.open_document(uri.clone(), version, content, language_id).await;
            
            // Send initial diagnostics
            if let Ok(diagnostics) = document_manager.get_diagnostics(&uri).await {
                // Convert to LSP diagnostics and send
                // TODO: Implement diagnostic sending
                tracing::debug!("Document {} has {} diagnostics", uri, diagnostics.len());
            }
        });
        
        Ok(())
    }

    /// Handle document change notification
    fn handle_did_change(&mut self, params: Value) -> anyhow::Result<()> {
        let params: lsp_types::DidChangeTextDocumentParams = serde_json::from_value(params)?;
        
        let uri = params.text_document.uri.to_string();
        let version = params.text_document.version;
        
        // Handle incremental changes
        let changes = if let Some(changes) = params.content_changes.first() {
            if let Some(range) = changes.range {
                // Incremental change
                Some(vec![crate::document_manager::DocumentChange {
                    range: crate::realtime_analyzer::Range {
                        start: crate::realtime_analyzer::Position {
                            line: range.start.line,
                            character: range.start.character,
                        },
                        end: crate::realtime_analyzer::Position {
                            line: range.end.line,
                            character: range.end.character,
                        },
                    },
                    text: changes.text.clone(),
                }])
            } else {
                // Full document change
                None
            }
        } else {
            None
        };

        let content = params.content_changes.first()
            .map(|c| c.text.clone())
            .unwrap_or_default();

        // Update the document
        let document_manager = self.document_manager.clone();
        tokio::spawn(async move {
            if let Err(e) = document_manager.update_document(&uri, version, content, changes).await {
                tracing::error!("Failed to update document {}: {}", uri, e);
            }
            
            // Send updated diagnostics
            if let Ok(diagnostics) = document_manager.get_diagnostics(&uri).await {
                tracing::debug!("Document {} has {} diagnostics after update", uri, diagnostics.len());
            }
        });
        
        Ok(())
    }

    /// Handle document close notification
    fn handle_did_close(&mut self, params: Value) -> anyhow::Result<()> {
        let params: lsp_types::DidCloseTextDocumentParams = serde_json::from_value(params)?;
        
        let uri = params.text_document.uri.to_string();
        
        let document_manager = self.document_manager.clone();
        tokio::spawn(async move {
            document_manager.close_document(&uri).await;
        });
        
        Ok(())
    }

    /// Handle completion request
    fn handle_completion_request(&mut self, req_id: RequestId, params: Value) -> anyhow::Result<()> {
        let params: lsp_types::CompletionParams = serde_json::from_value(params)?;
        
        let uri = params.text_document_position.text_document.uri.to_string();
        let position = crate::realtime_analyzer::Position {
            line: params.text_document_position.position.line,
            character: params.text_document_position.position.character,
        };
        
        let document_manager = self.document_manager.clone();
        let sender = self.connection.sender.clone();
        
        tokio::spawn(async move {
            let response = match document_manager.get_completions(&uri, position).await {
                Ok(completions) => {
                    // Convert to LSP completion items
                    let lsp_completions: Vec<lsp_types::CompletionItem> = completions
                        .into_iter()
                        .map(|c| lsp_types::CompletionItem {
                            label: c.label,
                            insert_text: Some(c.insert_text),
                            detail: c.detail,
                            kind: Some(match c.kind {
                                crate::realtime_analyzer::CompletionKind::Variable => lsp_types::CompletionItemKind::VARIABLE,
                                crate::realtime_analyzer::CompletionKind::Function => lsp_types::CompletionItemKind::FUNCTION,
                                crate::realtime_analyzer::CompletionKind::Keyword => lsp_types::CompletionItemKind::KEYWORD,
                                crate::realtime_analyzer::CompletionKind::Operator => lsp_types::CompletionItemKind::OPERATOR,
                                crate::realtime_analyzer::CompletionKind::Constructor => lsp_types::CompletionItemKind::CONSTRUCTOR,
                            }),
                            ..Default::default()
                        })
                        .collect();
                    
                    Response::new_ok(req_id, lsp_completions)
                }
                Err(e) => {
                    tracing::error!("Completion error: {}", e);
                    Response::new_err(req_id, lsp_server::ErrorCode::InternalError as i32, e)
                }
            };
            
            if let Err(e) = sender.send(Message::Response(response)) {
                tracing::error!("Failed to send completion response: {}", e);
            }
        });
        
        Ok(())
    }

    /// Handle hover request
    fn handle_hover_request(&mut self, req_id: RequestId, params: Value) -> anyhow::Result<()> {
        let params: lsp_types::HoverParams = serde_json::from_value(params)?;
        
        let uri = params.text_document_position_params.text_document.uri.to_string();
        let position = crate::realtime_analyzer::Position {
            line: params.text_document_position_params.position.line,
            character: params.text_document_position_params.position.character,
        };
        
        let document_manager = self.document_manager.clone();
        let sender = self.connection.sender.clone();
        
        tokio::spawn(async move {
            let response = match document_manager.get_hover(&uri, position).await {
                Ok(hover_info) => {
                    let hover = hover_info.map(|info| lsp_types::Hover {
                        contents: lsp_types::HoverContents::Scalar(
                            lsp_types::MarkedString::String(info.content)
                        ),
                        range: info.range.map(|r| lsp_types::Range {
                            start: lsp_types::Position {
                                line: r.start.line,
                                character: r.start.character,
                            },
                            end: lsp_types::Position {
                                line: r.end.line,
                                character: r.end.character,
                            },
                        }),
                    });
                    
                    Response::new_ok(req_id, hover)
                }
                Err(e) => {
                    tracing::error!("Hover error: {}", e);
                    Response::new_err(req_id, lsp_server::ErrorCode::InternalError as i32, e)
                }
            };
            
            if let Err(e) = sender.send(Message::Response(response)) {
                tracing::error!("Failed to send hover response: {}", e);
            }
        });
        
        Ok(())
    }

    /// Handle document symbol request
    fn handle_document_symbol_request(&mut self, req_id: RequestId, params: Value) -> anyhow::Result<()> {
        let params: lsp_types::DocumentSymbolParams = serde_json::from_value(params)?;
        
        let uri = params.text_document.uri.to_string();
        
        let document_manager = self.document_manager.clone();
        let sender = self.connection.sender.clone();
        
        tokio::spawn(async move {
            let response = match document_manager.get_document_symbols(&uri).await {
                Ok(symbols) => {
                    // Convert to LSP document symbols
                    let lsp_symbols: Vec<lsp_types::DocumentSymbol> = symbols
                        .into_iter()
                        .map(|s| lsp_types::DocumentSymbol {
                            name: s.name,
                            kind: lsp_types::SymbolKind::VARIABLE, // Simplified for now
                            range: lsp_types::Range {
                                start: lsp_types::Position {
                                    line: s.range.start.line,
                                    character: s.range.start.character,
                                },
                                end: lsp_types::Position {
                                    line: s.range.end.line,
                                    character: s.range.end.character,
                                },
                            },
                            selection_range: lsp_types::Range {
                                start: lsp_types::Position {
                                    line: s.selection_range.start.line,
                                    character: s.selection_range.start.character,
                                },
                                end: lsp_types::Position {
                                    line: s.selection_range.end.line,
                                    character: s.selection_range.end.character,
                                },
                            },
                            children: None, // Simplified for now
                            detail: None,
                            tags: None,
                            deprecated: None,
                        })
                        .collect();
                    
                    Response::new_ok(req_id, lsp_symbols)
                }
                Err(e) => {
                    tracing::error!("Document symbols error: {}", e);
                    Response::new_err(req_id, lsp_server::ErrorCode::InternalError as i32, e)
                }
            };
            
            if let Err(e) = sender.send(Message::Response(response)) {
                tracing::error!("Failed to send document symbols response: {}", e);
            }
        });
        
        Ok(())
    }

    /// Handle goto definition request
    fn handle_goto_definition_request(&mut self, req_id: RequestId, _params: Value) -> anyhow::Result<()> {
        // For now, return empty result
        let response = Response::new_ok(req_id, Option::<lsp_types::GotoDefinitionResponse>::None);
        self.connection.sender.send(Message::Response(response))?;
        Ok(())
    }
}}
}