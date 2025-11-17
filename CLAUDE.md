# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands
- Install dependencies: `uv sync`
- Install with dev dependencies: `uv sync --all-groups`
- Run the application: `uv run main.py` (auto-detects latest PDF) or `uv run main.py <PDF File>`
- Batch import PDFs: `uv run python batch/import.py --import`
- Export to CSV: `uv run python batch/import.py --csv <output_file>.csv`
- Export to JSON: `uv run python batch/import.py --json <output_file>.json`
- Run all tests: `uv run pytest`
- Run a specific test: `uv run pytest tests/replace_text.py::test_replace_spaces`
- Format code: `uv run black .`
- Pre-commit hooks: `uv run pre-commit run --all-files`

## Commit Message Style
- **Always use the `/commit` slash command** for consistency
- **Format**: Follow conventional commits (e.g., `feat:`, `fix:`, `chore:`, `docs:`, `refactor:`, `test:`)
- **Case**: Use lowercase for the description after the prefix
- **Punctuation**: No trailing periods
- **Length**: Keep messages short and concise

Examples:
- `chore: release 0.3.1`
- `fix: correct chart paths to charts directory`
- `feat: add new revenue analysis function`

## Project Structure
```
├── main.py                # Main entry point
├── src/                   # Source code library
│   ├── read_pdf.py        # PDF parsing
│   ├── charts.py          # Chart generation functions
│   ├── database.py        # SQLite database operations
│   └── utils/             # Utility modules
│       ├── find_latest_pdf.py
│       ├── calculate_growth_rate.py
│       └── replace_text.py
├── batch/                 # Batch processing scripts
│   └── import.py          # Batch import & export tool
├── data/                  # PDF files and database
│   ├── *.pdf              # Quarterly revenue PDFs
│   └── data.db            # SQLite database (generated)
├── charts/                # Generated chart outputs
└── tests/                 # Test files
```

## Project Overview
This project analyses NVIDIA quarterly revenue PDFs and generates visualisation charts. Key components:
- **main.py**: Main script - extracts data, calculates growth rates, generates stacked bar chart with revenue trend lines
- **src/read_pdf.py**: PDF parsing using pdfplumber to extract quarterly revenue data
- **src/charts.py**: Chart generation functions for all 8 visualisation types
- **src/database.py**: SQLite database operations for storing historical data, querying, and exporting to CSV/JSON
- **batch/import.py**: Batch import tool to process all PDFs in data/ folder and export data
- **src/utils/find_latest_pdf.py**: Auto-detects latest PDF by parsing quarter/year from filenames (e.g., Q226 = Q2 FY26)
- **src/utils/calculate_growth_rate.py**: Calculates quarter-over-quarter growth percentages
- **src/utils/replace_text.py**: Formats segment labels for display (e.g., "data_center" → "Data Centre")

## Database Features
- **SQLite Storage**: All quarterly revenue data stored in `data/data.db` for fast querying
- **Batch Import**: Process multiple PDFs at once with `batch/import.py --import`
- **CSV Export**: Export data for spreadsheet analysis with `--csv` flag
- **JSON Export**: Export data for programmatic use with `--json` flag
- **Query Methods**: Get all quarters, latest N quarters, or filter by date range
- **Import History**: Track which PDFs have been imported and when

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
