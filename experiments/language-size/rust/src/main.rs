use std::{env, error::Error, fmt::Write, fs, path::Path, process::Command};

struct Row {
    quarter: String,
    values: [i64; 6],
}

fn number(s: &str) -> Result<i64, std::num::ParseIntError> {
    s.replace(['$', ','], "").parse()
}

fn read_csv(path: &str) -> Result<Vec<Row>, Box<dyn Error>> {
    let text = fs::read_to_string(path)?;
    let mut lines = text.lines();
    if !lines.next().unwrap_or("").starts_with("quarter,") {
        return Err("expected repository revenue CSV".into());
    }
    let mut rows = Vec::new();
    for line in lines {
        // The repository export has no quoted fields. Reject rather than misparse them.
        if line.contains('"') {
            return Err("quoted CSV fields are outside this prototype's scope".into());
        }
        let fields: Vec<_> = line.split(',').collect();
        if fields.len() < 9 {
            return Err("invalid revenue CSV row".into());
        }
        let mut values = [0; 6];
        for i in 0..6 {
            values[i] = number(fields[i + 3])?;
        }
        rows.push(Row {
            quarter: fields[0].into(),
            values,
        });
    }
    Ok(rows)
}

fn read_pdf(path: &str) -> Result<Vec<Row>, Box<dyn Error>> {
    let output = Command::new("pdftotext")
        .args(["-f", "1", "-l", "1", "-layout", path, "-"])
        .output()?;
    if !output.status.success() {
        return Err("pdftotext failed".into());
    }
    let text = String::from_utf8(output.stdout)?;
    let mut quarters = Vec::new();
    let mut segments: [Option<Vec<i64>>; 6] = std::array::from_fn(|_| None);
    let mut segment = None;
    let labels = [
        "Data Center",
        "Gaming",
        "Professional",
        "Auto",
        "OEM & Other",
        "TOTAL",
    ];
    for line in text.lines() {
        let fields: Vec<_> = line.split_whitespace().collect();
        if line.contains("($ in millions)") {
            for pair in fields.windows(2) {
                if pair[0].starts_with('Q') && pair[1].starts_with("FY") {
                    quarters.push(format!("{} {}", pair[0], pair[1]));
                }
            }
            continue;
        }
        for (i, label) in labels.iter().enumerate() {
            if line.trim().starts_with(label) {
                segment = Some(i);
            }
        }
        if segment.is_none() || quarters.is_empty() || fields.len() < quarters.len() {
            continue;
        }
        let values: Result<Vec<_>, _> = fields[fields.len() - quarters.len()..]
            .iter()
            .map(|s| number(s))
            .collect();
        if let Ok(values) = values {
            let index = segment.take().unwrap();
            if segments[index].is_some() {
                return Err("duplicate segment".into());
            }
            segments[index] = Some(values);
        }
    }
    if quarters.is_empty()
        || segments
            .iter()
            .any(|s| s.as_ref().is_none_or(|v| v.len() != quarters.len()))
    {
        return Err("expected six revenue rows on page one".into());
    }
    Ok((0..quarters.len())
        .rev()
        .map(|i| Row {
            quarter: quarters[i].clone(),
            values: std::array::from_fn(|j| segments[j].as_ref().unwrap()[i]),
        })
        .collect())
}

fn run() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = env::args().collect();
    if args.len() != 3 {
        return Err("usage: revenue INPUT.csv|INPUT.pdf OUTPUT_DIRECTORY".into());
    }
    let rows = if args[1].to_lowercase().ends_with(".pdf") {
        read_pdf(&args[1])?
    } else {
        read_csv(&args[1])?
    };
    for r in &rows {
        if r.values.iter().any(|n| *n < 0) || r.values[..5].iter().sum::<i64>() != r.values[5] {
            return Err(format!("invalid segment total: {}", r.quarter).into());
        }
    }
    let max_total = rows.iter().map(|r| r.values[5]).max().unwrap_or(0);
    if max_total <= 0 {
        return Err("no positive revenue".into());
    }
    fs::create_dir_all(&args[2])?;
    let mut report = String::from(
        "quarter,data_center,gaming,professional_visualization,automotive,oem_other,total_revenue,qoq_percent\n",
    );
    let mut svg = String::from(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"1200\" height=\"600\" viewBox=\"0 0 1200 600\"><rect width=\"1200\" height=\"600\" fill=\"white\"/><text x=\"60\" y=\"30\">NVIDIA revenue by market ($ millions)</text>\n",
    );
    let colours = ["#76b900", "#2563eb", "#a855f7", "#f59e0b", "#64748b"];
    let labels = [
        "Data Centre",
        "Gaming",
        "Professional Visualisation",
        "Automotive",
        "OEM &amp; Other",
    ];
    for (j, label) in labels.iter().enumerate() {
        writeln!(
            svg,
            "<text x=\"{}\" y=\"580\" fill=\"{}\">{}</text>",
            60 + j * 215,
            colours[j],
            label
        )?;
    }
    let step = 1080.0 / rows.len() as f64;
    for (i, r) in rows.iter().enumerate() {
        let growth = if i > 0 && rows[i - 1].values[5] != 0 {
            (r.values[5] - rows[i - 1].values[5]) as f64 / rows[i - 1].values[5] as f64 * 100.0
        } else {
            0.0
        };
        write!(report, "{}", r.quarter)?;
        for n in r.values {
            write!(report, ",{n}")?;
        }
        writeln!(report, ",{growth:.6}")?;
        let x = 60.0 + i as f64 * step;
        let mut bottom = 520.0;
        for (j, n) in r.values[..5].iter().enumerate() {
            let h = *n as f64 / max_total as f64 * 430.0;
            bottom -= h;
            writeln!(
                svg,
                "<rect x=\"{x:.3}\" y=\"{bottom:.3}\" width=\"{:.3}\" height=\"{h:.3}\" fill=\"{}\"/>",
                step * 0.75,
                colours[j]
            )?;
        }
        let q = r
            .quarter
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&#39;");
        writeln!(
            svg,
            "<text x=\"{x:.3}\" y=\"{:.3}\" font-size=\"10\">{}</text><text transform=\"translate({x:.3} 535) rotate(30)\" font-size=\"10\">{q}</text>",
            bottom - 8.0,
            r.values[5]
        )?;
    }
    svg.push_str("</svg>\n");
    fs::write(Path::new(&args[2]).join("analysis.csv"), report)?;
    fs::write(Path::new(&args[2]).join("revenue.svg"), svg)?;
    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
