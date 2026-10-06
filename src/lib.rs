pub mod charts;
pub mod database;
pub mod download;
pub mod pdf;
pub mod release;

use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

/// A fiscal quarter's reported revenue, in USD millions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RevenueRow {
    pub quarter: String,
    pub fiscal_year: i32,
    pub quarter_number: u8,
    pub data_center: i64,
    pub gaming: i64,
    pub professional_visualization: i64,
    pub automotive: i64,
    pub oem_other: i64,
    pub total_revenue: i64,
}

impl RevenueRow {
    pub fn new(quarter: String, values: [i64; 6]) -> Result<Self> {
        let (fiscal_year, quarter_number) = parse_quarter(&quarter)?;
        let row = Self {
            quarter,
            fiscal_year,
            quarter_number,
            data_center: values[0],
            gaming: values[1],
            professional_visualization: values[2],
            automotive: values[3],
            oem_other: values[4],
            total_revenue: values[5],
        };
        row.validate()?;
        Ok(row)
    }

    pub fn validate(&self) -> Result<()> {
        let parsed = parse_quarter(&self.quarter)?;
        ensure!(
            parsed == (self.fiscal_year, self.quarter_number),
            "inconsistent fiscal quarter: {}",
            self.quarter
        );
        let values = self.integer_values();
        ensure!(
            values.iter().all(|v| *v >= 0),
            "negative revenue: {}",
            self.quarter
        );
        let sum = values[..5]
            .iter()
            .try_fold(0_i64, |sum, v| sum.checked_add(*v));
        ensure!(
            sum == Some(self.total_revenue),
            "segment total mismatch: {}",
            self.quarter
        );
        Ok(())
    }

    pub fn integer_values(&self) -> [i64; 6] {
        [
            self.data_center,
            self.gaming,
            self.professional_visualization,
            self.automotive,
            self.oem_other,
            self.total_revenue,
        ]
    }

    pub fn values(&self) -> [f64; 6] {
        self.integer_values().map(|v| v as f64)
    }
}

pub fn parse_quarter(label: &str) -> Result<(i32, u8)> {
    let parts: Vec<_> = label.split_whitespace().collect();
    ensure!(parts.len() == 2, "invalid quarter: {label}");
    ensure!(parts[0].len() == 2, "invalid quarter: {label}");
    let q = parts[0]
        .strip_prefix('Q')
        .ok_or_else(|| anyhow::anyhow!("invalid quarter: {label}"))?
        .parse::<u8>()?;
    let year = parts[1]
        .strip_prefix("FY")
        .ok_or_else(|| anyhow::anyhow!("invalid quarter: {label}"))?;
    ensure!(
        year.len() == 2 && year.bytes().all(|c| c.is_ascii_digit()),
        "invalid fiscal year: {label}"
    );
    ensure!((1..=4).contains(&q), "invalid quarter: {label}");
    ensure!(
        label == format!("Q{q} FY{year}"),
        "expected canonical fiscal quarter: {label}"
    );
    Ok((2000 + year.parse::<i32>()?, q))
}

/// Percentage change is undefined for zero or invalid prior revenue.
pub fn growth_rate(current: f64, previous: f64) -> Option<f64> {
    if !current.is_finite() || !previous.is_finite() || current < 0.0 || previous <= 0.0 {
        return None;
    }
    let rate = (current - previous) / previous * 100.0;
    rate.is_finite().then_some(rate)
}

pub fn format_growth(rate: Option<f64>) -> String {
    rate.map_or_else(|| "N/A".into(), |rate| format!("{rate:+.2}%"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_inconsistent_or_overflowing_revenue() {
        assert!(RevenueRow::new("Q2 FY26".into(), [1, 2, 3, 4, 5, 16]).is_err());
        assert!(RevenueRow::new("Q2 FY26".into(), [i64::MAX, 1, 0, 0, 0, i64::MAX]).is_err());
        assert!(RevenueRow::new("Q0 FY26".into(), [0; 6]).is_err());
        assert!(RevenueRow::new("Q01 FY26".into(), [0; 6]).is_err());
        assert!(RevenueRow::new("Q1  FY26".into(), [0; 6]).is_err());
    }

    #[test]
    fn positive_negative_and_zero_growth() {
        assert_eq!(growth_rate(125.0, 100.0), Some(25.0));
        assert_eq!(growth_rate(75.0, 100.0), Some(-25.0));
        assert_eq!(growth_rate(100.0, 0.0), None);
        assert_eq!(growth_rate(0.0, 0.0), None);
        assert_eq!(growth_rate(100.0, 100.0), Some(0.0));
        assert_eq!(growth_rate(0.0, 100.0), Some(-100.0));
        assert_eq!(growth_rate(f64::NAN, 100.0), None);
        assert_eq!(growth_rate(100.0, f64::INFINITY), None);
        assert_eq!(format_growth(None), "N/A");
        assert_eq!(format_growth(Some(25.0)), "+25.00%");
    }
}
