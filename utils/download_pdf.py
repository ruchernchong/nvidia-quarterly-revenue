"""Utility functions to download NVIDIA quarterly revenue PDF files."""

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


def build_pdf_url(year: int, quarter: int) -> str:
    """
    Build the NVIDIA quarterly revenue PDF URL.

    Args:
        year: Full fiscal year (e.g., 2026)
        quarter: Fiscal quarter number (1-4)

    Returns:
        Full URL to the PDF
    """
    quarter_str = format_quarter_string(year, quarter)
    base_url = "https://s201.q4cdn.com/141608511/files/doc_financials"
    return f"{base_url}/{year}/{quarter_str}/Rev_by_Mkt_Qtrly_Trend_{quarter_str}.pdf"


def check_pdf_exists(url: str) -> bool:
    """
    Check if a PDF is available at the given URL.

    Args:
        url: URL to check

    Returns:
        True if the PDF exists and is accessible, False otherwise
    """
    try:
        response = requests.head(url, timeout=10, allow_redirects=True)
        return response.status_code == 200
    except requests.RequestException:
        return False


def download_pdf(url: str, output_dir: str = "data") -> Optional[str]:
    """
    Download a PDF from the given URL.

    Args:
        url: URL of the PDF to download
        output_dir: Directory to save the PDF (default: data)

    Returns:
        Path to the downloaded file, or None if download failed
    """
    try:
        # Extract filename from URL
        filename = url.split("/")[-1]
        output_path = Path(output_dir) / filename

        # Skip if file already exists
        if output_path.exists():
            print(f"  Already exists: {filename}")
            return str(output_path)

        # Download the file
        response = requests.get(url, timeout=30, stream=True)
        response.raise_for_status()

        # Create output directory if it doesn't exist
        output_path.parent.mkdir(parents=True, exist_ok=True)

        # Write the file
        with open(output_path, "wb") as f:
            for chunk in response.iter_content(chunk_size=8192):
                f.write(chunk)

        print(f"  ✓ Downloaded: {filename}")
        return str(output_path)

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
        url = build_pdf_url(year, quarter)
        quarter_str = format_quarter_string(year, quarter)

        print(f"Checking {quarter_str} (FY{year} Q{quarter})...")

        if check_pdf_exists(url):
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
