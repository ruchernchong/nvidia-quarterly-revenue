"""Utility functions to download NVIDIA quarterly revenue PDF files."""

import re
import sys
from datetime import datetime
from pathlib import Path
from typing import Optional

import requests


def get_current_quarter() -> tuple[int, int]:
    """
    Get the current fiscal quarter and year based on NVIDIA's fiscal calendar.

    NVIDIA's fiscal year ends on the last Sunday in January.
    Fiscal quarters: Q1 (Feb-Apr), Q2 (May-Jul), Q3 (Aug-Oct), Q4 (Nov-Jan)

    Returns:
        Tuple of (fiscal_year, quarter)
    """
    now = datetime.now()
    month = now.month

    # Determine fiscal quarter based on calendar month
    if month in [2, 3, 4]:
        fiscal_quarter = 1
    elif month in [5, 6, 7]:
        fiscal_quarter = 2
    elif month in [8, 9, 10]:
        fiscal_quarter = 3
    else:  # month in [11, 12, 1]
        fiscal_quarter = 4

    # Fiscal year calculation
    # If we're in Q4 (Nov-Jan), the fiscal year is the following calendar year
    if fiscal_quarter == 4:
        fiscal_year = now.year + 1
    else:
        fiscal_year = now.year

    return (fiscal_year, fiscal_quarter)


def get_next_quarter(year: int, quarter: int) -> tuple[int, int]:
    """
    Get the next fiscal quarter.

    Args:
        year: Fiscal year
        quarter: Fiscal quarter (1-4)

    Returns:
        Tuple of (year, quarter) for the next quarter
    """
    if quarter == 4:
        return (year + 1, 1)
    return (year, quarter + 1)


def get_previous_quarter(year: int, quarter: int) -> tuple[int, int]:
    """
    Get the previous fiscal quarter.

    Args:
        year: Fiscal year
        quarter: Fiscal quarter (1-4)

    Returns:
        Tuple of (year, quarter) for the previous quarter
    """
    if quarter == 1:
        return (year - 1, 4)
    return (year, quarter - 1)


def format_quarter_string(year: int, quarter: int) -> str:
    """
    Format fiscal quarter and year into NVIDIA's URL format.

    Args:
        year: Full fiscal year (e.g., 2026)
        quarter: Fiscal quarter number (1-4)

    Returns:
        Formatted string like 'Q226' for fiscal Q2 2026
    """
    year_short = year % 100  # Convert 2026 to 26
    return f"Q{quarter}{year_short:02d}"


def generate_url_patterns(year: int, quarter: int) -> list[str]:
    """
    Generate all possible URL patterns for a NVIDIA quarterly revenue PDF.

    NVIDIA's URL structure is inconsistent across quarters. This function returns
    multiple URL patterns to try, ordered by likelihood (most recent patterns first).

    Args:
        year: Full fiscal year (e.g., 2026)
        quarter: Fiscal quarter number (1-4)

    Returns:
        List of possible URLs to try, ordered by likelihood
    """
    quarter_str = format_quarter_string(year, quarter)
    base_url = "https://s201.q4cdn.com/141608511/files/doc_financials"

    # Pattern 1: Current standard format (FY25 Q2-Q4, FY26 Q2)
    # Example: 2026/Q226/Rev_by_Mkt_Qtrly_Trend_Q226.pdf
    pattern1 = (
        f"{base_url}/{year}/{quarter_str}/Rev_by_Mkt_Qtrly_Trend_{quarter_str}.pdf"
    )

    # Pattern 2: Alternative filename format (FY26 Q1)
    # Example: 2026/Q126/Q126-NVDA-Quarterly-Revenue-Trend.pdf
    pattern2 = f"{base_url}/{year}/{quarter_str}/{quarter_str}-NVDA-Quarterly-Revenue-Trend.pdf"

    # Pattern 3: No quarter subdirectory (FY25 Q1)
    # Example: 2025/Rev_by_Mkt_Qtrly_Trend_Q125.pdf
    pattern3 = f"{base_url}/{year}/Rev_by_Mkt_Qtrly_Trend_{quarter_str}.pdf"

    # Pattern 4: Q#FY## subdirectory format (FY24 Q1, Q2, Q4)
    # Example: 2024/Q1FY24/Rev_by_Mkt_Qtrly_Trend_Q124.pdf
    pattern4 = f"{base_url}/{year}/Q{quarter}FY{year % 100:02d}/Rev_by_Mkt_Qtrly_Trend_{quarter_str}.pdf"

    # Pattern 5: No year directory, only Q#FY## (FY24 Q3)
    # Example: Q3FY24/Rev_by_Mkt_Qtrly_Trend_Q324.pdf
    pattern5 = f"{base_url}/Q{quarter}FY{year % 100:02d}/Rev_by_Mkt_Qtrly_Trend_{quarter_str}.pdf"

    return [pattern1, pattern2, pattern3, pattern4, pattern5]


