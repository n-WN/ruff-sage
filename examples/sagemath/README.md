# SageMath Examples

This directory contains example SageMath files demonstrating the syntax and features that ruff-sage-lsp handles.

## Files

- **`basic_examples.sage`**: Basic SageMath syntax including power operators, rational numbers, and matrix operations
- **`number_theory.sage`**: Number theory functions specific to SageMath like prime testing, factorization, and modular arithmetic

## Testing with ruff-sage-lsp

Once SageMath is installed, you can test the preprocessing with:

```bash
# Check if SageMath is available
cargo run -p ruff_sage_lsp --bin ruff-sage-lsp -- --check-sage

# Test preprocessing manually (for development)
cd examples/sagemath
sage --preparse basic_examples.sage
# This creates basic_examples.py with converted Python syntax
```

## Expected Behavior

The ruff-sage-lsp should:

1. **Preprocess** the `.sage` files to convert SageMath syntax to Python
2. **Analyze** the converted Python with Ruff's linting rules
3. **Map** any diagnostics back to the original `.sage` file locations
4. **Provide** LSP features like diagnostics, formatting, and code actions

## Common Transformations

- `^` → `**` (power operator)
- Rational number handling
- SageMath-specific function calls remain as-is
- Import statements and basic Python syntax preserved