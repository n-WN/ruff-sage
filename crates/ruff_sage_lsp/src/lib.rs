//! # Ruff SageMath Language Server
//!
//! This crate provides Language Server Protocol (LSP) support for SageMath files (.sage).
//! It implements a preprocessing gateway architecture that leverages SageMath's built-in
//! preparser to convert SageMath syntax to standard Python, then applies Ruff's analysis
//! capabilities to provide diagnostics, formatting, and other language features.
//!
//! ## Architecture Overview
//!
//! The core challenge is that SageMath uses syntax extensions that differ from standard
//! Python (e.g., `^` for exponentiation, implicit integer types). Rather than modifying
//! Ruff's core parser, this implementation uses a preprocessing gateway:
//!
//! 1. **Input**: SageMath (.sage) files with domain-specific syntax
//! 2. **Preprocessing**: Use `sage --preparse` to convert to standard Python
//! 3. **Analysis**: Apply Ruff's linting and formatting to the converted Python
//! 4. **Mapping**: Translate diagnostics back to original SageMath source locations
//!
//! ## Key Components
//!
//! - [`preprocess`]: SageMath preprocessing and Python conversion
//! - [`source_map`]: Bidirectional mapping between .sage and .py coordinates  
//! - [`server`]: LSP server implementation with SageMath-specific handlers
//! - [`session`]: Session management for .sage documents

use std::num::NonZeroUsize;

use anyhow::Context as _;
pub use preprocess::{PreprocessError, PreprocessResult, SagePreprocessor};
pub use server::{ConnectionSender, MainLoopSender, Server};
pub use session::{Client, ClientOptions, DocumentQuery, DocumentSnapshot, GlobalOptions, Session};
pub use source_map::{Position, Range, SourceMap};

use crate::server::ConnectionInitializer;

mod preprocess;
mod source_map;
mod server;
mod session;

pub(crate) const SERVER_NAME: &str = "ruff-sage";
pub(crate) const DIAGNOSTIC_NAME: &str = "Ruff-Sage";

/// A common result type used in most cases where a
/// result type is needed.
pub(crate) type Result<T> = anyhow::Result<T>;

pub fn version() -> &'static str {
    ruff_linter::VERSION
}

/// Run the Ruff SageMath Language Server
pub fn run(preview: Option<bool>) -> Result<()> {
    let four = NonZeroUsize::new(4).unwrap();

    // by default, we set the number of worker threads to `num_cpus`, with a maximum of 4.
    let worker_threads = std::thread::available_parallelism()
        .unwrap_or(four)
        .min(four);

    let (connection, io_threads) = ConnectionInitializer::stdio();

    let server_result = Server::new(worker_threads, connection, preview)
        .context("Failed to start Ruff-Sage server")?
        .run();

    let io_result = io_threads.join();

    let result = match (server_result, io_result) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(server), Err(io)) => Err(server).context(format!("IO thread error: {io}")),
        (Err(server), _) => Err(server),
        (_, Err(io)) => Err(io).context("IO thread error"),
    };

    if let Err(err) = result.as_ref() {
        tracing::warn!("Ruff-Sage server shut down with an error: {err}");
    } else {
        tracing::info!("Ruff-Sage server shut down");
    }

    result
}