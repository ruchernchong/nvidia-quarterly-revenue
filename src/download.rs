use anyhow::{Context, Result, bail, ensure};
use chrono::{Datelike, Local};
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
    process::Command,
};

use crate::pdf::parse_quarter_year;

pub fn fiscal_quarter(calendar_year: i32, month: u32) -> Result<(i32, u8)> {
    ensure!((1..=12).contains(&month), "invalid calendar month");
    // NVIDIA names a fiscal year for the January in which it ends.
    let year = calendar_year + i32::from(month != 1);
    let quarter = match month {
        2..=4 => 1,
        5..=7 => 2,
        8..=10 => 3,
        _ => 4,
    };
    Ok((year, quarter))
}

pub fn current_quarter() -> Result<(i32, u8)> {
    let now = Local::now();
    fiscal_quarter(now.year(), now.month())
}

pub fn next_quarter(year: i32, quarter: u8) -> (i32, u8) {
    if quarter == 4 {
        (year + 1, 1)
    } else {
        (year, quarter + 1)
    }
}

pub fn previous_quarter(year: i32, quarter: u8) -> (i32, u8) {
    if quarter == 1 {
        (year - 1, 4)
    } else {
        (year, quarter - 1)
    }
}

pub fn quarter_string(year: i32, quarter: u8) -> Result<String> {
    ensure!(
        (2000..=2099).contains(&year) && (1..=4).contains(&quarter),
        "expected year 2000–2099 and quarter 1–4"
    );
    Ok(format!("Q{quarter}{:02}", year % 100))
}

pub fn generate_url_patterns(year: i32, quarter: u8) -> Result<Vec<String>> {
    let q = quarter_string(year, quarter)?;
    let base = "https://s201.q4cdn.com/141608511/files/doc_financials";
    let filename = format!("Rev_by_Mkt_Qtrly_Trend_{q}.pdf");
    Ok(vec![
        format!("{base}/{year}/{q}/{filename}"),
        format!("{base}/{year}/{q}/{q}-NVDA-Quarterly-Revenue-Trend.pdf"),
        format!("{base}/{year}/{filename}"),
        format!("{base}/{year}/Q{quarter}FY{:02}/{filename}", year % 100),
        format!("{base}/Q{quarter}FY{:02}/{filename}", year % 100),
    ])
}

pub fn normalized_filename(url_or_name: &str) -> String {
    let name = url_or_name.rsplit('/').next().unwrap_or(url_or_name);
    match parse_quarter_year(name) {
        Some((year, quarter)) => format!("Rev_by_Mkt_Qtrly_Trend_Q{quarter}{:02}.pdf", year % 100),
        None => name.to_owned(),
    }
}

fn validate_pdf(path: &Path) -> Result<()> {
    let mut magic = [0; 5];
    fs::File::open(path)?
        .read_exact(&mut magic)
        .context("downloaded file is too short")?;
    ensure!(&magic == b"%PDF-", "downloaded content is not a PDF");
    Ok(())
}

/// Download a specific fiscal quarter. None means already present or unpublished.
pub fn download_quarter(year: i32, quarter: u8, directory: &Path) -> Result<Option<PathBuf>> {
    let q = quarter_string(year, quarter)?;
    fs::create_dir_all(directory)?;
    let destination = directory.join(format!("Rev_by_Mkt_Qtrly_Trend_{q}.pdf"));
    let alternate = directory.join(format!("{q}-NVDA-Quarterly-Revenue-Trend.pdf"));
    for existing in [&destination, &alternate] {
        if existing.exists() {
            validate_pdf(existing)
                .with_context(|| format!("checking existing {}", existing.display()))?;
            println!("Already exists: {}", existing.display());
            return Ok(None);
        }
    }
    let mut network_errors = Vec::new();
    for url in generate_url_patterns(year, quarter)? {
        let temporary = tempfile::NamedTempFile::new_in(directory)?;
        let output = Command::new("curl")
            .args([
                "--disable",
                "--fail",
                "--location",
                "--silent",
                "--show-error",
                "--connect-timeout",
                "10",
                "--max-time",
                "60",
                "--proto",
                "=https",
                "--proto-redir",
                "=https",
                "--write-out",
                "%{http_code}",
                "--output",
            ])
            .arg(temporary.path())
            .arg(&url)
            .output()
            .context("running curl; install curl for PDF downloads")?;
        let status = String::from_utf8_lossy(&output.stdout);
        if !output.status.success() {
            // Missing CDN paths can return 403 as well as 404.
            if !matches!(status.trim(), "403" | "404") {
                network_errors.push(format!(
                    "{url}: {}",
                    String::from_utf8_lossy(&output.stderr).trim()
                ));
            }
            continue;
        }
        if status.trim() != "200" {
            network_errors.push(format!("{url}: unexpected HTTP {}", status.trim()));
            continue;
        }
        if let Err(error) = validate_pdf(temporary.path()) {
            network_errors.push(format!("{url}: {error:#}"));
            continue;
        }
        temporary
            .persist_noclobber(&destination)
            .with_context(|| format!("saving {}", destination.display()))?;
        println!("Downloaded: {}", destination.display());
        return Ok(Some(destination));
    }
    if !network_errors.is_empty() {
        bail!("could not check {q}: {}", network_errors.join("; "));
    }
    println!("Not available: {q}");
    Ok(None)
}

