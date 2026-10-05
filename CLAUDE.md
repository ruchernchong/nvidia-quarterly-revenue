# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with this repository.

## Commands

- Build the application: `cargo build --release --locked`
- Install the local executable: `cargo install --path . --locked`
- Analyse the latest PDF: `cargo run --locked --`
- Analyse a specific PDF: `cargo run --locked -- analyse data/<PDF File>`
- Custom chart output: `cargo run --locked -- analyse --data-dir data --output-dir charts`
- Batch import and export: `cargo run --locked -- batch --import --csv data/revenue_export.csv --json data/revenue_export.json`
- Export an existing database: `cargo run --locked -- batch --db data/data.db --csv revenue.csv --json revenue.json`
- Find latest PDF: `cargo run --locked -- latest --data-dir data`
- Download reports: `cargo run --locked -- download --check-previous 1 --check-future 2`
- Download a specific quarter: `cargo run --locked -- download --year 2026 --quarter 2`
- Create a quarterly GitHub release: `cargo run --locked -- release` (publishes through authenticated `gh`)
- Run all tests: `cargo test --locked`
- Format code: `cargo fmt --all`
- Check formatting: `cargo fmt --all -- --check`
- Check lints: `cargo clippy --locked --all-targets -- -D warnings`
- Optional pre-commit hooks: `pre-commit run --all-files`

## Commit Message Style

- **Always use the `/commit` slash command** for consistency.
- Use plain descriptive messages without conventional commit prefixes.
- Capitalise the first letter, omit a trailing period, and keep messages to 50 characters or fewer.

Examples: `Correct chart paths to charts directory`, `Add new revenue analysis function`.

## Project Structure

```text
├── Cargo.toml              # Rust application manifest
├── Cargo.lock              # Committed dependency lockfile
├── src/                    # CLI, PDF parsing, charts, SQLite and utilities
├── tests/                  # Rust integration tests
├── data/                   # Quarterly PDFs, SQLite database and CSV/JSON exports
├── charts/                 # Nine generated chart PNGs
├── .github/workflows/      # Tests, PDF monitoring and scheduled downloads
└── experiments/language-size/  # Historical language-comparison prototypes
```

## Project Overview

This Rust CLI analyses NVIDIA quarterly revenue PDFs. Analysis selects a specific report or the latest fiscal quarter, extracts the five market segments and total revenue using Poppler's `pdftotext`, prints Q/Q growth, and generates nine PNG charts with Plotters. It does not open an interactive chart window.

The `batch` command imports every report in a data directory into SQLite and optionally exports CSV and JSON. `download` checks NVIDIA's report URL patterns, downloads using `curl`, and normalises filenames. `release` uses the authenticated GitHub CLI to create a release for the latest local report, skipping an existing release.

## Runtime and Build Dependencies

- Stable Rust and Cargo are required to build. Keep `Cargo.lock` committed.
- `pdftotext` (Poppler) is required for PDF analysis and imports.
- `curl` is required for downloads; `gh` is required only for GitHub releases.
- SQLite is bundled through rusqlite; do not require a separate SQLite installation.
- Linux builds need a C compiler, `pkg-config`, and Fontconfig development libraries. Install readable fonts such as DejaVu for chart rendering.
- The application has no Python runtime dependency. Historical comparison scripts under `experiments/` use Python as a verification harness.

## Data and Chart Contracts

- Preserve the SQLite `quarterly_revenue` and `import_metadata` schemas and exported column names.
- Preserve fiscal labels such as `Q3 FY26` and store revenue as integer millions of US dollars.
- Detect the latest report from fiscal quarter/year in its filename, for example `Q226` means Q2 FY26.
- Import overlapping reports in a deterministic order so the newest report wins for shared quarters.
- Show all quarters extracted from the selected PDF and keep the nine chart filenames already embedded in README.md.
- Use Data Centre, Gaming, Professional Visualisation, Automotive, and OEM & Other as display labels.
- Treat zero denominators and missing fiscal-year comparisons explicitly in growth calculations.
- The historical prototype's size and resource measurements exclude production SQLite and PNG dependencies; do not quote them as production results.

## Code Style Guidelines

- Use rustfmt and resolve Clippy warnings before completing changes.
- Use snake_case for variables, functions, and module files, and standard Rust type naming.
- Return contextual errors for invalid inputs, I/O failures, and external-command failures; avoid panics for normal user errors.
- Keep subprocess arguments as separate arguments rather than building shell commands from paths or user inputs.
- Add meaningful tests for parsing and calculations, SQLite compatibility, exports, and CLI behaviour.
- Keep trailing whitespace removed and files terminated by a newline.
- Use English (UK/Singapore) spelling, for example visualise, centre, and colour.
