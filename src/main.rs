use anyhow::{Context, Result, ensure};
use clap::{Args, Parser, Subcommand};
use nvidia_quarterly_revenue::{charts, database::Database, download, growth_rate, pdf, release};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    version,
    about = "Extract and visualise NVIDIA quarterly revenue reports",
    subcommand_negates_reqs = true,
    args_conflicts_with_subcommands = true
)]
struct Cli {
    /// PDF to analyse (defaults to the latest quarterly report).
    pdf: Option<PathBuf>,
    #[arg(long, default_value = "data")]
    data_dir: PathBuf,
    #[arg(long, default_value = "charts")]
    output_dir: PathBuf,
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Args)]
struct AnalyseArgs {
    pdf: Option<PathBuf>,
    #[arg(long, default_value = "data")]
    data_dir: PathBuf,
    #[arg(long, default_value = "charts")]
    output_dir: PathBuf,
}

#[derive(Subcommand)]
enum Commands {
    /// Analyse a PDF and generate nine PNG charts.
    #[command(alias = "analyze")]
    Analyse(AnalyseArgs),
    /// Import PDFs into SQLite and/or export CSV/JSON.
    Batch {
        #[arg(long = "import")]
        do_import: bool,
        #[arg(long, value_name = "FILE")]
        csv: Option<PathBuf>,
        #[arg(long, value_name = "FILE")]
        json: Option<PathBuf>,
        #[arg(long, default_value = "data")]
        data_dir: PathBuf,
        #[arg(long, default_value = "data/data.db")]
        db: PathBuf,
    },
    /// Download available quarterly PDFs from NVIDIA's CDN.
    Download {
        #[arg(long, default_value = "data")]
        data_dir: PathBuf,
        #[arg(long, default_value_t = 1, value_parser = clap::value_parser!(u8).range(0..=20))]
        check_previous: u8,
        #[arg(long, default_value_t = 2, value_parser = clap::value_parser!(u8).range(0..=20))]
        check_future: u8,
        #[arg(long, requires = "quarter", value_parser = clap::value_parser!(i32).range(2000..=2099))]
        year: Option<i32>,
        #[arg(long, requires = "year", value_parser = clap::value_parser!(u8).range(1..=4))]
        quarter: Option<u8>,
    },
    /// Create the latest quarter's GitHub release with gh.
    Release {
        #[arg(long, default_value = "data")]
        data_dir: PathBuf,
    },
    /// Print the latest quarterly PDF path.
    Latest {
        #[arg(long, default_value = "data")]
        data_dir: PathBuf,
    },
}

fn analyse(args: AnalyseArgs) -> Result<()> {
    let path = match args.pdf {
        Some(path) => path,
        None => pdf::get_latest_pdf(&args.data_dir)?,
    };
    println!("Processing: {}", path.display());
    let rows = pdf::extract_data_from_pdf(&path)?;
    for pair in rows.windows(2) {
        println!(
            "{}: {:+.2}%",
            pair[1].quarter,
            growth_rate(pair[1].total_revenue as f64, pair[0].total_revenue as f64)
        );
    }
    for file in charts::generate(&rows, &args.output_dir)? {
        println!("Saved: {}", file.display());
    }
    println!("All charts generated successfully!");
    Ok(())
}

fn run(cli: Cli) -> Result<()> {
    match cli.command {
        None => analyse(AnalyseArgs {
            pdf: cli.pdf,
            data_dir: cli.data_dir,
            output_dir: cli.output_dir,
        }),
        Some(Commands::Analyse(args)) => analyse(args),
        Some(Commands::Latest { data_dir }) => {
            println!("{}", pdf::get_latest_pdf(&data_dir)?.display());
            Ok(())
        }
        Some(Commands::Download {
            data_dir,
            check_previous,
            check_future,
            year,
            quarter,
        }) => {
            let files = match (year, quarter) {
                (Some(year), Some(quarter)) => {
                    download::download_quarter(year, quarter, &data_dir)?
                        .into_iter()
                        .collect()
                }
                _ => download::download_latest(&data_dir, check_previous, check_future)?,
            };
            println!("Downloaded {} new file(s)", files.len());
            Ok(())
        }
        Some(Commands::Release { data_dir }) => release::create_latest_release(&data_dir),
        Some(Commands::Batch {
            do_import,
            csv,
            json,
            data_dir,
            db,
        }) => {
            ensure!(
                do_import || csv.is_some() || json.is_some(),
                "batch requires --import, --csv, or --json; see batch --help"
            );
            let mut database = Database::open(&db)?;
            let mut errors = Vec::new();
            if do_import {
                let files = pdf::pdf_files(&data_dir)?;
                ensure!(!files.is_empty(), "no PDFs found in {}", data_dir.display());
                for path in files {
                    let import = (|| {
                        let rows = pdf::extract_data_from_pdf(&path)?;
                        let name = path
                            .file_name()
                            .context("PDF filename missing")?
                            .to_string_lossy();
                        let count = database.insert_quarterly_data(&rows, &name)?;
                        println!("Imported {count} quarter(s): {name}");
                        Ok::<_, anyhow::Error>(())
                    })();
                    if let Err(error) = import {
                        eprintln!("Failed {}: {error:#}", path.display());
                        errors.push(path.display().to_string());
                    }
                }
            }
            if let Some(path) = csv {
                database.export_to_csv(&path)?;
                println!("Exported CSV: {}", path.display());
            }
            if let Some(path) = json {
                database.export_to_json(&path)?;
                println!("Exported JSON: {}", path.display());
            }
            ensure!(
                errors.is_empty(),
                "{} PDF import(s) failed: {}",
                errors.len(),
                errors.join(", ")
            );
            Ok(())
        }
    }
}

fn main() {
    if let Err(error) = run(Cli::parse()) {
        eprintln!("Error: {error:#}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_accepts_default_positional_and_subcommands() {
        assert!(Cli::try_parse_from(["nvidia-revenue"]).is_ok());
        assert!(Cli::try_parse_from(["nvidia-revenue", "example.pdf"]).is_ok());
        assert!(
            Cli::try_parse_from([
                "nvidia-revenue",
                "analyse",
                "example.pdf",
                "--output-dir",
                "output"
            ])
            .is_ok()
        );
        assert!(
            Cli::try_parse_from(["nvidia-revenue", "batch", "--import", "--csv", "out.csv"])
                .is_ok()
        );
        assert!(Cli::try_parse_from(["nvidia-revenue", "download", "--quarter", "2"]).is_err());
        assert!(
            Cli::try_parse_from([
                "nvidia-revenue",
                "download",
                "--year",
                "2026",
                "--quarter",
                "5"
            ])
            .is_err()
        );
        assert!(
            Cli::try_parse_from(["nvidia-revenue", "download", "--check-future", "21"]).is_err()
        );
    }
}
