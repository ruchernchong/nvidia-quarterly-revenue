# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands
- Install dependencies: `uv pip install -e .` or `uv pip install -e ".[dev]"` for dev dependencies
- Run the application: `python main.py <PDF File>`
- Run all tests: `pytest`
- Run a specific test: `pytest tests/test_replace_text.py::test_replace_spaces`
- Format code: `black .`
- Pre-commit hooks: `pre-commit run --all-files`

## Code Style Guidelines
- **Formatting**: Use Black for code formatting
- **Imports**: Group imports by standard library, third-party, and local modules with a blank line between groups
- **Types**: Use type hints for function parameters and return values
- **Naming**: Use snake_case for variables, functions, and file names
- **Error Handling**: Use try/except blocks with specific exception types
- **Documentation**: Add docstrings for functions with parameters and return types
- **Pre-commit Hooks**: Ensure trailing whitespace is removed and files end with a newline
