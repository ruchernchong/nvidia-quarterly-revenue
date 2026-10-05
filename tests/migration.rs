use nvidia_quarterly_revenue::{RevenueRow, database::Database, pdf};
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
    process::Command,
};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn cli() -> Command {
    Command::new(env!("CARGO_BIN_EXE_nvidia-revenue"))
}

#[test]
fn every_pdf_matches_existing_python_export_and_latest_report() {
    let mut reader = csv::Reader::from_path(root().join("data/revenue_export.csv")).unwrap();
    let existing: HashMap<_, _> = reader
        .deserialize::<RevenueRow>()
        .map(|row| {
            let row = row.unwrap();
            (row.quarter.clone(), row)
        })
        .collect();
    let files = pdf::pdf_files(&root().join("data")).unwrap();
    assert!(files.len() >= 9);
    let mut overlap = 0;
    for path in files {
        let rows = pdf::extract_data_from_pdf(&path).unwrap();
        assert_eq!(rows.len(), 8, "{}", path.display());
        for row in rows {
            if let Some(expected) = existing.get(&row.quarter) {
                assert_eq!(&row, expected);
                overlap += 1;
            }
        }
    }
    assert!(overlap >= 71);
    // Keep this known report as a fixed extraction example when new PDFs arrive.
    let path = root().join("data/Rev_by_Mkt_Qtrly_Trend_Q326.pdf");
    let rows = pdf::extract_data_from_pdf(&path).unwrap();
    assert_eq!(rows.last().unwrap().quarter, "Q3 FY26");
    assert_eq!(
        rows.last().unwrap().integer_values(),
        [51215, 4265, 760, 592, 174, 57006]
    );
}

#[test]
fn existing_database_exports_match_python_without_migrating_the_schema() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("copy.db");
    fs::copy(root().join("data/data.db"), &db_path).unwrap();
    let database = Database::open(&db_path).unwrap();
    let expected_count = csv::Reader::from_path(root().join("data/revenue_export.csv"))
        .unwrap()
        .records()
        .count();
    assert_eq!(database.get_all_quarters().unwrap().len(), expected_count);
    database.export_to_csv(&dir.path().join("out.csv")).unwrap();
    database
        .export_to_json(&dir.path().join("out.json"))
        .unwrap();
    assert_eq!(
        fs::read(dir.path().join("out.csv")).unwrap(),
        fs::read(root().join("data/revenue_export.csv")).unwrap()
    );
    let actual: serde_json::Value =
        serde_json::from_slice(&fs::read(dir.path().join("out.json")).unwrap()).unwrap();
    let expected: serde_json::Value =
        serde_json::from_slice(&fs::read(root().join("data/revenue_export.json")).unwrap())
            .unwrap();
    assert_eq!(actual, expected);
}

#[test]
fn batch_cli_imports_all_pdfs_and_is_idempotent() {
    let files = pdf::pdf_files(&root().join("data")).unwrap();
    let expected_quarters: HashSet<_> = files
        .iter()
        .flat_map(|path| pdf::extract_data_from_pdf(path).unwrap())
        .map(|row| row.quarter)
        .collect();
    let latest = pdf::get_latest_pdf(&root().join("data")).unwrap();
    let latest_total = pdf::extract_data_from_pdf(&latest)
        .unwrap()
        .last()
        .unwrap()
        .total_revenue;
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("nested/history.db");
    let json_path = dir.path().join("exports/revenue.json");
    let csv_path = dir.path().join("exports/revenue.csv");
    for _ in 0..2 {
        let output = cli()
            .args(["batch", "--import", "--data-dir"])
            .arg(root().join("data"))
            .arg("--db")
            .arg(&db_path)
            .arg("--json")
            .arg(&json_path)
            .arg("--csv")
            .arg(&csv_path)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let database = Database::open(&db_path).unwrap();
        let rows = database.get_all_quarters().unwrap();
        assert_eq!(rows.len(), expected_quarters.len());
        assert_eq!(rows.last().unwrap().revenue.total_revenue, latest_total);
        assert_eq!(database.get_import_history().unwrap().len(), files.len());
    }
    let exported: Vec<RevenueRow> = csv::Reader::from_path(csv_path)
        .unwrap()
        .deserialize()
        .map(Result::unwrap)
        .collect();
    assert_eq!(exported.len(), expected_quarters.len());
}

#[test]
fn cli_rejects_bad_inputs_without_creating_charts_or_database() {
    let dir = tempfile::tempdir().unwrap();
    let output_dir = dir.path().join("charts");
    let output = cli()
        .arg(dir.path().join("missing.pdf"))
        .arg("--output-dir")
        .arg(&output_dir)
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(!output_dir.exists());
    let db_path = dir.path().join("should-not-exist.db");
    let output = cli()
        .args(["batch", "--db"])
        .arg(&db_path)
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(!db_path.exists());
    let bad = dir.path().join("bad.pdf");
    fs::write(&bad, b"<html>Error</html>").unwrap();
    assert!(pdf::extract_data_from_pdf(&bad).is_err());
    let missing = Path::new("missing-dir-never-created-for-this-test");
    assert!(pdf::get_latest_pdf(missing).is_err());
}
