"""Tests for database operations."""

import json
import tempfile
from pathlib import Path

import pytest

from src.database import Database


@pytest.fixture
def temp_db():
    """Create a temporary database for testing."""
    with tempfile.NamedTemporaryFile(suffix=".db", delete=False) as f:
        db_path = f.name

    yield db_path

    # Clean up
    Path(db_path).unlink(missing_ok=True)


@pytest.fixture
def sample_data():
    """Sample quarterly revenue data for testing."""
    return {
        "quarters": ["Q1 FY24", "Q2 FY24", "Q3 FY24"],
        "data_center": [10000, 12000, 15000],
        "gaming": [2000, 2200, 2500],
        "professional_visualization": [500, 550, 600],
        "auto": [200, 250, 300],
        "oem_other": [100, 120, 150],
        "total": [12800, 15120, 18550],
    }


def test_database_initialization(temp_db):
    """Test database initialisation and table creation."""
    with Database(temp_db) as db:
        cursor = db.conn.cursor()

        # Check that tables were created
        cursor.execute(
            "SELECT name FROM sqlite_master WHERE type='table' AND name='quarterly_revenue'"
        )
        assert cursor.fetchone() is not None

        cursor.execute(
            "SELECT name FROM sqlite_master WHERE type='table' AND name='import_metadata'"
        )
        assert cursor.fetchone() is not None


def test_insert_quarterly_data(temp_db, sample_data):
    """Test inserting quarterly revenue data."""
    with Database(temp_db) as db:
        rows_inserted = db.insert_quarterly_data(sample_data, "test.pdf")

        # Should insert 3 quarters
        assert rows_inserted == 3

        # Verify data was inserted
        quarters = db.get_all_quarters()
        assert len(quarters) == 3
        assert quarters[0]["quarter"] == "Q1 FY24"
        assert quarters[0]["data_center"] == 10000
        assert quarters[0]["total_revenue"] == 12800


def test_insert_duplicate_quarters(temp_db, sample_data):
    """Test that duplicate quarters are replaced, not duplicated."""
    with Database(temp_db) as db:
        # Insert same data twice
        db.insert_quarterly_data(sample_data, "test1.pdf")
        db.insert_quarterly_data(sample_data, "test2.pdf")

        # Should still only have 3 quarters (duplicates replaced)
        quarters = db.get_all_quarters()
        assert len(quarters) == 3


def test_get_all_quarters_ordered(temp_db):
    """Test retrieving all quarters in chronological order."""
    data_q1 = {
        "quarters": ["Q1 FY24"],
        "data_center": [10000],
        "gaming": [2000],
        "professional_visualization": [500],
        "auto": [200],
        "oem_other": [100],
        "total": [12800],
    }
    data_q2 = {
        "quarters": ["Q2 FY24"],
        "data_center": [12000],
        "gaming": [2200],
        "professional_visualization": [550],
        "auto": [250],
        "oem_other": [120],
        "total": [15120],
    }

    with Database(temp_db) as db:
        # Insert in reverse order
        db.insert_quarterly_data(data_q2, "test2.pdf")
        db.insert_quarterly_data(data_q1, "test1.pdf")

        # Should be returned in chronological order
        quarters = db.get_all_quarters()
        assert len(quarters) == 2
        assert quarters[0]["quarter"] == "Q1 FY24"
        assert quarters[1]["quarter"] == "Q2 FY24"


def test_get_latest_n_quarters(temp_db, sample_data):
    """Test retrieving the latest N quarters."""
    with Database(temp_db) as db:
        db.insert_quarterly_data(sample_data, "test.pdf")

        # Get latest 2 quarters
        latest = db.get_latest_n_quarters(2)
        assert len(latest) == 2
        assert latest[0]["quarter"] == "Q2 FY24"
        assert latest[1]["quarter"] == "Q3 FY24"


def test_get_quarters_by_date_range(temp_db, sample_data):
    """Test retrieving quarters within a date range."""
    with Database(temp_db) as db:
        db.insert_quarterly_data(sample_data, "test.pdf")

        # Get Q2 FY24 to Q3 FY24
        quarters = db.get_quarters_by_date_range(2024, 2, 2024, 3)
        assert len(quarters) == 2
        assert quarters[0]["quarter"] == "Q2 FY24"
        assert quarters[1]["quarter"] == "Q3 FY24"


def test_export_to_csv(temp_db, sample_data):
    """Test CSV export functionality."""
    with tempfile.NamedTemporaryFile(suffix=".csv", delete=False, mode="w") as f:
        csv_path = f.name

    try:
        with Database(temp_db) as db:
            db.insert_quarterly_data(sample_data, "test.pdf")
            db.export_to_csv(csv_path)

        # Verify CSV file was created and contains data
        with open(csv_path) as f:
            content = f.read()
            assert "quarter,fiscal_year,quarter_number" in content
            assert "Q1 FY24" in content
            assert "Q2 FY24" in content
            assert "Q3 FY24" in content

    finally:
        Path(csv_path).unlink(missing_ok=True)


def test_export_to_json(temp_db, sample_data):
    """Test JSON export functionality."""
    with tempfile.NamedTemporaryFile(suffix=".json", delete=False, mode="w") as f:
        json_path = f.name

    try:
        with Database(temp_db) as db:
            db.insert_quarterly_data(sample_data, "test.pdf")
            db.export_to_json(json_path)

        # Verify JSON file was created and contains valid data
        with open(json_path) as f:
            data = json.load(f)
            assert len(data) == 3
            assert data[0]["quarter"] == "Q1 FY24"
            assert data[0]["data_center"] == 10000

    finally:
        Path(json_path).unlink(missing_ok=True)


def test_get_import_history(temp_db, sample_data):
    """Test retrieving import history."""
    with Database(temp_db) as db:
        db.insert_quarterly_data(sample_data, "test1.pdf")
        db.insert_quarterly_data(sample_data, "test2.pdf")

        history = db.get_import_history()
        assert len(history) == 2
        assert history[0]["pdf_filename"] in ["test1.pdf", "test2.pdf"]
        assert history[0]["quarters_count"] == 3


def test_context_manager(temp_db):
    """Test database context manager properly closes connection."""
    with Database(temp_db) as db:
        assert db.conn is not None

    # Connection should be closed after exiting context
    # Attempting to execute on closed connection should fail
    with pytest.raises(Exception):
        db.conn.execute("SELECT 1")
