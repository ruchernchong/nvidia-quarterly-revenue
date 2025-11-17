# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands
- Install dependencies: `uv sync`
- Install with dev dependencies: `uv sync --all-groups`
- Run the application: `uv run main.py` (auto-detects latest PDF) or `uv run main.py <PDF File>`
- Run all tests: `uv run pytest`
- Run a specific test: `uv run pytest tests/test_replace_text.py::test_replace_spaces`
- Format code: `uv run black .`
- Pre-commit hooks: `uv run pre-commit run --all-files`

## Project Structure
```
├── main.py                # Main entry point
├── src/                   # Source code library
│   ├── read_pdf.py        # PDF parsing
│   ├── charts.py          # Chart generation functions
│   └── utils/             # Utility modules
│       ├── find_latest_pdf.py
│       ├── calculate_growth_rate.py
│       └── replace_text.py
├── data/                  # PDF files
├── charts/                # Generated chart outputs
└── tests/                 # Test files
```

## Project Overview
This project analyses NVIDIA quarterly revenue PDFs and generates visualisation charts. Key components:
- **main.py**: Main script - extracts data, calculates growth rates, generates stacked bar chart with revenue trend lines
- **src/read_pdf.py**: PDF parsing using pdfplumber to extract quarterly revenue data
- **src/charts.py**: Chart generation functions for all 8 visualisation types
- **src/utils/find_latest_pdf.py**: Auto-detects latest PDF by parsing quarter/year from filenames (e.g., Q226 = Q2 FY26)
- **src/utils/calculate_growth_rate.py**: Calculates quarter-over-quarter growth percentages
- **src/utils/replace_text.py**: Formats segment labels for display (e.g., "data_center" → "Data Centre")

## Chart Configuration
- **Display**: Shows all 8 quarters from the PDF (no filtering)
- **Format**: Stacked bar chart with dual revenue trend lines (total + data centre)
- **Width**: Dynamic - 3 inches per quarter, minimum 12 inches
- **Segments**: Data Centre, Gaming, Professional Visualisation, Automotive, OEM & Other
- **Annotations**: Growth rates displayed on both total and data centre trend lines

## Code Style Guidelines
- **Formatting**: Use Black for code formatting
- **Imports**: Group imports by standard library, third-party, and local modules with a blank line between groups
- **Types**: Use type hints for function parameters and return values
- **Naming**: Use snake_case for variables, functions, and file names
- **Error Handling**: Use try/except blocks with specific exception types
- **Documentation**: Add docstrings for functions with parameters and return types
- **Pre-commit Hooks**: Ensure trailing whitespace is removed and files end with a newline
- **Spelling**: Use English (UK/Singapore) spelling (e.g., "visualise", "centre", "colour")
