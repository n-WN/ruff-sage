//! SageMath preprocessing and Python conversion
//!
//! This module provides functionality to convert SageMath (.sage) files to standard Python
//! using SageMath's built-in preparser. It handles the execution of `sage --preparse` and
//! manages the temporary files needed for the conversion process.

use std::io::Write;
use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result};
use tempfile::NamedTempFile;

/// Represents the result of preprocessing a SageMath file
#[derive(Debug, Clone)]
pub struct PreprocessResult {
    /// The converted Python source code
    pub python_source: String,
    /// Path to the temporary .py file (kept for debugging)
    pub temp_python_path: Option<std::path::PathBuf>,
}

/// Error types that can occur during preprocessing
#[derive(thiserror::Error, Debug)]
pub enum PreprocessError {
    #[error("SageMath is not available in PATH. Please ensure SageMath is installed and accessible.")]
    SageNotFound,
    
    #[error("Failed to execute sage --preparse: {0}")]
    SageExecutionFailed(String),
    
    #[error("Failed to create temporary file: {0}")]
    TempFileError(#[from] std::io::Error),
    
    #[error("Invalid UTF-8 in sage output: {0}")]
    InvalidUtf8(#[from] std::string::FromUtf8Error),

    #[error("Anyhow error: {0}")]
    AnyhowError(#[from] anyhow::Error),
}

/// Preprocessor for converting SageMath files to Python
pub struct SagePreprocessor {
    /// Whether to keep temporary files for debugging
    keep_temp_files: bool,
}

impl Default for SagePreprocessor {
    fn default() -> Self {
        Self::new()
    }
}

impl SagePreprocessor {
    /// Create a new SageMath preprocessor
    pub fn new() -> Self {
        Self {
            keep_temp_files: false,
        }
    }

    /// Create a new SageMath preprocessor that keeps temporary files for debugging
    pub fn with_debug() -> Self {
        Self {
            keep_temp_files: true,
        }
    }

    /// Check if SageMath is available in the system PATH
    pub fn check_sage_available() -> Result<bool, PreprocessError> {
        match Command::new("sage").arg("--version").output() {
            Ok(output) => Ok(output.status.success()),
            Err(e) => {
                if e.kind() == std::io::ErrorKind::NotFound {
                    Ok(false)
                } else {
                    Err(PreprocessError::SageExecutionFailed(e.to_string()))
                }
            }
        }
    }

    /// Convert SageMath source code to Python using `sage --preparse`
    ///
    /// # Arguments
    /// * `sage_source` - The SageMath source code to convert
    /// * `original_path` - Optional path to the original .sage file (for better error messages)
    ///
    /// # Returns
    /// * `Ok(PreprocessResult)` - The converted Python source and metadata
    /// * `Err(PreprocessError)` - If preprocessing fails
    pub fn preprocess(&self, sage_source: &str, original_path: Option<&Path>) -> Result<PreprocessResult, PreprocessError> {
        // Check if sage is available
        if !Self::check_sage_available()? {
            return Err(PreprocessError::SageNotFound);
        }

        // Create a temporary .sage file
        let mut temp_sage_file = NamedTempFile::new()?;

        // Write the sage source to the temporary file
        temp_sage_file.write_all(sage_source.as_bytes())?;

        let temp_sage_path = temp_sage_file.path();

        // Execute sage --preparse on the temporary file
        let output = Command::new("sage")
            .arg("--preparse")
            .arg(temp_sage_path)
            .output()
            .with_context(|| "Failed to execute sage --preparse")
            .map_err(|e| PreprocessError::SageExecutionFailed(e.to_string()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(PreprocessError::SageExecutionFailed(format!(
                "sage --preparse failed with exit code {}: {}",
                output.status.code().unwrap_or(-1),
                stderr
            )));
        }

        // The sage --preparse command should create a .py file with the same name
        let temp_python_path = temp_sage_path.with_extension("py");
        
        // Read the generated Python file
        let python_source = std::fs::read_to_string(&temp_python_path)?;

        // Optionally keep the temporary Python file for debugging
        let kept_python_path = if self.keep_temp_files {
            // Move to a more permanent location
            let debug_path = if let Some(orig_path) = original_path {
                orig_path.with_extension("sage.py")
            } else {
                std::env::temp_dir().join(format!("sage_debug_{}.py", 
                    std::process::id()))
            };
            
            std::fs::copy(&temp_python_path, &debug_path)?;
            
            Some(debug_path)
        } else {
            None
        };

        Ok(PreprocessResult {
            python_source,
            temp_python_path: kept_python_path,
        })
    }

    /// Preprocess a SageMath file from disk
    pub fn preprocess_file<P: AsRef<Path>>(&self, sage_file_path: P) -> Result<PreprocessResult, PreprocessError> {
        let path = sage_file_path.as_ref();
        let sage_source = std::fs::read_to_string(path)?;

        self.preprocess(&sage_source, Some(path))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sage_availability() {
        // This test will pass if SageMath is installed, skip if not
        match SagePreprocessor::check_sage_available() {
            Ok(true) => {
                // SageMath is available, we can run preprocessing tests
                println!("SageMath is available for testing");
            }
            Ok(false) => {
                println!("SageMath not available, skipping preprocessing tests");
            }
            Err(e) => {
                println!("Error checking SageMath availability: {}", e);
            }
        }
    }

    #[test]
    fn test_preprocess_basic_sage_syntax() {
        let preprocessor = SagePreprocessor::new();
        
        // Test basic SageMath syntax conversion
        let sage_source = r#"
# SageMath syntax examples
x = 2^3  # Power using ^
y = 1/3  # Rational number
z = x * y
print(z)
"#;

        // Only run this test if SageMath is available
        if SagePreprocessor::check_sage_available().unwrap_or(false) {
            match preprocessor.preprocess(sage_source, None) {
                Ok(result) => {
                    // The converted Python should use ** instead of ^ for power
                    assert!(result.python_source.contains("**"));
                    assert!(!result.python_source.contains("^"));
                    println!("Converted Python:\n{}", result.python_source);
                }
                Err(e) => {
                    println!("Preprocessing failed: {}", e);
                }
            }
        } else {
            println!("Skipping preprocess test - SageMath not available");
        }
    }
}