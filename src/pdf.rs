use crate::{RevenueRow, parse_quarter};
use anyhow::{Context, Result, bail, ensure};
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
    process::Command,
};

const LABELS: [&str; 6] = [
    "Data Center",
    "Gaming",
    "Professional",
    "Auto",
    "OEM & Other",
    "TOTAL",
];

/// Read the first page with Poppler, then parse the labelled revenue table.
pub fn extract_data_from_pdf(path: &Path) -> Result<Vec<RevenueRow>> {
    let mut magic = [0; 5];
    fs::File::open(path)
        .with_context(|| format!("opening {}", path.display()))?
        .read_exact(&mut magic)?;
    ensure!(&magic == b"%PDF-", "not a PDF: {}", path.display());
    let output = Command::new("pdftotext")
        .args(["-f", "1", "-l", "1", "-layout"])
        .arg(path).arg("-").output()
        .context("running pdftotext; install Poppler (brew install poppler or apt install poppler-utils)")?;
    ensure!(
        output.status.success(),
        "PDF extraction failed for {}: {}",
        path.display(),
        String::from_utf8_lossy(&output.stderr).trim()
    );
    parse_table(&String::from_utf8(output.stdout).context("pdftotext produced invalid UTF-8")?)
        .with_context(|| format!("parsing {}", path.display()))
}

pub fn parse_table(text: &str) -> Result<Vec<RevenueRow>> {
    let mut quarters = Vec::new();
    let mut segments: [Option<Vec<i64>>; 6] = std::array::from_fn(|_| None);
    let mut current = None;
    for line in text.lines() {
        let fields: Vec<_> = line.split_whitespace().collect();
        if line.contains("$ in millions") {
            ensure!(quarters.is_empty(), "duplicate revenue header");
            for pair in fields.windows(2) {
                if pair[0].starts_with('Q') && pair[1].starts_with("FY") {
                    let label = format!("{} {}", pair[0], pair[1]);
                    parse_quarter(&label)?;
                    quarters.push(label);
                }
            }
            continue;
        }
        for (index, label) in LABELS.iter().enumerate() {
            if line.trim().starts_with(label) {
                ensure!(current.is_none(), "missing revenue values before {label}");
                current = Some(index);
            }
        }
        let Some(index) = current else {
            continue;
        };
        if quarters.is_empty() || fields.len() < quarters.len() {
            continue;
        }
        let values: Result<Vec<i64>, _> = fields[fields.len() - quarters.len()..]
            .iter()
            .map(|field| field.replace(['$', ','], "").parse())
            .collect();
        if let Ok(values) = values {
            ensure!(
                !fields[..fields.len() - quarters.len()]
                    .iter()
                    .any(|field| field.replace(['$', ','], "").parse::<i64>().is_ok()),
                "extra revenue columns: {}",
                LABELS[index]
            );
            ensure!(
                segments[index].is_none(),
                "duplicate segment: {}",
                LABELS[index]
            );
            segments[index] = Some(values);
            current = None;
        }
    }
    ensure!(
        !quarters.is_empty(),
        "no quarterly revenue header on the first page"
    );
    for (index, segment) in segments.iter().enumerate() {
        ensure!(
            segment.as_ref().is_some_and(|s| s.len() == quarters.len()),
            "missing or incomplete segment: {}",
            LABELS[index]
        );
    }
    let mut rows = quarters
        .iter()
        .enumerate()
        .map(|(i, quarter)| {
            RevenueRow::new(
                quarter.clone(),
                std::array::from_fn(|j| segments[j].as_ref().expect("validated segment")[i]),
            )
        })
        .collect::<Result<Vec<_>>>()?;
    rows.sort_by_key(|r| (r.fiscal_year, r.quarter_number));
    ensure!(
        rows.windows(2).all(|w| w[0].quarter != w[1].quarter),
        "duplicate quarter"
    );
    Ok(rows)
}

