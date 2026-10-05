# Setup Project

Prepare the Rust revenue CLI for development.

1. Verify stable Rust and Cargo, a C compiler, and Poppler's `pdftotext` are installed.
2. On Linux, install `pkg-config`, Fontconfig development libraries, and fonts such as DejaVu. Confirm `curl` is available for downloads; install and authenticate `gh` only if releases are needed.
3. Build with `cargo build --release --locked`.
4. Run `cargo fmt --all -- --check`, `cargo clippy --locked --all-targets -- -D warnings`, and `cargo test --locked`.
5. Optionally run `pre-commit install` if the separate pre-commit development tool is installed.
6. Confirm the project is ready by running `cargo run --locked -- --help`.

Do not install Python application dependencies; Python is only used by the historical language-comparison harness.
