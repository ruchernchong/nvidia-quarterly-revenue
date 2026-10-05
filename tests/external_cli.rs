#![cfg(unix)]

//! Exercise external-tool handling through the CLI with local executable stubs.
//! Each subprocess receives its own PATH and environment; no network is used.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const CURL_STUB: &str = r#"#!/bin/sh
printf '%s\n' "$*" >> "$NVIDIA_TEST_COMMAND_LOG"
output_path=''
next_output=''
url=''
for argument in "$@"; do
    if [ "$next_output" = 'yes' ]; then
        output_path="$argument"
        next_output=''
    elif [ "$argument" = '--output' ]; then
        next_output='yes'
    fi
    url="$argument"
done
case "$NVIDIA_TEST_MODE" in
    invalid_then_valid)
        case "$url" in
            *-NVDA-Quarterly-Revenue-Trend.pdf) printf '%%PDF-valid fixture' > "$output_path" ;;
            *) printf '<html>not a PDF</html>' > "$output_path" ;;
        esac
        printf '200'
        exit 0
        ;;
    not_found)
        printf '404'
        printf 'curl: HTTP 404\n' >&2
        exit 22
        ;;
    partial_timeout)
        printf '%%PDF-partial interrupted download' > "$output_path"
        printf '000'
        printf 'curl: operation timed out\n' >&2
        exit 28
        ;;
    invalid_pdf)
        printf '<html>not a PDF</html>' > "$output_path"
        printf '200'
        exit 0
        ;;
esac
printf 'unexpected test mode\n' >&2
exit 99
"#;

const GH_STUB: &str = r#"#!/bin/sh
printf '%s\n' "$*" >> "$NVIDIA_TEST_COMMAND_LOG"
if [ "$1" != 'release' ]; then
    printf 'unexpected gh command\n' >&2
    exit 99
fi
if [ "$2" = 'view' ]; then
    case "$NVIDIA_TEST_MODE" in
        exists) printf '{"tagName":"2026.Q1"}\n'; exit 0 ;;
        missing|create_failure) printf 'release not found\n' >&2; exit 1 ;;
        auth_failure) printf 'authentication failed: please run gh auth login\n' >&2; exit 4 ;;
    esac
elif [ "$2" = 'create' ]; then
    case "$NVIDIA_TEST_MODE" in
        missing) printf 'release created locally by test stub\n'; exit 0 ;;
        create_failure) printf 'HTTP 403: release creation forbidden\n' >&2; exit 1 ;;
    esac
fi
printf 'unexpected gh invocation\n' >&2
exit 99
"#;

struct Harness {
    _directory: tempfile::TempDir,
    bin_directory: PathBuf,
    data_directory: PathBuf,
    log: PathBuf,
}

impl Harness {
    fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let bin_directory = directory.path().join("bin");
        let data_directory = directory.path().join("data");
        let log = directory.path().join("commands.log");
        fs::create_dir(&bin_directory).unwrap();
        fs::create_dir(&data_directory).unwrap();
        write_executable(&bin_directory.join("curl"), CURL_STUB);
        write_executable(&bin_directory.join("gh"), GH_STUB);
        Self {
            _directory: directory,
            bin_directory,
            data_directory,
            log,
        }
    }

    fn run(&self, arguments: &[&str], mode: &str) -> Output {
        Command::new(env!("CARGO_BIN_EXE_nvidia-revenue"))
            .args(arguments)
            .arg("--data-dir")
            .arg(&self.data_directory)
            .env("PATH", &self.bin_directory)
            .env("NVIDIA_TEST_COMMAND_LOG", &self.log)
            .env("NVIDIA_TEST_MODE", mode)
            .current_dir(self._directory.path())
            .output()
            .unwrap()
    }

    fn calls(&self) -> Vec<String> {
        fs::read_to_string(&self.log)
            .unwrap()
            .lines()
            .map(str::to_owned)
            .collect()
    }

    fn data_files(&self) -> Vec<PathBuf> {
        fs::read_dir(&self.data_directory)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect()
    }

    fn release_fixture(&self) {
        fs::write(
            self.data_directory.join("Rev_by_Mkt_Qtrly_Trend_Q126.pdf"),
            b"%PDF-fixture",
        )
        .unwrap();
    }
}

fn write_executable(path: &Path, content: &str) {
    fs::write(path, content).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
}

const DOWNLOAD: [&str; 5] = ["download", "--year", "2026", "--quarter", "1"];

#[test]
fn invalid_200_response_falls_back_and_existing_download_is_idempotent() {
    let harness = Harness::new();
    let output = harness.run(&DOWNLOAD, "invalid_then_valid");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(harness.calls().len(), 2);
    let destination = harness
        .data_directory
        .join("Rev_by_Mkt_Qtrly_Trend_Q126.pdf");
    assert_eq!(fs::read(&destination).unwrap(), b"%PDF-valid fixture");
    assert_eq!(harness.data_files(), vec![destination.clone()]);

    let second = harness.run(&DOWNLOAD, "invalid_then_valid");
    assert!(second.status.success());
    assert!(String::from_utf8_lossy(&second.stdout).contains("Downloaded 0 new file(s)"));
    assert_eq!(harness.calls().len(), 2);
    assert_eq!(fs::read(destination).unwrap(), b"%PDF-valid fixture");
}

