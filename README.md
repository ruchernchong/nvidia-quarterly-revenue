# NVIDIA Quarterly Revenue

![NVIDIA Revenue Trend](nvidia-revenue-trend.png)

A Python tool that extracts and visualises NVIDIA's quarterly revenue data from PDF reports. The tool generates comprehensive charts showing revenue breakdown by market segment with growth rate trends.

## Features

- **Automated PDF Processing**: Automatically detects and processes the latest quarterly PDF from the `data/` directory
- **Stacked Bar Chart**: Visualises revenue breakdown across five market segments:
  - Data Centre
  - Gaming
  - Professional Visualisation
  - Automotive
  - OEM & Other
- **Growth Rate Tracking**: Displays quarter-over-quarter growth percentages for total and data centre revenue
- **Dual Revenue Lines**: Overlays total revenue and data centre revenue trend lines on the chart
- **Dynamic Scaling**: Chart width adjusts based on the number of quarters displayed (currently showing all 8 quarters from the PDF)

## Installation

Install dependencies:
```bash
uv sync
```

Or with development dependencies:
```bash
uv sync --all-groups
```

## Usage

Run with a specific PDF file:
```bash
uv run python main.py data/<PDF File>
```

Or run without arguments to automatically use the latest quarterly PDF from the `data/` directory:
```bash
uv run python main.py
```

The script automatically detects the latest file by parsing the quarter and year from filenames (e.g., Q226 = Q2 FY26).

**Note:** PDF files should be placed in the `data/` directory.

### Output

The script generates:
- **Console Output**: Quarter-over-quarter growth rates for total revenue
- **Chart Image**: `nvidia-revenue-trend.png` containing:
  - Stacked bar chart showing revenue by market segment
  - Total revenue trend line with growth rate annotations
  - Data centre revenue trend line with growth rate annotations
  - Dynamic width based on number of quarters (3 inches per quarter, minimum 12 inches)

## Development

### Running Tests

```bash
uv run pytest
```

Run a specific test:
```bash
uv run pytest tests/test_replace_text.py::test_replace_spaces
```

### Code Formatting

Format code with Black:
```bash
uv run black .
```

### Pre-commit Hooks

Run pre-commit hooks (trailing whitespace removal, EOF fixes, merge conflict checks):
```bash
uv run pre-commit run --all-files
```

Install hooks to run automatically on commit:
```bash
uv run pre-commit install
```

## Project Structure

```
nvidia-quarterly-revenue/
├── data/                   # PDF files directory
├── main.py                 # Main script for data extraction and visualisation
├── read_pdf.py            # PDF parsing logic
├── utils/
│   ├── calculate_growth_rate.py  # Growth rate calculation
│   ├── find_latest_pdf.py        # Latest PDF detection
│   ├── replace_text.py           # Text formatting utilities
│   └── download_pdf.py           # PDF download automation
├── tests/                 # Test suite
└── nvidia-revenue-trend.png  # Generated chart output
```

## Automated Workflows

The project includes GitHub Actions workflows for:
- **PDF Monitor**: Automatically checks for new quarterly PDFs and downloads them
- **Testing**: Runs pytest on pull requests and pushes
- **PDF Downloader**: Manual workflow to download specific quarterly PDFs

## Requirements

- Python ~3.12
- Dependencies (managed by uv):
  - matplotlib >= 3.10.1
  - pdfminer-six >= 20250327
  - pdfplumber >= 0.11.6
  - requests >= 2.32.0
