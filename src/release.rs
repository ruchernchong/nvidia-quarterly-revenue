use crate::pdf::{get_latest_pdf, parse_quarter_year};
use anyhow::{Context, Result, ensure};
use std::{io::Write, path::Path, process::Command};

pub fn release_tag(directory: &Path) -> Result<String> {
    let latest = get_latest_pdf(directory)?;
    let (year, quarter) =
        parse_quarter_year(&latest.file_name().unwrap_or_default().to_string_lossy())
            .context("latest PDF has no fiscal quarter")?;
    Ok(format!("{year}.Q{quarter}"))
}

pub fn create_latest_release(directory: &Path) -> Result<()> {
    let tag = release_tag(directory)?;
    let view = Command::new("gh")
        .args(["release", "view", &tag, "--json", "tagName"])
        .output()
        .context("running gh; install and authenticate GitHub CLI for releases")?;
    if view.status.success() {
        println!("Release {tag} already exists, skipping");
        return Ok(());
    }
    let error = String::from_utf8_lossy(&view.stderr);
    ensure!(
        error.to_lowercase().contains("release not found") || error.contains("HTTP 404"),
        "cannot check release {tag}: {}",
        error.trim()
    );
    let mut notes = tempfile::NamedTempFile::new()?;
    writeln!(notes, "Added NVIDIA quarterly revenue data for {tag}")?;
    let result = Command::new("gh")
        .args(["release", "create", &tag, "--title", &tag, "--notes-file"])
        .arg(notes.path())
        .arg("--latest")
        .output()
        .context("creating GitHub release")?;
    ensure!(
        result.status.success(),
        "failed to create release {tag}: {}",
        String::from_utf8_lossy(&result.stderr).trim()
    );
    println!("Created release: {tag}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tag_uses_latest_fiscal_quarter_without_network_calls() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("Rev_by_Mkt_Qtrly_Trend_Q425.pdf"),
            b"fixture",
        )
        .unwrap();
        std::fs::write(
            dir.path().join("Rev_by_Mkt_Qtrly_Trend_Q126.pdf"),
            b"fixture",
        )
        .unwrap();
        assert_eq!(release_tag(dir.path()).unwrap(), "2026.Q1");
    }
}
