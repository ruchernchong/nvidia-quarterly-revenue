# Go, Rust and Julia size experiment

The application has since been migrated to Rust with SQLite and all nine PNG charts. The tables below describe the original smaller prototypes. See [production-rust.json](results/production-rust.json) and the [project README](../../README.md) for the full CLI's measurements. Reproduce those with `python3 experiments/language-size/benchmark_resources.py --production-rust` after building the release executable.

Measured on 5 October 2026 on macOS Apple Silicon. **Rust had the smallest runtime footprint in this experiment; Julia had the smallest source project.** This replaces the earlier estimate that Go would be smallest.

| Language | Source project | CSV runtime footprint | With PDF executable and libraries | Local build cache / target |
| --- | ---: | ---: | ---: | ---: |
| Rust | 6.74 KiB | 0.34 MiB | 15.66 MiB | 0.68 MiB |
| Go | 5.18 KiB | 1.88 MiB | 17.20 MiB | 39.64 MiB |
| Julia | 4.21 KiB | 766.24 MiB | 781.56 MiB | 0.27 MiB |

One KiB = 1,024 bytes; one MiB = 1,048,576 bytes. Exact bytes, tool versions, dependency paths and validation counts are in [measurements.json](results/measurements.json).

## Equivalent functionality

Each independently implemented prototype:

- Reads the existing repository revenue CSV or the first page of a quarterly PDF.
- Extracts the five named market segments and total revenue, in chronological order.
- Checks segment totals and calculates total revenue quarter-over-quarter growth.
- Writes `analysis.csv` and a stacked `revenue.svg` with market colours, legend, revenue totals and quarter labels.
- Rejects missing inputs and inconsistent totals.

Older PDFs put Gaming before Data Centre. Each parser identifies segments by label, including the multi-line Professional Visualisation label.

This is a working migration prototype, not a complete replacement for the Python project. SQLite, PDF downloads, GitHub releases, automatic latest-PDF selection, eight additional charts and PNG rendering are outside the experiment. Those features can change the final size. Rust and Julia's CSV readers deliberately support this repository's unquoted export format; they reject quoted fields rather than silently misparse them.

## What was measured

- **Source project:** regular files in each language directory, including manifests and Rust's generated dependency lockfile. Excludes the shared harness, README, input data and generated results.
- **CSV runtime footprint:** Go or Rust's executable; Julia's source plus the entire official runtime directory. Assumes the operating system's libraries are already installed.
- **PDF footprint:** CSV runtime footprint plus `pdftotext` and its transitive non-system Mach-O library dependencies: 16,061,816 bytes (15.32 MiB), across 30 files. This measures binaries and libraries, not a tested relocatable distribution or optional resource files/licences.
- **Build cache / target:** the experiment's isolated Go cache, Rust target directory or Julia depot after building/running. Compiler toolchains are excluded. This is an observed local working-directory size, not a minimum or universal requirement.
- Counts logical regular-file bytes, excluding symlinks and filesystem allocation overhead. Runtime archives and generated charts are excluded.

Go uses `CGO_ENABLED=0`, `-trimpath` and `-ldflags='-s -w'`. Rust uses release optimisation for size, LTO, one codegen unit, symbol stripping and abort-on-panic. Neither uses third-party language packages. Both executables link only operating-system libraries for the CSV path.

Julia uses `--startup-file=no`, the standard-library `Printf` module and an isolated depot. The 68,032-byte Julia launcher is **not** the whole runtime. The measured official runtime tree is 803,456,386 bytes. No PackageCompiler, JuliaC, trimmed/AOT runtime or custom sysimage was attempted; this result does not establish Julia's theoretical minimum executable size.

