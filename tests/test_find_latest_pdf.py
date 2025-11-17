"""Tests for find_latest_pdf utility functions."""

import pytest
from utils.find_latest_pdf import parse_quarter_year


def test_parse_quarter_year_q226():
    """Test parsing Q226 (Q2 2026)."""
    year, quarter = parse_quarter_year("Rev_by_Mkt_Qtrly_Trend_Q226.pdf")
    assert year == 2026
    assert quarter == 2


def test_parse_quarter_year_q425():
    """Test parsing Q425 (Q4 2025)."""
    year, quarter = parse_quarter_year("Rev_by_Mkt_Qtrly_Trend_Q425.pdf")
    assert year == 2025
    assert quarter == 4


def test_parse_quarter_year_q124():
    """Test parsing Q124 (Q1 2024)."""
    year, quarter = parse_quarter_year("Q124")
    assert year == 2024
    assert quarter == 1


def test_parse_quarter_year_no_match():
    """Test parsing filename with no quarter pattern."""
    year, quarter = parse_quarter_year("invalid_file.pdf")
    assert year == 0
    assert quarter == 0


def test_quarter_comparison():
    """Test that quarter parsing allows correct chronological comparison."""
    q425 = parse_quarter_year("Rev_by_Mkt_Qtrly_Trend_Q425.pdf")
    q226 = parse_quarter_year("Rev_by_Mkt_Qtrly_Trend_Q226.pdf")
    q124 = parse_quarter_year("Rev_by_Mkt_Qtrly_Trend_Q124.pdf")

    # Q226 (2026 Q2) should be latest
    assert q226 > q425
    assert q226 > q124
    assert q425 > q124
