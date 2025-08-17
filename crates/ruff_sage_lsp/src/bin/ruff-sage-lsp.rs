//! Ruff SageMath Language Server binary
//!
//! This binary provides the `ruff-sage-lsp` command-line interface for running
//! the SageMath Language Server Protocol implementation.

use std::io::{self, IsTerminal};

use anyhow::Result;
use clap::{Parser, ValueEnum};

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum LogLevel {
    /// Print nothing
    Silent,
    /// Print errors only
    Error,
    /// Print errors and warnings
    Warn,
    /// Print errors, warnings, and info
    Info,
    /// Print all messages including debug info
    Debug,
    /// Print all messages including trace info  
    Trace,
}

impl From<LogLevel> for tracing::Level {
    fn from(level: LogLevel) -> Self {
        match level {
            LogLevel::Silent => tracing::Level::ERROR, // Will be filtered out by setup
            LogLevel::Error => tracing::Level::ERROR,
            LogLevel::Warn => tracing::Level::WARN,
            LogLevel::Info => tracing::Level::INFO,
            LogLevel::Debug => tracing::Level::DEBUG,
            LogLevel::Trace => tracing::Level::TRACE,
        }
    }
}

#[derive(Parser)]
#[command(
    name = "ruff-sage-lsp",
    about = "Ruff SageMath Language Server - LSP support for SageMath files",
    long_about = "A Language Server Protocol implementation that brings Ruff's code analysis \
                  capabilities to SageMath files through preprocessing gateway architecture."
)]
struct Args {
    /// Set the logging level
    #[arg(long, value_enum, default_value = "error")]
    log_level: LogLevel,

    /// Enable preview mode features
    #[arg(long)]
    preview: bool,

    /// Check SageMath availability without starting the server
    #[arg(long)]
    check_sage: bool,

    /// Show version information
    #[arg(long)]
    version: bool,
}

fn setup_logging(level: LogLevel) -> Result<()> {
    if level == LogLevel::Silent {
        return Ok(());
    }

    let subscriber = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::from(level))
        .with_writer(io::stderr) // Write logs to stderr to not interfere with LSP protocol
        .finish();

    tracing::subscriber::set_global_default(subscriber)?;
    Ok(())
}

fn check_sage_installation() -> Result<()> {
    use ruff_sage_lsp::SagePreprocessor;

    println!("Checking SageMath installation...");
    
    match SagePreprocessor::check_sage_available() {
        Ok(true) => {
            println!("✓ SageMath is available and accessible");
            
            // Test basic preprocessing
            let preprocessor = SagePreprocessor::new();
            let test_sage = "x = 2^3\nprint(x)";
            
            match preprocessor.preprocess(test_sage, None) {
                Ok(result) => {
                    println!("✓ SageMath preprocessing is working");
                    println!("  Original: x = 2^3");
                    println!("  Converted: {}", result.python_source.lines().next().unwrap_or(""));
                }
                Err(e) => {
                    println!("✗ SageMath preprocessing failed: {}", e);
                    return Err(e.into());
                }
            }
        }
        Ok(false) => {
            println!("✗ SageMath is not available in PATH");
            println!("  Please ensure SageMath is installed and accessible via 'sage' command");
            std::process::exit(1);
        }
        Err(e) => {
            println!("✗ Error checking SageMath: {}", e);
            return Err(e.into());
        }
    }

    Ok(())
}

fn main() -> Result<()> {
    let args = Args::parse();

    if args.version {
        println!("ruff-sage-lsp {}", ruff_sage_lsp::version());
        println!("Using Ruff core: {}", ruff_linter::VERSION);
        return Ok(());
    }

    setup_logging(args.log_level)?;

    if args.check_sage {
        return check_sage_installation();
    }

    // Check if we're being invoked as a language server (stdin is not a tty)
    if std::io::stdin().is_terminal() {
        eprintln!("ruff-sage-lsp: Language Server for SageMath");
        eprintln!();
        eprintln!("This program implements the Language Server Protocol for SageMath files.");
        eprintln!("It should be invoked by an LSP client (like VS Code, Vim, Emacs, etc.)");
        eprintln!();
        eprintln!("To test SageMath integration, run: ruff-sage-lsp --check-sage");
        eprintln!("For more options, run: ruff-sage-lsp --help");
        std::process::exit(1);
    }

    tracing::info!("Starting Ruff SageMath Language Server");
    tracing::info!("Preview mode: {}", args.preview);

    // Run the language server
    ruff_sage_lsp::run(if args.preview { Some(true) } else { None })?;

    tracing::info!("Ruff SageMath Language Server shutdown");
    Ok(())
}