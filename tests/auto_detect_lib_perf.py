"""Build-time performance guard for the `auto-detect-python-lib` feature.

The feature is build-time only, so this script compares the median time of
several clean rebuilds of `pyo3-ffi` with the feature enabled and disabled.

Runtime performance is covered by the existing `pyo3-benches` workflow; the
feature does not change generated code paths (only `#[link]` metadata).

Exit codes:
    0: build-time median is within the allowed 2% noise budget
    1: performance regression detected or a build failed
"""

from __future__ import annotations

import argparse
import os
import statistics
import subprocess
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
TARGET_DIR = ROOT / "target" / "auto-detect-perf"
FEATURE = "auto-detect-python-lib"
MAX_REGRESSION = 1.02


def cargo_build(features: str | None) -> float:
    env = os.environ.copy()
    env["CARGO_TARGET_DIR"] = str(TARGET_DIR)
    env["CARGO_TERM_COLOR"] = "never"

    subprocess.run(
        ["cargo", "clean", "--package", "pyo3-ffi"],
        cwd=ROOT,
        env=env,
        check=True,
        capture_output=True,
    )

    command = ["cargo", "build", "--package", "pyo3-ffi"]
    if features:
        command.extend(["--features", features])
    started = time.perf_counter()
    process = subprocess.run(
        command, check=False, cwd=ROOT, env=env, capture_output=True, text=True
    )
    elapsed = time.perf_counter() - started
    if process.returncode != 0:
        raise SystemExit(
            f"`{' '.join(command)}` failed:\n{process.stdout}{process.stderr}"
        )
    return elapsed


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--samples", type=int, default=5)
    args = parser.parse_args()
    if args.samples < 3:
        parser.error("--samples must be at least 3")

    # Warm up dependency compilation for both feature configurations.
    cargo_build(None)
    cargo_build(FEATURE)

    baseline = [cargo_build(None) for _ in range(args.samples)]
    enabled = [cargo_build(FEATURE) for _ in range(args.samples)]

    baseline_median = statistics.median(baseline)
    enabled_median = statistics.median(enabled)
    ratio = enabled_median / baseline_median

    print(f"samples: {args.samples}")
    print(f"baseline (feature off): {baseline}")
    print(f"enabled  (feature on): {enabled}")
    print(f"median baseline: {baseline_median:.3f}s")
    print(f"median enabled:  {enabled_median:.3f}s")
    print(f"ratio: {ratio:.4f} (limit {MAX_REGRESSION})")
    print(
        "note: interpreter probing is reused from the existing build-script path; "
        "runtime performance is verified by the pyo3-benches workflow"
    )

    if ratio > MAX_REGRESSION:
        print("FAIL: build-time performance regression exceeds 2%")
        return 1
    print("PASS: build-time performance within the 2% noise budget")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
