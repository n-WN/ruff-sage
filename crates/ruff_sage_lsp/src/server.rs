//! Server implementation for Ruff SageMath LSP

use std::num::NonZeroUsize;

use lsp_server::{Connection, IoThreads};

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
        })
    }

    pub fn run(self) -> anyhow::Result<()> {
        // TODO: Implement the main server loop for SageMath LSP
        // This will be similar to ruff_server but with SageMath-specific handling
        
        tracing::info!("Starting Ruff-Sage LSP server");
        tracing::info!("Worker threads: {}", self.worker_threads);
        tracing::info!("Preview mode: {:?}", self.preview);
        
        // For now, just return success - full implementation will follow
        Ok(())
    }
}