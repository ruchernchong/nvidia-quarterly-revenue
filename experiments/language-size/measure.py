"""Build, verify, and measure equivalent native-language revenue prototypes.

Python is only the benchmark harness; none of the prototypes needs it at runtime.
Logical file sizes exclude system libraries, input data, outputs, and compiler tools.
"""

import argparse
import csv
import io
import json
import os
from pathlib import Path
import platform
import re
import shutil
import subprocess
import time
import xml.etree.ElementTree as ET

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
ARTIFACTS = HERE / ".artifacts"
RESULTS = HERE / "results"
COLUMNS = ["quarter", "data_center", "gaming", "professional_visualization", "automotive", "oem_other", "total_revenue"]


def execute(command, *, cwd=ROOT, env=None):
    started = time.perf_counter()
    result = subprocess.run([str(s) for s in command], cwd=cwd, env=env, text=True, capture_output=True)
    if result.returncode:
        raise RuntimeError(f"{command}:\n{result.stdout}\n{result.stderr}")
    return result.stdout.strip(), time.perf_counter() - started


def tree_bytes(path):
    # Count stored regular-file bytes, without following or double-counting symlinks.
    return sum(p.stat().st_size for p in path.rglob("*") if p.is_file() and not p.is_symlink())


def native_dependencies(executable):
    """Enumerate transitive non-system Mach-O libraries, resolving loader rpaths."""
    files = set()
    unresolved = set()

    def visit(path, inherited=()):
        path = path.resolve()
        if path in files:
            return
        files.add(path)
        commands, _ = execute(["otool", "-l", path])
        paths = re.findall(r"cmd LC_RPATH\s+cmdsize \d+\s+path (.*?) \(offset", commands)

        def expand(s):
            return s.replace("@loader_path", str(path.parent)).replace("@executable_path", str(executable.resolve().parent))

        rpaths = tuple(expand(p) for p in paths) + tuple(inherited)
        linked, _ = execute(["otool", "-L", path])
        for line in linked.splitlines()[1:]:
            name = line.strip().split(" (compatibility")[0]
            if name.startswith(("/usr/lib/", "/System/Library/")):
                continue
            if name.startswith("@rpath/"):
                candidates = [Path(p) / name.removeprefix("@rpath/") for p in rpaths]
            else:
                candidates = [Path(expand(name))]
            target = next((p for p in candidates if p.exists()), None)
            if target is None:
                unresolved.add(name)
            elif target.resolve() != path:
                visit(target, rpaths)

    visit(executable)
    if unresolved:
        raise RuntimeError(f"Unresolved native libraries: {sorted(unresolved)}")
    return {"bytes": sum(p.stat().st_size for p in files), "files": [str(p) for p in sorted(files)]}


def reference_rows(input_path):
    if input_path.suffix == ".csv":
        with input_path.open(newline="") as f:
            return [{key: row[key] for key in COLUMNS} for row in csv.DictReader(f)]
    # Independent Python parser used only for validation of the shared converter.
    text, _ = execute(["pdftotext", "-f", "1", "-l", "1", "-layout", input_path, "-"])
    quarters = re.findall(r"Q[1-4]\s+FY\d+", text)
    segments = {}
    labels = {"Data Center": "data_center", "Gaming": "gaming", "Professional": "professional_visualization", "Auto": "automotive", "OEM & Other": "oem_other", "TOTAL": "total_revenue"}
    current = None
    for line in text.splitlines():
        for label, key in labels.items():
            if line.strip().startswith(label):
                current = key
        cells = re.findall(r"\$?[\d,]+", line)
        if current and "FY" not in line and len(cells) == len(quarters):
            segments[current] = [int(cell.replace("$", "").replace(",", "")) for cell in cells]
            current = None
    assert quarters and len(segments) == 6, input_path
    return [dict(zip(COLUMNS, [quarters[i]] + [str(segments[key][i]) for key in COLUMNS[1:]])) for i in reversed(range(len(quarters)))]