pub fn download_latest(directory: &Path, previous: u8, future: u8) -> Result<Vec<PathBuf>> {
    ensure!(
        previous <= 20 && future <= 20,
        "check counts must be at most 20"
    );
    let (mut year, mut quarter) = current_quarter()?;
    println!("Current fiscal quarter: Q{quarter} FY{year}");
    for _ in 0..previous {
        (year, quarter) = previous_quarter(year, quarter);
    }
    let mut downloaded = Vec::new();
    let mut errors = Vec::new();
    for _ in 0..(u16::from(previous) + 1 + u16::from(future)) {
        match download_quarter(year, quarter, directory) {
            Ok(Some(file)) => downloaded.push(file),
            Ok(None) => {}
            Err(error) => errors.push(error.to_string()),
        }
        (year, quarter) = next_quarter(year, quarter);
    }
    ensure!(
        errors.is_empty(),
        "download checks failed: {}",
        errors.join("; ")
    );
    Ok(downloaded)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fiscal_calendar_handles_january_and_february_rollover() {
        assert_eq!(fiscal_quarter(2026, 1).unwrap(), (2026, 4));
        assert_eq!(fiscal_quarter(2026, 2).unwrap(), (2027, 1));
        assert_eq!(fiscal_quarter(2026, 7).unwrap(), (2027, 2));
        assert_eq!(fiscal_quarter(2026, 10).unwrap(), (2027, 3));
        assert_eq!(fiscal_quarter(2026, 12).unwrap(), (2027, 4));
        assert_eq!(next_quarter(2026, 4), (2027, 1));
        assert_eq!(previous_quarter(2027, 1), (2026, 4));
        assert!(fiscal_quarter(2026, 0).is_err());
    }

    #[test]
    fn preserves_all_five_url_patterns_and_normalisation() {
        let urls = generate_url_patterns(2026, 1).unwrap();
        assert_eq!(urls.len(), 5);
        assert!(urls[0].ends_with("2026/Q126/Rev_by_Mkt_Qtrly_Trend_Q126.pdf"));
        assert!(urls[1].ends_with("2026/Q126/Q126-NVDA-Quarterly-Revenue-Trend.pdf"));
        assert_eq!(
            normalized_filename(&urls[1]),
            "Rev_by_Mkt_Qtrly_Trend_Q126.pdf"
        );
        assert!(generate_url_patterns(2026, 0).is_err());
        assert!(generate_url_patterns(1999, 1).is_err());
    }

    #[test]
    fn existing_files_are_never_overwritten_and_invalid_files_are_errors() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Rev_by_Mkt_Qtrly_Trend_Q126.pdf");
        fs::write(&path, b"%PDF-fixture").unwrap();
        assert!(download_quarter(2026, 1, dir.path()).unwrap().is_none());
        assert_eq!(fs::read(&path).unwrap(), b"%PDF-fixture");
        fs::write(&path, b"<html>bad download</html>").unwrap();
        assert!(download_quarter(2026, 1, dir.path()).is_err());
    }
}
