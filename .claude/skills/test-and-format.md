# Test and Format

Run the Rust test and code-quality suite before completing changes.

1. Format with `cargo fmt --all`.
2. Check formatting with `cargo fmt --all -- --check`.
3. Check lints with `cargo clippy --locked --all-targets -- -D warnings`.
4. Run all tests with `cargo test --locked`. PDF and chart integration checks require Poppler and system fonts.
5. If the optional pre-commit tool is installed, run `pre-commit run --all-files`.
6. Report any failures and fix issues within the requested scope.
