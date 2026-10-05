"""Measure CPU seconds, wall time and peak RSS for the existing prototypes.

Uses wait4 rather than time(1), which cannot read kern.clockrate in the sandbox.
Runs one process tree at a time, with warmed filesystem caches and fresh runtimes.
"""

import argparse
from datetime import datetime
import hashlib
import json
import os
from pathlib import Path
import platform
import random
import statistics
import subprocess
import time
from zoneinfo import ZoneInfo

from measure import ARTIFACTS, HERE, RESULTS, ROOT, reference_rows, verify, native_dependencies


def sample(command, env):
    start = time.perf_counter()
    with (ARTIFACTS / "resource-stderr.txt").open("wb") as stderr:
        process = subprocess.Popen(
            [str(part) for part in command], cwd=ROOT, env=env,
            stdout=subprocess.DEVNULL, stderr=stderr,
        )
        _, status, usage = os.wait4(process.pid, 0)
        process.returncode = os.waitstatus_to_exitcode(status)
    wall = time.perf_counter() - start
    if process.returncode:
        raise RuntimeError((ARTIFACTS / "resource-stderr.txt").read_text())
    # macOS returns bytes, Linux returns KiB. The report records this conversion.
    rss_bytes = usage.ru_maxrss * (1 if platform.system() == "Darwin" else 1024)
    return {
        "wall_seconds": wall,
        "user_cpu_seconds": usage.ru_utime,
        "system_cpu_seconds": usage.ru_stime,
        "total_cpu_seconds": usage.ru_utime + usage.ru_stime,
        "peak_rss_bytes": rss_bytes,
    }


def summary(samples):
    return {
        metric: {
            "median": statistics.median([row[metric] for row in samples]),
            "minimum": min(row[metric] for row in samples),
            "maximum": max(row[metric] for row in samples),
        }
        for metric in samples[0]
    }


