# Nvidia Quarterly Revenue

## Installation

```bash
uv pip install -e .
```

Or with development dependencies:

```bash
uv pip install -e ".[dev]"
```

## Usage

Run with a specific PDF file:
```bash
python main.py data/<PDF File>
```

Or run without arguments to automatically use the latest quarterly PDF from the `data/` directory:
```bash
python main.py
```

The script automatically detects the latest file by parsing the quarter and year from filenames (e.g., Q226 = Q2 2026).

**Note:** PDF files should be placed in the `data/` directory.

## Testing

```bash
uv run pytest
```

## Formatting

```bash
black .
```

## Quarterly Revenue Trend

![Nvidia Revenue Trend](nvidia-revenue-trend.png)
