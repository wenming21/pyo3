"""Cross-build checks for the `auto-detect-python-lib` feature.

The script only uses the standard library plus the `cargo` CLI. It is run via
the `test-auto-detect-lib` nox session.

Exit codes:
    0: all requested checks passed
    1: a check failed
    2: a requested mode is not available in this environment
"""

from __future__ import annotations

import argparse
import os
import platform
import re
import subprocess
import sys
import sysconfig
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
LOG_RE = re.compile(
    r"auto-detect-python-lib: lib_name=(?P<name>\S+) "
    r"target=(?P<target>\S+) tool=(?P<tool>\w+) "
    r"source=(?P<source>\w+)"
)


def expected_lib_name(*, abi3: bool = False) -> str:
    implementation = sys.implementation.name
    version = sys.version_info
    is_windows = sys.platform == "win32"
    debug = bool(sysconfig.get_config_var("Py_DEBUG"))
    free_threaded = bool(sysconfig.get_config_var("Py_GIL_DISABLED"))

    if implementation == "pypy":
        name = f"libpypy{version.major}.{version.minor}-c"
        return name if is_windows else name.removeprefix("lib")
    if implementation in {"graalpy", "graalvm"}:
        return "python-native"

    if abi3:
        name = f"python{version.major}"
        if free_threaded:
            name += "t"
    else:
        if is_windows:
            name = f"python{version.major}{version.minor}"
        else:
            name = f"python{version.major}.{version.minor}"
        if free_threaded:
            name += "t"

    if debug:
        name += "_d" if is_windows else "d"
    return name


def run_cargo(
    *extra_args: str, env: dict[str, str] | None = None
) -> tuple[int, str]:
    full_env = os.environ.copy()
    full_env["CARGO_TERM_COLOR"] = "never"
    if env:
        full_env.update(env)
    process = subprocess.run(
        ["cargo", "build", "-p", "pyo3-ffi", *extra_args],
        cwd=ROOT,
        env=full_env,
        capture_output=True,
        text=True,
    )
    return process.returncode, process.stdout + process.stderr


def parse_decision(output: str) -> dict[str, str]:
    match = LOG_RE.search(output)
    if match is None:
        raise AssertionError(
            "auto-detect decision log not found in cargo output:\n" + output
        )
    return match.groupdict()


def git_status_porcelain() -> str | None:
    try:
        process = subprocess.run(
            ["git", "status", "--porcelain", "--untracked-files=all"],
            cwd=ROOT,
            capture_output=True,
            text=True,
            check=True,
        )
    except (OSError, subprocess.CalledProcessError):
        return None
    return process.stdout


def assert_resource_tree_unchanged(before: str | None, after: str | None) -> None:
    if before is None or after is None:
        print("skip: git is not available; resource tree check skipped")
        return
    new_entries = sorted(set(after.splitlines()) - set(before.splitlines()))
    unexpected = [
        entry
        for entry in new_entries
        if "target/" not in entry and ".security/" not in entry
    ]
    check(
        not unexpected,
        "build wrote outside the build directory: " + repr(unexpected),
    )
    print("ok: source/resources tree unchanged by the build")


def check(condition: bool, message: str) -> None:
    if not condition:
        raise AssertionError(message)


def check_disabled_path() -> None:
    code, output = run_cargo()
    check(code == 0, "default (feature disabled) build failed:\n" + output)
    check(
        "auto-detect-python-lib:" not in output,
        "decision log emitted while the feature is disabled:\n" + output,
    )
    print("ok: feature disabled keeps the enumerated path")


def check_enabled_path() -> None:
    code, output = run_cargo("--features", "auto-detect-python-lib")
    check(code == 0, "auto-detect build failed:\n" + output)
    decision = parse_decision(output)
    expected = expected_lib_name()
    check(
        decision["name"] == expected,
        f"detected name {decision['name']!r} != expected {expected!r}",
    )
    check(decision["tool"] == "cargo", "decision log does not report tool=cargo")
    check(decision["source"] == "detected", "decision log does not report source=detected")
    print(f"ok: auto-detect name {decision['name']} (cargo)")