def get_normalized_filename(url_or_filename: str) -> str:
    """
    Convert any NVIDIA PDF filename to normalized format.

    Converts both filename formats to standard: Rev_by_Mkt_Qtrly_Trend_Q###.pdf

    Args:
        url_or_filename: Either a full URL or just a filename

    Returns:
        Normalized filename in format Rev_by_Mkt_Qtrly_Trend_Q###.pdf
        Returns original if no quarter pattern is found
    """
    # Extract filename if it's a URL
    filename = url_or_filename.split("/")[-1]

    # Extract Q### pattern (e.g., Q126, Q226)
    match = re.search(r"Q(\d)(\d{2})", filename)
    if match:
        quarter_year = match.group(0)  # e.g., "Q126"
        return f"Rev_by_Mkt_Qtrly_Trend_{quarter_year}.pdf"

    return filename


def find_available_pdf(year: int, quarter: int) -> Optional[str]:
    """
    Find the first available PDF URL for a given fiscal quarter.

    Tries multiple URL patterns in order of likelihood and returns the first
    successful URL. Logs which patterns are attempted and which one succeeds.

    Args:
        year: Full fiscal year (e.g., 2026)
        quarter: Fiscal quarter number (1-4)

    Returns:
        URL of the available PDF, or None if not found with any pattern
    """
    patterns = generate_url_patterns(year, quarter)
    quarter_str = format_quarter_string(year, quarter)

    for i, url in enumerate(patterns, 1):
        try:
            response = requests.head(url, timeout=10, allow_redirects=True)
            if response.status_code == 200:
                # Extract pattern description from the URL structure
                pattern_desc = url.split("/doc_financials/")[1].split("/")[0:2]
                print(f"  ✓ Found (pattern {i}: {'/'.join(pattern_desc)}...)")
                return url
        except requests.RequestException:
            continue

    return None


def download_pdf(url: str, output_dir: str = "data") -> Optional[str]:
    """
    Download a PDF from the given URL.

    Checks for both original and normalized filenames to avoid re-downloading
    files that have been renamed to the standard format.

    Args:
        url: URL of the PDF to download
        output_dir: Directory to save the PDF (default: data)

    Returns:
        Path to the downloaded file, or None if download failed
    """
    try:
        # Extract original filename from URL
        original_filename = url.split("/")[-1]
        normalized_filename = get_normalized_filename(url)

        # Check if file exists with either name
        original_path = Path(output_dir) / original_filename
        normalized_path = Path(output_dir) / normalized_filename

        # If normalized file exists, return that
        if normalized_path.exists():
            print(f"  Already exists: {normalized_filename}")
            return str(normalized_path)

        # If original filename exists (edge case), return that
        if original_path.exists() and original_filename != normalized_filename:
            print(f"  Already exists: {original_filename}")
            return str(original_path)

        # Download the file with normalized filename
        response = requests.get(url, timeout=30, stream=True)
        response.raise_for_status()

        # Create output directory if it doesn't exist
        normalized_path.parent.mkdir(parents=True, exist_ok=True)

        # Write the file with normalized name
        with open(normalized_path, "wb") as f:
            for chunk in response.iter_content(chunk_size=8192):
                f.write(chunk)

        print(f"  ✓ Downloaded: {normalized_filename}")
        return str(normalized_path)

    except requests.RequestException as e:
        print(f"  ✗ Error downloading: {e}")
        return None


def download_latest_pdfs(
    output_dir: str = "data",
    check_previous: int = 1,
    check_future: int = 2,
) -> list[str]:
    """
    Check for and download the latest NVIDIA quarterly revenue PDFs.

    Args:
        output_dir: Directory to save PDFs (default: data)
        check_previous: Number of previous quarters to check (default: 1)
        check_future: Number of future quarters to check (default: 2)

    Returns:
        List of paths to newly downloaded PDFs
    """
    downloaded_files = []
    current_year, current_quarter = get_current_quarter()

    print(f"Current fiscal quarter: Q{current_quarter} FY{current_year}")
    print(
        f"Checking {check_previous} previous + current + {check_future} future quarters\n"
    )

    # Start from N quarters ago
    year, quarter = current_year, current_quarter
    for _ in range(check_previous):
        year, quarter = get_previous_quarter(year, quarter)

    # Check from N quarters ago to M quarters ahead
    total_checks = check_previous + 1 + check_future
    for _ in range(total_checks):
        quarter_str = format_quarter_string(year, quarter)

        print(f"Checking {quarter_str} (FY{year} Q{quarter})...")

        url = find_available_pdf(year, quarter)
        if url:
            downloaded = download_pdf(url, output_dir)
            if downloaded:
                downloaded_files.append(downloaded)
        else:
            print(f"  Not available")

        year, quarter = get_next_quarter(year, quarter)

    return downloaded_files


if __name__ == "__main__":
    output_dir = sys.argv[1] if len(sys.argv) > 1 else "data"
    print("=" * 60)
    print("NVIDIA Quarterly Revenue PDF Downloader")
    print("=" * 60)
    print()

    files = download_latest_pdfs(output_dir)

    print()
    print("=" * 60)
    if files:
        print(f"Downloaded {len(files)} new file(s):")
        for file in files:
            print(f"  • {Path(file).name}")
    else:
        print("No new PDFs downloaded.")
    print("=" * 60)