def production_rust(runs):
    """Measure the full migrated CLI without rebuilding during measurement."""
    binary = ROOT / "target/release/nvidia-revenue"
    if not binary.exists():
        raise RuntimeError("Run cargo build --release --locked first")
    output = ROOT / ".artifacts/validation/release-charts"
    pdf = ROOT / "data/Rev_by_Mkt_Qtrly_Trend_Q326.pdf"
    command = [binary, "analyse", pdf, "--output-dir", output]
    env = dict(os.environ)
    for _ in range(2):
        sample(command, env)
    measured = []
    for _ in range(runs):
        measured.append(sample(command, env))
        images = sorted(output.glob("*.png"))
        assert len(images) == 9
        assert all(p.read_bytes().startswith(b"\x89PNG\r\n\x1a\n") for p in images)
    report = {
        "date": datetime.now(ZoneInfo("Asia/Singapore")).date().isoformat(),
        "platform": platform.platform(), "scope": "Full Rust CLI: extract eight-quarter PDF and render all nine PNG charts; includes startup and waited-for pdftotext child",
        "runs": runs, "warmups": 2,
        "release_binary_bytes": binary.stat().st_size,
        "release_binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
        "median": {key: statistics.median(s[key] for s in measured) for key in measured[0]},
        "raw_samples": measured,
        "method": "Sequential fresh processes with warm input/output caches. wait4 user+system CPU includes waited-for child; ru_maxrss is largest individual process peak, not summed process-tree memory. Python harness and compiler resources excluded.",
        "input": str(pdf.relative_to(ROOT)),
        "commands": ["cargo build --release --locked", "python3 experiments/language-size/benchmark_resources.py --production-rust"],
    }
    if platform.system() == "Darwin":
        report["native_libraries"] = native_dependencies(binary)
    (RESULTS / "production-rust.json").write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps(report["median"], indent=2))


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--runs", type=int, default=15)
    parser.add_argument("--julia-single-thread", action="store_true", help="Benchmark only Julia with runtime/BLAS thread counts capped at one; saves a separate report")
    parser.add_argument("--production-rust", action="store_true", help="Benchmark the migrated full CLI and all nine PNG charts instead of the prototypes")
    args = parser.parse_args()
    if args.runs < 5:
        parser.error("at least five runs are required")
    if platform.system() not in ("Darwin", "Linux"):
        parser.error("requires Unix wait4")
    if args.production_rust:
        if args.julia_single_thread:
            parser.error("--production-rust and --julia-single-thread cannot be combined")
        ARTIFACTS.mkdir(exist_ok=True)
        RESULTS.mkdir(exist_ok=True)
        production_rust(args.runs)
        return
    julia = next(iter(sorted((ARTIFACTS / "julia-runtime").glob("*/bin/julia"))), None)
    if julia is None:
        parser.error("unpack Julia in .artifacts/julia-runtime first")
    commands = {
        "go": [ARTIFACTS / "go-revenue"],
        "rust": [ARTIFACTS / "rust-target/release/revenue-size-prototype"],
        "julia": [julia, "--startup-file=no", HERE / "julia/main.jl"],
    }
    if args.julia_single_thread:
        commands = {"julia": commands["julia"]}
    for command in commands.values():
        if not command[0].exists():
            parser.error("run measure.py to build the prototypes first")
    env = dict(os.environ)
    env.update({"JULIA_DEPOT_PATH": str(ARTIFACTS / "julia-depot"), "JULIA_LOAD_PATH": "@stdlib"})
    thread_keys = ["JULIA_NUM_THREADS", "JULIA_NUM_GC_THREADS", "OPENBLAS_NUM_THREADS", "VECLIB_MAXIMUM_THREADS", "GOMAXPROCS"]
    if args.julia_single_thread:
        env.update({key: "1" for key in thread_keys if key != "GOMAXPROCS"})
    inputs = {
        "csv": ROOT / "data/revenue_export.csv",
        "pdf": ROOT / "data/Rev_by_Mkt_Qtrly_Trend_Q326.pdf",
    }
    raw = {workload: {name: [] for name in commands} for workload in inputs}
    rng = random.Random(20261006)
    for workload, path in inputs.items():
        expected = reference_rows(path)
        # Two unmeasured launches per language populate caches and verify output.
        for name, command in commands.items():
            output = RESULTS / "performance-output" / workload / name
            for _ in range(2):
                sample([*command, path, output], env)
            verify(output, expected)
        for repetition in range(args.runs):
            order = list(commands)
            rng.shuffle(order)
            for name in order:
                output = RESULTS / "performance-output" / workload / name
                raw[workload][name].append(sample([*commands[name], path, output], env))
            # Check every measured output outside the resource-measurement interval.
            for name in commands:
                verify(RESULTS / "performance-output" / workload / name, expected)
        print(f"Verified {args.runs} fresh-process measurements per language for {workload}", flush=True)
    # Shared PDF converter baseline helps explain the main-process memory difference.
    baseline_command = ["pdftotext", "-f", "1", "-l", "1", "-layout", inputs["pdf"], "-"]
    for _ in range(2):
        sample(baseline_command, env)
    converter = [sample(baseline_command, env) for _ in range(args.runs)]
    report = {
        "date": datetime.now(ZoneInfo("Asia/Singapore")).date().isoformat(), "platform": platform.platform(),
        "cpu_model": subprocess.run(["sysctl", "-n", "machdep.cpu.brand_string"], text=True, capture_output=True).stdout.strip() if platform.system() == "Darwin" else platform.processor(),
        "versions": json.loads((RESULTS / "measurements.json").read_text())["versions"],
        "thread_environment": {key: env.get(key) for key in thread_keys},
        "julia_single_thread_controls": args.julia_single_thread,
        "runs_per_language_per_workload": args.runs, "warmups_per_language_per_workload": 2,
        "method": {
            "cpu": "wait4 user + system CPU time; includes resource usage of children that the prototype waits for, including pdftotext. Python harness and output verification excluded.",
            "memory": "wait4 ru_maxrss high-water mark, converted from bytes on macOS. For process trees this is the largest single-process RSS high-water mark, not summed concurrent memory of parent and child.",
            "wall": "Python perf_counter around process creation and wait4, including launcher overhead and process startup.",
            "execution": "Sequential fresh process launches, two warmups, shuffled language order each repetition, warmed filesystem and persistent Julia package caches. Julia still compiles application code in each new process. No builds inside timed intervals.",
            "scope": "Original small inputs: CSV (21 quarters), latest repository PDF (8 quarters). Writes the same analysis CSV and one SVG chart. These are CLI invocation costs, not long-running compute throughput or complete Python-migration results.",
        },
        "inputs": {key: {"path": str(path.relative_to(ROOT)), "bytes": path.stat().st_size, "sha256": hashlib.sha256(path.read_bytes()).hexdigest(), "quarters": len(reference_rows(path))} for key, path in inputs.items()},
        "summary": {key: {name: summary(samples) for name, samples in languages.items()} for key, languages in raw.items()},
        "pdf_converter_baseline": summary(converter), "raw_samples": raw, "pdf_converter_raw_samples": converter,
    }
    filename = "resources-julia-single-thread.json" if args.julia_single_thread else "resources.json"
    (RESULTS / filename).write_text(json.dumps(report, indent=2) + "\n")
    for workload, languages in report["summary"].items():
        print(workload)
        for name, data in languages.items():
            print(f"  {name}: CPU {data['total_cpu_seconds']['median'] * 1000:.3f} ms; RSS {data['peak_rss_bytes']['median'] / 1048576:.2f} MiB; wall {data['wall_seconds']['median'] * 1000:.3f} ms")


if __name__ == "__main__":
    main()