def verify(output, expected):
    with (output / "analysis.csv").open(newline="") as f:
        actual = list(csv.DictReader(f))
    assert len(actual) == len(expected)
    previous = 0
    for row, ref in zip(actual, expected):
        assert {key: row[key] for key in COLUMNS} == ref, (row, ref)
        total = int(ref["total_revenue"])
        growth = (total - previous) / previous * 100 if previous else 0
        assert abs(float(row["qoq_percent"]) - growth) <= 0.00000051
        previous = total
    chart = ET.parse(output / "revenue.svg").getroot()
    ns = {"s": "http://www.w3.org/2000/svg"}
    bars = chart.findall("s:rect", ns)[1:]
    assert len(bars) == len(expected) * 5
    maximum = max(int(row["total_revenue"]) for row in expected)
    for i, row in enumerate(expected):
        bottom = 520.0
        for j, key in enumerate(COLUMNS[1:6]):
            bar = bars[i * 5 + j]
            height = int(row[key]) / maximum * 430
            bottom -= height
            assert abs(float(bar.attrib["height"]) - height) <= 0.00051
            assert abs(float(bar.attrib["y"]) - bottom) <= 0.00051
    labels = [node.text for node in chart.findall("s:text", ns)]
    assert all(row["quarter"] in labels for row in expected)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--julia", type=Path, help="Path to a Julia executable; runtime tree is its parent directory's parent")
    args = parser.parse_args()
    ARTIFACTS.mkdir(exist_ok=True)
    RESULTS.mkdir(exist_ok=True)
    julia = args.julia or next(iter(sorted((ARTIFACTS / "julia-runtime").glob("*/bin/julia"))), None)
    if julia is None:
        installed = shutil.which("julia")
        julia = Path(installed) if installed else None
    if julia is None:
        raise RuntimeError("Provide --julia or unpack an official Julia tarball in .artifacts/julia-runtime")
    julia = julia.resolve()
    env = dict(os.environ)
    env.update({
        "GOCACHE": str(ARTIFACTS / "go-cache"), "GOPATH": str(ARTIFACTS / "go-path"),
        "CGO_ENABLED": "0", "CARGO_TARGET_DIR": str(ARTIFACTS / "rust-target"),
        "JULIA_DEPOT_PATH": str(ARTIFACTS / "julia-depot"),
        "JULIA_LOAD_PATH": "@stdlib", "JULIA_HISTORY": str(ARTIFACTS / "julia-history"),
    })
    versions = {}
    for name, command in [("go", ["go", "version"]), ("rust", ["rustc", "--version"]), ("julia", [julia, "--version"])]:
        versions[name] = execute(command, env=env)[0]
    build_times = {}
    _, build_times["go"] = execute(["go", "build", "-trimpath", "-ldflags=-s -w", "-o", ARTIFACTS / "go-revenue", "."], cwd=HERE / "go", env=env)
    _, build_times["rust"] = execute(["cargo", "build", "--release", "--offline"], cwd=HERE / "rust", env=env)
    commands = {
        "go": [ARTIFACTS / "go-revenue"],
        "rust": [ARTIFACTS / "rust-target/release/revenue-size-prototype"],
        "julia": [julia, "--startup-file=no", HERE / "julia/main.jl"],
    }
    inputs = [ROOT / "data/revenue_export.csv", *sorted((ROOT / "data").glob("*.pdf"))]
    references = {p: reference_rows(p) for p in inputs}
    export = {row["quarter"]: row for row in references[inputs[0]]}
    overlap = 0
    for p in inputs[1:]:
        for row in references[p]:
            if row["quarter"] in export:
                assert row == export[row["quarter"]], (p, row, export[row["quarter"]])
                overlap += 1
    checks, run_times = {}, {}
    for name, command in commands.items():
        run_times[name] = {}
        for path in inputs:
            output = RESULTS / name / path.stem
            _, elapsed = execute([*command, path, output], env=env)
            verify(output, references[path])
            run_times[name][path.name] = elapsed
        # Test growth after zero revenue and fail-fast behaviour on invalid input.
        fixture = ARTIFACTS / "edge.csv"
        fixture.write_text("quarter,fiscal_year,quarter_number,data_center,gaming,professional_visualization,automotive,oem_other,total_revenue\nQ1 FY26,2026,1,0,0,0,0,0,0\nQ2 FY26,2026,2,1,2,3,4,5,15\n")
        output = RESULTS / name / "edge"
        execute([*command, fixture, output], env=env)
        verify(output, reference_rows(fixture))
        invalid = ARTIFACTS / "invalid.csv"
        invalid.write_text(fixture.read_text().replace(",15\n", ",16\n"))
        for bad in [invalid, ARTIFACTS / "missing.csv"]:
            result = subprocess.run([str(s) for s in [*command, bad, output]], env=env, capture_output=True)
            assert result.returncode != 0, (name, bad)
        checks[name] = {"inputs_passed": len(inputs), "quarters_verified": sum(map(len, references.values())), "edge_and_failure_checks": 3}
    converter = native_dependencies(Path(shutil.which("pdftotext")))
    sizes = {}
    for name, command in commands.items():
        source = tree_bytes(HERE / name)
        if name == "julia":
            runtime = tree_bytes(julia.parent.parent)
            deployment = source + runtime
            build_workspace = tree_bytes(ARTIFACTS / "julia-depot")
        else:
            runtime = 0
            closure = native_dependencies(Path(command[0]))
            assert len(closure["files"]) == 1, closure
            deployment = closure["bytes"]
            build_workspace = tree_bytes(ARTIFACTS / ("go-cache" if name == "go" else "rust-target"))
        sizes[name] = {
            "source_project_bytes": source,
            "executable_bytes": Path(command[0]).stat().st_size,
            "runtime_tree_bytes": runtime,
            "csv_deployment_bytes": deployment,
            "pdf_deployment_bytes": deployment + converter["bytes"],
            "build_cache_or_target_bytes": build_workspace,
        }
    report = {
        "platform": platform.platform(), "versions": versions, "sizes": sizes,
        "shared_pdf_converter": converter, "verification": checks,
        "pdf_quarters_cross_checked_against_existing_export": overlap,
        "build_seconds_with_existing_cache": build_times, "observed_run_seconds": run_times,
        "measurement": "Logical regular-file bytes; compiler toolchains, OS libraries, inputs and outputs excluded. Julia uses the entire official runtime, not an AOT/trimmed build. PDF dependency closure includes pdftotext and all non-system linked libraries. No third-party language packages.",
        "scope": "CSV and page-one PDF input, segment validation, total revenue QoQ, one stacked SVG chart. Excludes SQLite, downloads/releases, eight additional charts and PNG rendering.",
    }
    (RESULTS / "measurements.json").write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps({"sizes": sizes, "verification": checks, "export_overlap": overlap}, indent=2))


if __name__ == "__main__":
    main()
