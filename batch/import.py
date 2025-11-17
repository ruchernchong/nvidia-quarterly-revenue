"""Batch import all PDFs from the data directory into the database."""

import argparse
import sys
from pathlib import Path

# Add parent directory to path to import src module
sys.path.insert(0, str(Path(__file__).parent.parent))

from src import read_pdf
from src.database import Database


def batch_import_pdfs(data_dir: str = "data", db_path: str = "data/data.db") -> None:
    """
    Import all PDF files from the data directory into the database.

    Args:
        data_dir: Directory containing PDF files
        db_path: Path to the SQLite database
    """
    data_path = Path(data_dir)
    pdf_files = sorted(data_path.glob("*.pdf"))

    if not pdf_files:
        print(f"No PDF files found in {data_dir}")
        return

    print(f"Found {len(pdf_files)} PDF file(s) to process")

    with Database(db_path) as db:
        total_inserted = 0

        for pdf_file in pdf_files:
            print(f"\nProcessing: {pdf_file.name}")

            try:
                # Extract data from PDF
                data = read_pdf.extract_data_from_pdf(str(pdf_file))

                if not data:
                    print(f"  ⚠ No data extracted from {pdf_file.name}")
                    continue

                # Insert into database
                rows_inserted = db.insert_quarterly_data(data, pdf_file.name)
                total_inserted += rows_inserted

                quarters_count = len(data.get("quarters", []))
                print(
                    f"  ✓ Imported {rows_inserted} quarter(s) (total in PDF: {quarters_count})"
                )

            except Exception as e:
                print(f"  ✗ Error processing {pdf_file.name}: {e}")
                continue

        print(f"\n{'='*60}")
        print(f"Batch import complete!")
        print(f"Total quarters inserted/updated: {total_inserted}")
        print(f"\nImport history:")

        # Display import history
        history = db.get_import_history()
        for record in history:
            print(
                f"  • {record['pdf_filename']} - {record['quarters_count']} quarters "
                f"(imported: {record['imported_at']})"
            )


def export_data(
    db_path: str = "data/data.db",
    csv_output: str = None,
    json_output: str = None,
) -> None:
    """
    Export database contents to CSV and/or JSON.

    Args:
        db_path: Path to the SQLite database
        csv_output: Path to output CSV file
        json_output: Path to output JSON file
    """
    with Database(db_path) as db:
        if csv_output:
            db.export_to_csv(csv_output)
            print(f"✓ Exported to CSV: {csv_output}")

        if json_output:
            db.export_to_json(json_output)
            print(f"✓ Exported to JSON: {json_output}")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(
        description="Batch import NVIDIA revenue PDFs and export data"
    )
    parser.add_argument(
        "--import",
        dest="do_import",
        action="store_true",
        help="Import all PDFs from data directory",
    )
    parser.add_argument(
        "--csv", type=str, help="Export database to CSV file", metavar="FILE"
    )
    parser.add_argument(
        "--json", type=str, help="Export database to JSON file", metavar="FILE"
    )
    parser.add_argument(
        "--data-dir",
        type=str,
        default="data",
        help="Directory containing PDF files (default: data)",
    )
    parser.add_argument(
        "--db",
        type=str,
        default="data/data.db",
        help="Path to SQLite database (default: data/data.db)",
    )

    args = parser.parse_args()

    # If no arguments provided, show help
    if not any([args.do_import, args.csv, args.json]):
        parser.print_help()
    else:
        # Perform import if requested
        if args.do_import:
            batch_import_pdfs(args.data_dir, args.db)

        # Perform exports if requested
        if args.csv or args.json:
            export_data(args.db, args.csv, args.json)