#[test]
fn unpublished_quarter_is_success_with_no_persisted_files() {
    let harness = Harness::new();
    let output = harness.run(&DOWNLOAD, "not_found");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("Not available: Q126"));
    assert_eq!(harness.calls().len(), 5);
    assert!(harness.data_files().is_empty());
}

#[test]
fn timeout_errors_leave_no_partial_download_or_temporary_files() {
    let harness = Harness::new();
    let output = harness.run(&DOWNLOAD, "partial_timeout");
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("timed out"));
    assert_eq!(harness.calls().len(), 5);
    assert!(harness.data_files().is_empty());
}

#[test]
fn invalid_pdf_responses_fail_without_persisting_content() {
    let harness = Harness::new();
    let output = harness.run(&DOWNLOAD, "invalid_pdf");
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("not a PDF"));
    assert_eq!(harness.calls().len(), 5);
    assert!(harness.data_files().is_empty());
}

#[test]
fn existing_release_is_skipped_without_create_call() {
    let harness = Harness::new();
    harness.release_fixture();
    let output = harness.run(&["release"], "exists");
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("already exists"));
    let calls = harness.calls();
    assert_eq!(calls.len(), 1);
    assert!(calls[0].starts_with("release view 2026.Q1 "));
}

#[test]
fn missing_release_is_created_with_local_gh_stub() {
    let harness = Harness::new();
    harness.release_fixture();
    let output = harness.run(&["release"], "missing");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("Created release: 2026.Q1"));
    let calls = harness.calls();
    assert_eq!(calls.len(), 2);
    assert!(calls[0].starts_with("release view 2026.Q1 "));
    assert!(calls[1].starts_with("release create 2026.Q1 "));
    assert!(calls[1].contains("--notes-file"));
    assert!(calls[1].contains("--latest"));
}

#[test]
fn authentication_error_stops_before_release_creation() {
    let harness = Harness::new();
    harness.release_fixture();
    let output = harness.run(&["release"], "auth_failure");
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("authentication failed"));
    assert_eq!(harness.calls().len(), 1);
}

#[test]
fn release_creation_error_returns_failure() {
    let harness = Harness::new();
    harness.release_fixture();
    let output = harness.run(&["release"], "create_failure");
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("HTTP 403"));
    assert_eq!(harness.calls().len(), 2);
}

#[test]
fn batch_partial_failure_preserves_successful_import_and_exports_but_returns_failure() {
    let harness = Harness::new();
    write_executable(
        &harness.bin_directory.join("pdftotext"),
        r#"#!/bin/sh
printf '%s\n' '($ in millions) Q1 FY26
Data Center 10
Gaming 4
Professional 3
Auto 2
OEM & Other 1
TOTAL 20'
"#,
    );
    fs::write(
        harness
            .data_directory
            .join("Rev_by_Mkt_Qtrly_Trend_Q126.pdf"),
        b"%PDF-valid fixture",
    )
    .unwrap();
    fs::write(
        harness
            .data_directory
            .join("Rev_by_Mkt_Qtrly_Trend_Q226.pdf"),
        b"<html>malformed PDF</html>",
    )
    .unwrap();
    let database_path = harness._directory.path().join("history.db");
    let csv_path = harness._directory.path().join("revenue.csv");
    let json_path = harness._directory.path().join("revenue.json");
    let output = harness.run(
        &[
            "batch",
            "--import",
            "--db",
            database_path.to_str().unwrap(),
            "--csv",
            csv_path.to_str().unwrap(),
            "--json",
            json_path.to_str().unwrap(),
        ],
        "unused",
    );
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("1 PDF import(s) failed"));
    let database = nvidia_quarterly_revenue::database::Database::open(&database_path).unwrap();
    let rows = database.get_all_quarters().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].revenue.quarter, "Q1 FY26");
    assert_eq!(rows[0].revenue.total_revenue, 20);
    assert_eq!(database.get_import_history().unwrap().len(), 1);
    let csv_rows: Vec<nvidia_quarterly_revenue::RevenueRow> = csv::Reader::from_path(csv_path)
        .unwrap()
        .deserialize()
        .collect::<csv::Result<_>>()
        .unwrap();
    assert_eq!(csv_rows.len(), 1);
    let json: serde_json::Value = serde_json::from_slice(&fs::read(json_path).unwrap()).unwrap();
    assert_eq!(json.as_array().unwrap().len(), 1);
    assert_eq!(json[0]["quarter"], "Q1 FY26");
}
