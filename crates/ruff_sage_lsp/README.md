# Ruff SageMath Language Server

A Language Server Protocol (LSP) implementation that brings Ruff's high-performance code analysis capabilities to SageMath files (.sage).

## Architecture Overview

The core challenge is that SageMath uses syntax extensions that differ from standard Python (e.g., `^` for exponentiation, implicit integer types). Rather than modifying Ruff's core parser, this implementation uses a **preprocessing gateway** architecture:

1. **Input**: SageMath (.sage) files with domain-specific syntax
2. **Preprocessing**: Use `sage --preparse` to convert to standard Python
3. **Analysis**: Apply Ruff's linting and formatting to the converted Python
4. **Mapping**: Translate diagnostics back to original SageMath source locations

## Installation

### Prerequisites

- **SageMath**: Must be installed and accessible via the `sage` command
- **Rust**: For building from source

### Building

```bash
cargo build -p ruff_sage_lsp
```

### Running

```bash
# Check SageMath installation
cargo run -p ruff_sage_lsp --bin ruff-sage-lsp -- --check-sage

# Show version
cargo run -p ruff_sage_lsp --bin ruff-sage-lsp -- --version

# Run as language server (typically invoked by LSP client)
cargo run -p ruff_sage_lsp --bin ruff-sage-lsp
```

## Usage

### With VS Code

1. Install the Ruff extension for VS Code
2. Configure the extension to use the custom `ruff-sage-lsp` binary
3. Associate `.sage` files with the language server

### With Other Editors

Configure your editor's LSP client to:
- Associate `.sage` files with the `ruff-sage-lsp` command
- Set the server command: `ruff-sage-lsp`

## Architecture Details

### Key Components

- **`preprocess`**: SageMath preprocessing and Python conversion
- **`source_map`**: Bidirectional mapping between .sage and .py coordinates  
- **`server`**: LSP server implementation with SageMath-specific handlers
- **`session`**: Session management for .sage documents

### Preprocessing Pipeline

1. **Input**: Original `.sage` file with SageMath syntax
2. **Conversion**: Execute `sage --preparse file.sage` → `file.py`
3. **Analysis**: Run Ruff analysis on the generated Python code
4. **Mapping**: Translate line/column positions back to original source
5. **Response**: Send LSP diagnostics pointing to original `.sage` locations

### Source Mapping

The source mapping system handles the translation between SageMath and Python coordinates:

- **Line Mapping**: Tracks which lines in Python correspond to lines in SageMath
- **Character Mapping**: Accounts for syntax transformations (e.g., `^` → `**`)
- **Range Translation**: Converts diagnostic ranges back to original source

## Example

Given this SageMath code:

```sage
# file.sage
x = 2^3          # SageMath power syntax
y = 1/3          # Rational number
z = x * y
print(z)
```

The preprocessor converts it to:

```python
# file.py (generated)
x = 2**3         # Standard Python power
y = 1/3          # Python handles this
z = x * y
print(z)
```

Ruff analyzes the Python version and any diagnostics are mapped back to the original SageMath locations.

## Configuration

Currently supports standard Ruff configuration through:
- `pyproject.toml`
- `ruff.toml` 
- `.ruff.toml`

SageMath-specific configuration options will be added in future versions.

## Limitations

- **SageMath Dependency**: Requires SageMath installation and `sage` command availability
- **Preprocessing Overhead**: Each analysis requires running `sage --preparse`
- **Limited Syntax Coverage**: Currently handles basic syntax transformations; complex SageMath constructs may need additional mapping logic

## Development

### Running Tests

```bash
cargo test -p ruff_sage_lsp
```

### Testing with SageMath

If SageMath is available, the tests will automatically use it for preprocessing validation:

```bash
# This will test actual SageMath integration
cargo test -p ruff_sage_lsp -- test_preprocess_basic_sage_syntax
```

## Future Enhancements

- **Enhanced Source Mapping**: More sophisticated mapping for complex syntax transformations
- **Caching**: Cache preprocessing results to improve performance
- **Incremental Updates**: Support for incremental document changes
- **Advanced SageMath Features**: Support for more SageMath-specific constructs
- **Error Recovery**: Better error handling for malformed SageMath syntax

## Contributing

See the main Ruff [CONTRIBUTING.md](../../CONTRIBUTING.md) for general guidelines.

For ruff-sage-lsp specific development:

1. Ensure SageMath is installed for testing
2. Run tests with `cargo test -p ruff_sage_lsp`
3. Test the binary with `cargo run -p ruff_sage_lsp --bin ruff-sage-lsp -- --check-sage`