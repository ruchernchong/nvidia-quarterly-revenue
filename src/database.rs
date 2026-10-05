//! SQLite storage compatible with the database and exports produced by Python.

use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::Path;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use rusqlite::{Connection, Row, params};
use serde::{Deserialize, Serialize};

use crate::RevenueRow;

const COLUMNS: &str = "quarter, fiscal_year, quarter_number, data_center, gaming, \
    professional_visualization, automotive, oem_other, total_revenue, imported_at, source_pdf";

const CSV_COLUMNS: [&str; 11] = [
    "quarter",
    "fiscal_year",
    "quarter_number",
    "data_center",
    "gaming",
    "professional_visualization",
    "automotive",
    "oem_other",
    "total_revenue",
    "imported_at",
    "source_pdf",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredRow {
    #[serde(flatten)]
    pub revenue: RevenueRow,
    pub imported_at: String,
    pub source_pdf: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportMetadata {
    pub pdf_filename: String,
    pub imported_at: String,
    pub quarters_count: usize,
    pub fiscal_year_max: i32,
}

pub struct Database {
    connection: Connection,
}

impl Database {
    pub fn open(path: &Path) -> Result<Self> {
        create_parent(path)?;
        let connection = Connection::open(path)
            .with_context(|| format!("opening database {}", path.display()))?;
        connection.busy_timeout(Duration::from_secs(5))?;
        connection.execute_batch(
            "CREATE TABLE IF NOT EXISTS quarterly_revenue (
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
            );
            CREATE TABLE IF NOT EXISTS import_metadata (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                pdf_filename TEXT UNIQUE NOT NULL,
                imported_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                quarters_count INTEGER NOT NULL,
                fiscal_year_max INTEGER NOT NULL
            );",
        )?;
        Ok(Self { connection })
    }

    /// Import one PDF atomically. Reimporting replaces its quarter values and metadata
    /// without creating duplicate quarters or changing existing record IDs.
    pub fn insert_quarterly_data(
        &mut self,
        rows: &[RevenueRow],
        pdf_filename: &str,
    ) -> Result<usize> {
        let Some(fiscal_year_max) = rows.iter().map(|row| row.fiscal_year).max() else {
            bail!("cannot import an empty revenue table from {pdf_filename}");
        };
        let quarters_count = i64::try_from(rows.len()).context("too many quarters to import")?;
        let transaction = self.connection.transaction()?;
        {
            let mut statement = transaction.prepare(
                "INSERT INTO quarterly_revenue
                    (quarter, fiscal_year, quarter_number, data_center, gaming,
                     professional_visualization, automotive, oem_other, total_revenue,
                     source_pdf)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
                 ON CONFLICT(quarter, fiscal_year, quarter_number) DO UPDATE SET
                     data_center = excluded.data_center,
                     gaming = excluded.gaming,
                     professional_visualization = excluded.professional_visualization,
                     automotive = excluded.automotive,
                     oem_other = excluded.oem_other,
                     total_revenue = excluded.total_revenue,
                     source_pdf = excluded.source_pdf,
                     imported_at = CURRENT_TIMESTAMP",
            )?;
            for row in rows {
                row.validate().with_context(|| {
                    format!("invalid revenue row {} in {pdf_filename}", row.quarter)
                })?;
                statement.execute(params![
                    row.quarter,
                    row.fiscal_year,
                    row.quarter_number,
                    row.data_center,
                    row.gaming,
                    row.professional_visualization,
                    row.automotive,
                    row.oem_other,
                    row.total_revenue,
                    pdf_filename,
                ])?;
            }
        }
        transaction.execute(
            "INSERT INTO import_metadata (pdf_filename, quarters_count, fiscal_year_max)
             VALUES (?1, ?2, ?3)
             ON CONFLICT(pdf_filename) DO UPDATE SET
                 quarters_count = excluded.quarters_count,
                 fiscal_year_max = excluded.fiscal_year_max,
                 imported_at = CURRENT_TIMESTAMP",
            params![pdf_filename, quarters_count, fiscal_year_max],
        )?;
        transaction.commit()?;
        Ok(rows.len())
    }

    pub fn get_all_quarters(&self) -> Result<Vec<StoredRow>> {
        self.query_quarters(
            &format!(
                "SELECT {COLUMNS} FROM quarterly_revenue ORDER BY fiscal_year, quarter_number"
            ),
            [],
        )
    }

    pub fn get_latest_n_quarters(&self, n: usize) -> Result<Vec<StoredRow>> {
        let limit = i64::try_from(n).context("quarter count exceeds SQLite's limit")?;
        let mut rows = self.query_quarters(
            &format!(
                "SELECT {COLUMNS} FROM quarterly_revenue \
                 ORDER BY fiscal_year DESC, quarter_number DESC LIMIT ?1"
            ),
            params![limit],
        )?;
        rows.reverse();
        Ok(rows)
    }

    pub fn get_quarters_by_date_range(
        &self,
        start_year: i32,
        start_quarter: u8,
        end_year: i32,
        end_quarter: u8,
    ) -> Result<Vec<StoredRow>> {
        if !(1..=4).contains(&start_quarter) || !(1..=4).contains(&end_quarter) {
            bail!("quarter numbers must be between 1 and 4");
        }
        if (start_year, start_quarter) > (end_year, end_quarter) {
            bail!("the starting quarter must not follow the ending quarter");
        }
        self.query_quarters(
            &format!(
                "SELECT {COLUMNS} FROM quarterly_revenue
                 WHERE (fiscal_year, quarter_number) >= (?1, ?2)
                   AND (fiscal_year, quarter_number) <= (?3, ?4)
                 ORDER BY fiscal_year, quarter_number"
            ),
            params![start_year, start_quarter, end_year, end_quarter],
        )
    }

    pub fn get_import_history(&self) -> Result<Vec<ImportMetadata>> {
        let mut statement = self.connection.prepare(
            "SELECT pdf_filename, imported_at, quarters_count, fiscal_year_max
             FROM import_metadata ORDER BY imported_at DESC, id DESC",
        )?;
        let rows = statement.query_map([], |row| {
            Ok(ImportMetadata {
                pdf_filename: row.get(0)?,
                imported_at: row.get(1)?,
                quarters_count: usize::try_from(row.get::<_, i64>(2)?).map_err(|error| {
                    rusqlite::Error::FromSqlConversionFailure(
                        2,
                        rusqlite::types::Type::Integer,
                        Box::new(error),
                    )
                })?,
                fiscal_year_max: row.get(3)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn export_to_csv(&self, output_path: &Path) -> Result<()> {
        let rows = self.get_all_quarters()?;
        create_parent(output_path)?;
        let mut writer = csv::WriterBuilder::new()
            .has_headers(false)
            .terminator(csv::Terminator::CRLF)
            .from_path(output_path)
            .with_context(|| format!("creating CSV export {}", output_path.display()))?;
        // Python emits an empty file when there are no rows; keep that behavior.
        if !rows.is_empty() {
            writer.write_record(CSV_COLUMNS)?;
        }
        for row in rows {
            let revenue = &row.revenue;
            writer.serialize((
                &revenue.quarter,
                revenue.fiscal_year,
                revenue.quarter_number,
                revenue.data_center,
                revenue.gaming,
                revenue.professional_visualization,
                revenue.automotive,
                revenue.oem_other,
                revenue.total_revenue,
                &row.imported_at,
                &row.source_pdf,
            ))?;
        }
        writer.flush()?;
        Ok(())
    }

    pub fn export_to_json(&self, output_path: &Path) -> Result<()> {
        let rows = self.get_all_quarters()?;
        create_parent(output_path)?;
        let file = File::create(output_path)
            .with_context(|| format!("creating JSON export {}", output_path.display()))?;
        let mut writer = BufWriter::new(file);
        serde_json::to_writer_pretty(&mut writer, &rows)?;
        writer.flush()?;
        Ok(())
    }

    fn query_quarters<P: rusqlite::Params>(&self, sql: &str, params: P) -> Result<Vec<StoredRow>> {
        let mut statement = self.connection.prepare(sql)?;
        let rows = statement.query_map(params, read_quarter)?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }
}

fn read_quarter(row: &Row<'_>) -> rusqlite::Result<StoredRow> {
    Ok(StoredRow {
        revenue: RevenueRow {
            quarter: row.get(0)?,
            fiscal_year: row.get(1)?,
            quarter_number: row.get(2)?,
            data_center: row.get(3)?,
            gaming: row.get(4)?,
            professional_visualization: row.get(5)?,
            automotive: row.get(6)?,
            oem_other: row.get(7)?,
            total_revenue: row.get(8)?,
        },
        imported_at: row.get(9)?,
        source_pdf: row.get(10)?,
    })
}

fn create_parent(path: &Path) -> Result<()> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)
            .with_context(|| format!("creating directory {}", parent.display()))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_row(year: i32, quarter: u8, data_center: i64) -> RevenueRow {
        RevenueRow {
            quarter: format!("Q{quarter} FY{:02}", year % 100),
            fiscal_year: year,
            quarter_number: quarter,
            data_center,
            gaming: 2_000,
            professional_visualization: 500,
            automotive: 200,
            oem_other: 100,
            total_revenue: data_center + 2_800,
        }
    }

    fn test_database() -> (tempfile::TempDir, Database) {
        let directory = tempfile::tempdir().unwrap();
        let database = Database::open(&directory.path().join("nested/data.db")).unwrap();
        (directory, database)
    }

    #[test]
    fn chronological_queries_cross_year_boundaries() {
        let (_directory, mut database) = test_database();
        database
            .insert_quarterly_data(
                &[
                    sample_row(2025, 2, 15_000),
                    sample_row(2024, 4, 10_000),
                    sample_row(2025, 1, 12_000),
                ],
                "unsorted.pdf",
            )
            .unwrap();
        let labels = |rows: Vec<StoredRow>| {
            rows.into_iter()
                .map(|row| row.revenue.quarter)
                .collect::<Vec<_>>()
        };
        assert_eq!(
            labels(database.get_all_quarters().unwrap()),
            ["Q4 FY24", "Q1 FY25", "Q2 FY25"]
        );
        assert_eq!(
            labels(database.get_latest_n_quarters(2).unwrap()),
            ["Q1 FY25", "Q2 FY25"]
        );
        assert!(database.get_latest_n_quarters(0).unwrap().is_empty());
        assert_eq!(database.get_latest_n_quarters(99).unwrap().len(), 3);
        assert_eq!(
            labels(
                database
                    .get_quarters_by_date_range(2024, 4, 2025, 1)
                    .unwrap()
            ),
            ["Q4 FY24", "Q1 FY25"]
        );
        assert!(
            database
                .get_quarters_by_date_range(2025, 0, 2025, 1)
                .is_err()
        );
        assert!(
            database
                .get_quarters_by_date_range(2025, 2, 2024, 4)
                .is_err()
        );
    }

    #[test]
    fn reimport_updates_values_and_source_without_duplicates() {
        let (_directory, mut database) = test_database();
        database
            .insert_quarterly_data(&[sample_row(2024, 1, 10_000)], "first.pdf")
            .unwrap();
        let original_id: i64 = database
            .connection
            .query_row("SELECT id FROM quarterly_revenue", [], |row| row.get(0))
            .unwrap();
        assert_eq!(
            database
                .insert_quarterly_data(&[sample_row(2024, 1, 12_000)], "second.pdf")
                .unwrap(),
            1
        );
        let rows = database.get_all_quarters().unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].revenue.data_center, 12_000);
        assert_eq!(rows[0].source_pdf.as_deref(), Some("second.pdf"));
        let current_id: i64 = database
            .connection
            .query_row("SELECT id FROM quarterly_revenue", [], |row| row.get(0))
            .unwrap();
        assert_eq!(original_id, current_id);
        database
            .insert_quarterly_data(&[sample_row(2024, 1, 12_000)], "second.pdf")
            .unwrap();
        assert_eq!(database.get_import_history().unwrap().len(), 2);
        assert_eq!(database.get_all_quarters().unwrap().len(), 1);
    }

    #[test]
    fn metadata_uses_maximum_year_even_when_rows_are_unsorted() {
        let (_directory, mut database) = test_database();
        database
            .insert_quarterly_data(
                &[sample_row(2025, 2, 15_000), sample_row(2024, 4, 10_000)],
                "mixed.pdf",
            )
            .unwrap();
        let history = database.get_import_history().unwrap();
        assert_eq!(history[0].fiscal_year_max, 2025);
        assert_eq!(history[0].quarters_count, 2);
        assert!(!history[0].imported_at.is_empty());
        database
            .insert_quarterly_data(&[sample_row(2024, 1, 10_000)], "mixed.pdf")
            .unwrap();
        let history = database.get_import_history().unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].quarters_count, 1);
        assert_eq!(history[0].fiscal_year_max, 2024);
    }

    #[test]
    fn failed_pdf_import_rolls_back_quarters_and_metadata() {
        let (_directory, mut database) = test_database();
        database
            .insert_quarterly_data(&[sample_row(2024, 1, 10_000)], "existing.pdf")
            .unwrap();
        // A trigger models a write failure after the first row has been upserted.
        database
            .connection
            .execute_batch(
                "CREATE TRIGGER fail_second_quarter BEFORE INSERT ON quarterly_revenue
             WHEN NEW.quarter_number = 2
             BEGIN SELECT RAISE(ABORT, 'simulated failure'); END;",
            )
            .unwrap();
        assert!(
            database
                .insert_quarterly_data(
                    &[sample_row(2024, 1, 12_000), sample_row(2024, 2, 15_000)],
                    "failed.pdf",
                )
                .is_err()
        );
        let rows = database.get_all_quarters().unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].revenue.data_center, 10_000);
        assert_eq!(rows[0].source_pdf.as_deref(), Some("existing.pdf"));
        assert_eq!(database.get_import_history().unwrap().len(), 1);
        assert!(database.insert_quarterly_data(&[], "empty.pdf").is_err());
    }

    #[test]
    fn metadata_failure_also_rolls_back_quarter_upserts() {
        let (_directory, mut database) = test_database();
        database
            .connection
            .execute_batch(
                "CREATE TRIGGER fail_metadata BEFORE INSERT ON import_metadata
             BEGIN SELECT RAISE(ABORT, 'simulated metadata failure'); END;",
            )
            .unwrap();
        assert!(
            database
                .insert_quarterly_data(&[sample_row(2024, 1, 10_000)], "failed.pdf",)
                .is_err()
        );
        assert!(database.get_all_quarters().unwrap().is_empty());
        assert!(database.get_import_history().unwrap().is_empty());
    }

    #[test]
    fn exports_keep_python_column_names_order_and_flat_json() {
        let (directory, mut database) = test_database();
        database
            .insert_quarterly_data(
                &[sample_row(2024, 2, 12_000), sample_row(2024, 1, 10_000)],
                "source, with comma.pdf",
            )
            .unwrap();
        let csv_path = directory.path().join("exports/revenue.csv");
        let json_path = directory.path().join("exports/revenue.json");
        database.export_to_csv(&csv_path).unwrap();
        database.export_to_json(&json_path).unwrap();
        let mut csv_reader = csv::Reader::from_path(csv_path).unwrap();
        assert_eq!(
            csv_reader.headers().unwrap().iter().collect::<Vec<_>>(),
            CSV_COLUMNS
        );
        let csv_records = csv_reader
            .records()
            .collect::<csv::Result<Vec<_>>>()
            .unwrap();
        assert_eq!(csv_records[0].get(0), Some("Q1 FY24"));
        assert_eq!(csv_records[0].get(3), Some("10000"));
        assert_eq!(csv_records[0].get(10), Some("source, with comma.pdf"));
        let json: serde_json::Value =
            serde_json::from_reader(File::open(&json_path).unwrap()).unwrap();
        assert_eq!(json[0]["quarter"], "Q1 FY24");
        assert_eq!(json[0]["data_center"], 10_000);
        assert_eq!(json[0]["source_pdf"], "source, with comma.pdf");
        assert!(json[0].get("revenue").is_none());
        assert_eq!(json[0].as_object().unwrap().len(), 11);
        let decoded: Vec<StoredRow> = serde_json::from_value(json).unwrap();
        assert_eq!(decoded[1].revenue.quarter, "Q2 FY24");
    }

    #[test]
    fn empty_exports_match_python() {
        let (directory, database) = test_database();
        let csv_path = directory.path().join("empty.csv");
        let json_path = directory.path().join("empty.json");
        database.export_to_csv(&csv_path).unwrap();
        database.export_to_json(&json_path).unwrap();
        assert_eq!(fs::read_to_string(csv_path).unwrap(), "");
        assert_eq!(fs::read_to_string(json_path).unwrap(), "[]");
    }

    #[test]
    fn reopens_existing_python_schema_without_losing_metadata() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("existing.db");
        {
            let database = Database::open(&path).unwrap();
            database
                .connection
                .execute(
                    "INSERT INTO quarterly_revenue
                 (quarter, fiscal_year, quarter_number, data_center, gaming,
                  professional_visualization, automotive, oem_other, total_revenue,
                  imported_at, source_pdf)
                 VALUES ('Q1 FY24', 2024, 1, 10000, 2000, 500, 200, 100, 12800,
                         '2025-11-17 10:51:00', NULL)",
                    [],
                )
                .unwrap();
        }
        let database = Database::open(&path).unwrap();
        let rows = database.get_all_quarters().unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].imported_at, "2025-11-17 10:51:00");
        assert!(rows[0].source_pdf.is_none());
    }
}