pub fn parse_quarter_year(filename: &str) -> Option<(i32, u8)> {
    filename
        .as_bytes()
        .windows(4)
        .enumerate()
        .find_map(|(index, w)| {
            if w[0] == b'Q'
                && (b'1'..=b'4').contains(&w[1])
                && w[2].is_ascii_digit()
                && w[3].is_ascii_digit()
                && !filename
                    .as_bytes()
                    .get(index + 4)
                    .is_some_and(u8::is_ascii_digit)
            {
                Some((
                    2000 + i32::from(w[2] - b'0') * 10 + i32::from(w[3] - b'0'),
                    w[1] - b'0',
                ))
            } else {
                None
            }
        })
}

pub fn pdf_files(directory: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for entry in
        fs::read_dir(directory).with_context(|| format!("reading {}", directory.display()))?
    {
        let path = entry?.path();
        if path.is_file()
            && path
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("pdf"))
        {
            files.push(path);
        }
    }
    files.sort_by_key(|p| {
        (
            parse_quarter_year(&p.file_name().unwrap_or_default().to_string_lossy()),
            p.clone(),
        )
    });
    Ok(files)
}

pub fn get_latest_pdf(directory: &Path) -> Result<PathBuf> {
    let files = pdf_files(directory)?;
    let latest = files.into_iter().rfind(|path| {
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        let supported = name.starts_with("Rev_by_Mkt_Qtrly_Trend_Q")
            || (name.starts_with('Q') && name.ends_with("-NVDA-Quarterly-Revenue-Trend.pdf"));
        supported && parse_quarter_year(&name).is_some()
    });
    match latest {
        Some(path) => Ok(path),
        None => bail!(
            "no NVIDIA quarterly revenue PDFs found in {}",
            directory.display()
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table() -> &'static str {
        "($ in millions) Q2 FY26 Q1 FY26\nGaming 4 2\nData Center $10 $5\nProfessional\nVisualization\n3 1\nAuto 2 1\nOEM & Other 1 1\nTOTAL $20 $10\n"
    }

    #[test]
    fn named_segments_and_chronological_quarters() {
        let rows = parse_table(table()).unwrap();
        assert_eq!(rows[0].quarter, "Q1 FY26");
        assert_eq!(rows[1].integer_values(), [10, 4, 3, 2, 1, 20]);
    }

    #[test]
    fn malformed_tables_are_errors() {
        assert!(parse_table("").is_err());
        assert!(parse_table(&table().replace("TOTAL $20 $10", "TOTAL $21 $10")).is_err());
        assert!(parse_table(&table().replace("Auto 2 1\n", "")).is_err());
        assert!(parse_table(&table().replace("Q2 FY26", "Q1 FY26")).is_err());
        assert!(parse_table(&format!("{}Gaming 4 2\n", table())).is_err());
        assert!(parse_table(&table().replace("Gaming 4 2", "Gaming 99 4 2")).is_err());
        assert!(parse_table(&table().replace("Gaming 4 2", "Gaming 4")).is_err());
        assert!(parse_table(&table().replace("Gaming 4 2", "Gaming 4 N/A")).is_err());
        assert!(parse_table(&table().replace("Gaming 4 2", "Gaming -4 2")).is_err());
    }

    #[test]
    fn latest_uses_fiscal_year_then_quarter_and_both_naming_schemes() {
        let dir = tempfile::tempdir().unwrap();
        for name in [
            "Rev_by_Mkt_Qtrly_Trend_Q425.pdf",
            "Q126-NVDA-Quarterly-Revenue-Trend.pdf",
            "Rev_by_Mkt_Qtrly_Trend_Q999.pdf",
            "unrelated_Q226.pdf",
        ] {
            fs::write(dir.path().join(name), b"fixture").unwrap();
        }
        assert_eq!(
            get_latest_pdf(dir.path()).unwrap().file_name().unwrap(),
            "Q126-NVDA-Quarterly-Revenue-Trend.pdf"
        );
        assert!(get_latest_pdf(tempfile::tempdir().unwrap().path()).is_err());
        assert_eq!(
            parse_quarter_year("Rev_by_Mkt_Qtrly_Trend_Q326.pdf"),
            Some((2026, 3))
        );
        assert_eq!(parse_quarter_year("Rev_by_Mkt_Qtrly_Trend_Q3269.pdf"), None);
    }
}
