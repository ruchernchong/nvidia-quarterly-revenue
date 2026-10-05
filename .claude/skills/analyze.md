# Analyse NVIDIA Revenue

Run the Rust revenue analysis pipeline.

1. Verify Cargo, Poppler's `pdftotext`, and chart-rendering fonts are available.
2. Run `cargo run --locked -- analyse` to select the latest quarterly PDF, or `cargo run --locked -- analyse "<PDF File>"` for a specified report.
3. Inspect the printed quarter-over-quarter growth and the nine PNGs in `charts/`.
4. Report extraction or rendering failures with the input path and error message.

Use `--data-dir` and `--output-dir` when custom directories are needed. Analysis generates charts; use `batch --import` separately to update SQLite and exports.