def check_conflict() -> None:
    code, output = run_cargo(
        "--features", "auto-detect-python-lib generate-import-lib"
    )
    check(code == 0, "feature conflict build must not fail:\n" + output)
    check(
        "generate-import-lib is disabled" in output,
        "feature conflict warning missing:\n" + output,
    )
    print("ok: generate-import-lib conflict warns and continues")


def check_abi3() -> None:
    free_threaded = bool(sysconfig.get_config_var("Py_GIL_DISABLED"))
    if free_threaded:
        if sys.version_info < (3, 15):
            print("skip: abi3 on free-threaded Python requires Python 3.15+")
            return
        features = "auto-detect-python-lib abi3t"
    else:
        features = "auto-detect-python-lib abi3"

    code, output = run_cargo("--features", features)
    check(code == 0, f"abi3 build failed ({features}):\n" + output)
    decision = parse_decision(output)
    expected = expected_lib_name(abi3=True)
    check(
        decision["name"] == expected,
        f"abi3 detected name {decision['name']!r} != expected {expected!r}",
    )
    print(f"ok: abi3 name {decision['name']}")


def print_config() -> str:
    env = os.environ.copy()
    env["PYO3_PRINT_CONFIG"] = "1"
    env["CARGO_TERM_COLOR"] = "never"
    process = subprocess.run(
        ["cargo", "build", "-p", "pyo3-ffi"],
        cwd=ROOT,
        env=env,
        capture_output=True,
        text=True,
    )
    output = process.stdout + process.stderr
    start = output.find("-- PYO3_PRINT_CONFIG")
    end = output.find("note: unset the PYO3_PRINT_CONFIG")
    check(start != -1 and end != -1, "failed to capture PYO3_PRINT_CONFIG output")
    return output[start:end]


def check_explicit_config() -> None:
    config = print_config()
    lines = []
    replaced = False
    for line in config.splitlines():
        stripped = line.strip()
        if stripped.startswith("lib_name="):
            lines.append("lib_name=python310")
            replaced = True
        elif stripped.startswith("-- PYO3_PRINT_CONFIG"):
            continue
        elif stripped.startswith("note:"):
            continue
        elif stripped:
            lines.append(stripped)
    check(replaced, "PYO3_PRINT_CONFIG output did not contain a lib_name line")

    with tempfile.TemporaryDirectory(prefix="pyo3-auto-detect-") as directory:
        config_path = Path(directory) / "pyo3-config"
        config_path.write_text("\n".join(lines) + "\n", encoding="utf-8")
        code, output = run_cargo(
            "--features",
            "auto-detect-python-lib",
            env={"PYO3_CONFIG_FILE": str(config_path)},
        )
    check(code == 0, "explicit config build failed:\n" + output)
    decision = parse_decision(output)
    check(
        decision["name"] == "python310",
        "explicit lib_name was not preferred: " + repr(decision["name"]),
    )
    check(
        decision["source"] == "configured",
        "explicit lib_name decision source is not `configured`: "
        + repr(decision["source"]),
    )
    print("ok: explicit PYO3_CONFIG_FILE lib_name takes priority")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--mode",
        choices=("cargo", "meson", "all"),
        default="cargo",
        help="which build tool to check",
    )
    args = parser.parse_args()

    print(f"platform: {platform.platform()}, python: {sys.version.split()[0]}")

    if args.mode in {"cargo", "all"}:
        before = git_status_porcelain()
        for check_fn in (
            check_disabled_path,
            check_enabled_path,
            check_abi3,
            check_conflict,
            check_explicit_config,
        ):
            check_fn()
        assert_resource_tree_unchanged(before, git_status_porcelain())

    if args.mode in {"meson", "all"}:
        print(
            "skip: meson checks are driven by the "
            "`examples/meson-starter` nox session in CI"
        )
        if args.mode == "meson":
            return 2

    print("all auto-detect-python-lib checks passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
