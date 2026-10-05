# Claude Code Skills

These reusable workflows support the Rust NVIDIA quarterly revenue CLI.

- **analyze** runs PDF extraction, growth calculations, and all nine PNG charts for the latest or a specified report.
- **test-and-format** runs rustfmt, Clippy, Rust tests, and optional pre-commit checks.
- **setup-project** verifies Rust and native dependencies, builds the executable, and runs checks.
- **add-new-quarter** validates report naming, updates SQLite and exports, and generates the latest charts.

Invoke the corresponding skill for its task. To add another reusable workflow, create a Markdown file here with concrete commands and verification steps.
