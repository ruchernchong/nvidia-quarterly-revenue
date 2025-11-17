"""Database operations for storing and retrieving NVIDIA quarterly revenue data."""

import json
import sqlite3
from pathlib import Path
from typing import Any


class Database:
    """Manages SQLite database operations for quarterly revenue data."""

    def __init__(self, db_path: str = "data/data.db"):
        """
        Initialise the database connection.

        Args:
            db_path: Path to the SQLite database file
        """
        self.db_path = db_path
        # Ensure data directory exists
        Path(db_path).parent.mkdir(parents=True, exist_ok=True)
        self.conn = sqlite3.connect(db_path)
        self.conn.row_factory = sqlite3.Row
        self._create_tables()

    def _create_tables(self) -> None:
        """Create database tables if they don't exist."""
        cursor = self.conn.cursor()

        # Table for quarterly revenue data
        cursor.execute(
            """
            CREATE TABLE IF NOT EXISTS quarterly_revenue (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                quarter TEXT NOT NULL,
                fiscal_year INTEGER NOT NULL,
                quarter_number INTEGER NOT NULL,
                data_center INTEGER NOT NULL,
                gaming INTEGER NOT NULL,
                professional_visualization INTEGER NOT NULL,
                automotive INTEGER NOT NULL,
                oem_other INTEGER NOT NULL,
                total_revenue INTEGER NOT NULL,
                imported_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                source_pdf TEXT,
                UNIQUE(quarter, fiscal_year, quarter_number)
            )
        """
        )

        # Table for import metadata
        cursor.execute(
            """
            CREATE TABLE IF NOT EXISTS import_metadata (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                pdf_filename TEXT UNIQUE NOT NULL,
                imported_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                quarters_count INTEGER NOT NULL,
                fiscal_year_max INTEGER NOT NULL
            )
        """
        )

        self.conn.commit()

    def insert_quarterly_data(self, data: dict[str, Any], pdf_filename: str) -> int:
        """
        Insert quarterly revenue data from a parsed PDF.

        Args:
            data: Dictionary containing quarters and revenue segments
            pdf_filename: Source PDF filename

        Returns:
            Number of rows inserted
        """
        cursor = self.conn.cursor()
        rows_inserted = 0

        quarters = data.get("quarters", [])
        data_center = data.get("data_center", [])
        gaming = data.get("gaming", [])
        professional_visualization = data.get("professional_visualization", [])
        auto = data.get("auto", [])
        oem_other = data.get("oem_other", [])
        total = data.get("total", [])

        for i, quarter in enumerate(quarters):
            # Parse quarter format like "Q3 FY24" to extract fiscal year and quarter number
            parts = quarter.strip().split()
            quarter_number = int(parts[0].replace("Q", ""))
            fiscal_year = 2000 + int(parts[1].replace("FY", ""))

            try:
                cursor.execute(
                    """
                    INSERT OR REPLACE INTO quarterly_revenue
                    (quarter, fiscal_year, quarter_number, data_center, gaming,
                     professional_visualization, automotive, oem_other, total_revenue,
                     source_pdf)
                    VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                """,
                    (
                        quarter,
                        fiscal_year,
                        quarter_number,
                        data_center[i],
                        gaming[i],
                        professional_visualization[i],
                        auto[i],
                        oem_other[i],
                        total[i],
                        pdf_filename,
                    ),
                )
                rows_inserted += 1
            except sqlite3.IntegrityError:
                # Quarter already exists, skip
                pass

        # Record import metadata
        cursor.execute(
            """
            INSERT OR REPLACE INTO import_metadata
            (pdf_filename, quarters_count, fiscal_year_max)
            VALUES (?, ?, ?)
        """,
            (pdf_filename, len(quarters), max(fiscal_year for q in quarters)),
        )

        self.conn.commit()
        return rows_inserted

    def get_all_quarters(self) -> list[dict[str, Any]]:
        """
        Retrieve all quarterly revenue data ordered by fiscal year and quarter.

        Returns:
            List of dictionaries containing quarterly revenue data
        """
        cursor = self.conn.cursor()
        cursor.execute(
            """
            SELECT quarter, fiscal_year, quarter_number, data_center, gaming,
                   professional_visualization, automotive, oem_other, total_revenue,
                   imported_at, source_pdf
            FROM quarterly_revenue
            ORDER BY fiscal_year ASC, quarter_number ASC
        """
        )

        return [dict(row) for row in cursor.fetchall()]

    def get_quarters_by_date_range(
        self, start_year: int, start_quarter: int, end_year: int, end_quarter: int
    ) -> list[dict[str, Any]]:
        """
        Retrieve quarterly data within a date range.

        Args:
            start_year: Starting fiscal year
            start_quarter: Starting quarter number (1-4)
            end_year: Ending fiscal year
            end_quarter: Ending quarter number (1-4)

        Returns:
            List of dictionaries containing quarterly revenue data
        """
        cursor = self.conn.cursor()
        cursor.execute(
            """
            SELECT quarter, fiscal_year, quarter_number, data_center, gaming,
                   professional_visualization, automotive, oem_other, total_revenue,
                   imported_at, source_pdf
            FROM quarterly_revenue
            WHERE (fiscal_year > ? OR (fiscal_year = ? AND quarter_number >= ?))
              AND (fiscal_year < ? OR (fiscal_year = ? AND quarter_number <= ?))
            ORDER BY fiscal_year ASC, quarter_number ASC
        """,
            (start_year, start_year, start_quarter, end_year, end_year, end_quarter),
        )

        return [dict(row) for row in cursor.fetchall()]

    def get_latest_n_quarters(self, n: int = 8) -> list[dict[str, Any]]:
        """
        Retrieve the latest N quarters.

        Args:
            n: Number of quarters to retrieve

        Returns:
            List of dictionaries containing quarterly revenue data
        """
        cursor = self.conn.cursor()
        cursor.execute(
            """
            SELECT quarter, fiscal_year, quarter_number, data_center, gaming,
                   professional_visualization, automotive, oem_other, total_revenue,
                   imported_at, source_pdf
            FROM quarterly_revenue
            ORDER BY fiscal_year DESC, quarter_number DESC
            LIMIT ?
        """,
            (n,),
        )

        # Reverse to get chronological order
        return list(reversed([dict(row) for row in cursor.fetchall()]))

    def export_to_csv(self, output_path: str) -> None:
        """
        Export all quarterly revenue data to CSV.

        Args:
            output_path: Path to output CSV file
        """
        import csv

        quarters = self.get_all_quarters()

        with open(output_path, "w", newline="") as csvfile:
            if quarters:
                fieldnames = quarters[0].keys()
                writer = csv.DictWriter(csvfile, fieldnames=fieldnames)
                writer.writeheader()
                writer.writerows(quarters)

    def export_to_json(self, output_path: str) -> None:
        """
        Export all quarterly revenue data to JSON.

        Args:
            output_path: Path to output JSON file
        """
        quarters = self.get_all_quarters()

        with open(output_path, "w") as jsonfile:
            json.dump(quarters, jsonfile, indent=2, default=str)

    def get_import_history(self) -> list[dict[str, Any]]:
        """
        Retrieve import history metadata.

        Returns:
            List of dictionaries containing import metadata
        """
        cursor = self.conn.cursor()
        cursor.execute(
            """
            SELECT pdf_filename, imported_at, quarters_count, fiscal_year_max
            FROM import_metadata
            ORDER BY imported_at DESC
        """
        )

        return [dict(row) for row in cursor.fetchall()]

    def close(self) -> None:
        """Close the database connection."""
        self.conn.close()

    def __enter__(self):
        """Context manager entry."""
        return self

    def __exit__(self, exc_type, exc_val, exc_tb):
        """Context manager exit."""
        self.close()