Versions: Go 1.27.1, Rust 1.98.1, Julia 1.13.1. The [official Julia binary](https://julialang.org/downloads/manual-downloads/) was downloaded into `.artifacts/julia-runtime/` and checked against its published SHA-256:

```text
a3e0259d4777c2b776c2cba134d5b6c6c124048dcb913f8881517892c49a0f7f
```

## CPU and RAM comparison

Measured on 6 October 2026 on the same macOS ARM64 host. These are medians of **15 measured fresh-process launches** for each language and input, after two unmeasured warmups. Language order was shuffled each round, with one invocation running at a time. Every measured output passed the data, growth and SVG checks.

| Language | CSV CPU time | CSV peak RSS | PDF CPU time | PDF peak RSS |
| --- | ---: | ---: | ---: | ---: |
| Rust | 1.89 ms | 2.03 MiB | 14.23 ms | 11.12 MiB |
| Go | 2.77 ms | 5.94 MiB | 15.16 ms | 11.14 MiB |
| Julia, default thread settings | 4,175.82 ms | 291.39 MiB | 4,182.35 ms | 291.31 MiB |
| Julia, runtime/library threads capped at one | 735.52 ms | 288.50 MiB | 759.37 ms | 289.00 MiB |

**Rust had the lowest measured CPU and memory costs.** For PDFs, Go and Rust are close: the common converter alone used a median 13.17 ms CPU time and 11.11 MiB peak RSS. The sub-millisecond CPU difference between Go and Rust is small and should not determine the migration by itself.

CPU time is user plus system time collected through `wait4`, including waited-for child processes such as `pdftotext`. It sums work across threads, so it can exceed elapsed time: default Julia's median elapsed times were 862.59 ms for CSV and 865.01 ms for PDF, despite approximately 4.2 CPU-seconds per invocation. Capping Julia runtime/GC and numerical-library thread settings to one reduced CPU cost substantially; the experiment does not attribute the extra default CPU cost to a particular library. This controlled variant set `JULIA_NUM_THREADS`, `JULIA_NUM_GC_THREADS`, `OPENBLAS_NUM_THREADS` and `VECLIB_MAXIMUM_THREADS` to `1`.

Peak RSS comes from `wait4.ru_maxrss` (bytes on macOS). For the PDF process tree it reports the largest single-process high-water mark, **not the sum of simultaneous parent and child memory**. Thus the PDF RAM column reflects the shared converter for Go and Rust. CSV measurements show the prototypes' own process peaks more directly. RSS measures resident memory, not virtual address space or bytes allocated over time.

These small workloads contain 21 CSV quarters or eight quarters from `Rev_by_Mkt_Qtrly_Trend_Q326.pdf`. Results include startup, file access, output writes and Julia's per-process application compilation. Filesystem and persistent Julia package caches are warm; Julia application execution still starts in a fresh process each time. This measures the project's intended short CLI usage, not Julia's warmed long-running numerical throughput. Full SQLite and chart-suite performance is not measured. Compiler/build costs and the Python harness's CPU/RAM are excluded.

Exact observations, ranges and individual runs are in [resources.json](results/resources.json) and [resources-julia-single-thread.json](results/resources-julia-single-thread.json). The CPU model string could not be read in this sandbox; both reports identify the OS and architecture.

Reproduce from the repository root after building with `measure.py`:

```sh
python3 experiments/language-size/benchmark_resources.py
python3 experiments/language-size/benchmark_resources.py --julia-single-thread
```

The resource harness uses `wait4` because the sandbox prevents `/usr/bin/time` from reading `kern.clockrate`. It stores its outputs in an ignored results subdirectory and its JSON reports alongside the size measurements.

## Verification and reproduction

All three passed the CSV plus all nine PDFs: **10 inputs and 93 quarter records per language**. Every extracted segment and total was checked against an independent Python reference parser; 71 PDF quarter records were also checked against the existing revenue CSV. Growth percentages were checked to six decimal places. SVG bar counts, heights, vertical positions and quarter labels were verified. Each prototype also passed a zero-previous-revenue case and rejected missing input and mismatched totals.

From the repository root, with Go, Cargo, Python 3, `pdftotext`, `otool` and the downloaded Julia runtime available:

```sh
python3 experiments/language-size/measure.py
```

For a different Julia installation:

```sh
python3 experiments/language-size/measure.py --julia /absolute/path/to/julia/bin/julia
```

The script builds the Go and Rust prototypes, executes and verifies all three, and updates `results/measurements.json`. It measures the Julia runtime as the directory two levels above the resolved executable; use a complete official runtime layout for a comparable result. Measured build times use existing caches and run times include process startup; these timings are incidental and are not a controlled performance benchmark.

Run a prototype directly after building:

```sh
experiments/language-size/.artifacts/go-revenue data/revenue_export.csv /tmp/revenue-go
experiments/language-size/.artifacts/rust-target/release/revenue-size-prototype data/revenue_export.csv /tmp/revenue-rust
experiments/language-size/.artifacts/julia-runtime/julia-1.13.1/bin/julia --startup-file=no experiments/language-size/julia/main.jl data/revenue_export.csv /tmp/revenue-julia
```

Replace the CSV path with any repository PDF to exercise PDF extraction. Python is used only for the verification/measurement harness, not by the prototypes. Runtime downloads, compiler caches and per-language generated output directories are ignored by Git. The original Python application and its data/charts remain unchanged.
