# Add New Quarter Data

Add a quarterly NVIDIA revenue report and verify the resulting analysis.

1. Check the filename contains its fiscal quarter and year, for example `Rev_by_Mkt_Qtrly_Trend_Q226.pdf` for Q2 FY26, and place the PDF in `data/`.
2. If a download is requested, use `cargo run --locked -- download --year <fiscal-year> --quarter <1-4> --data-dir data`.
3. Run `cargo run --locked -- analyse "<PDF File>"` and verify all five market segments and total revenue were extracted correctly.
4. Import and export history with `cargo run --locked -- batch --import --csv data/revenue_export.csv --json data/revenue_export.json`.
5. Run `cargo run --locked -- analyse` so the nine generated charts represent the latest local report, and inspect the images.
6. Report the new quarter and updated data/chart outputs. Creating a release publishes externally; do so only when the user requests it.
