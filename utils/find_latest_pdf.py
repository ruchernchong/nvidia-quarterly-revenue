"""Utility functions to find the latest NVIDIA quarterly revenue PDF file."""

import re
from pathlib import Path


def parse_quarter_year(filename: str) -> tuple[int, int]:
    """
    Extract quarter and year from filename.

    Args:
        filename: Filename like 'Rev_by_Mkt_Qtrly_Trend_Q226.pdf'

    Returns:
        Tuple of (year, quarter) e.g., (2026, 2) for Q226
        Returns (0, 0) if no match found
    """
    match = re.search(r"Q(\d)(\d{2})", filename)
    if match:
        quarter, year = int(match.group(1)), int(match.group(2))
        # Convert 2-digit year to 4-digit (26 -> 2026, 25 -> 2025)
        full_year = 2000 + year
        return (full_year, quarter)
    return (0, 0)


def get_latest_pdf(directory: str = "data") -> str:
    """
    Find the PDF file with the latest quarter/year.

    Args:
        directory: Directory to search for PDF files (default: data)

    Returns:
        Path to the latest PDF file as a string

    Raises:
        FileNotFoundError: If no PDF files are found
    """
    pdf_files = list(Path(directory).glob("Rev_by_Mkt_Qtrly_Trend_Q*.pdf"))
    if not pdf_files:
        raise FileNotFoundError(
            "No NVIDIA quarterly revenue PDF files found in the directory"
        )

    latest_pdf = max(pdf_files, key=lambda p: parse_quarter_year(p.name))
    return str(latest_pdf)
