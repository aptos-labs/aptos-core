#!/usr/bin/env python3
"""Run the production selector, with an explicit package override for CI benchmarks."""

import argparse
import json
import os
import subprocess
from pathlib import Path


def package_args(names, metadata):
    packages = {package["name"]: package for package in metadata["packages"]}
    result = []
    for name in names:
        package = packages[name]
        # aptos-cargo-cli expects a package name after '#', not a Cargo version.
        path = Path(package["manifest_path"]).parent.as_uri()
        result.extend(["-p", f"{path}#{name}"])
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mode", choices=["run", "archive"])
    parser.add_argument("args", nargs=argparse.REMAINDER)
    args = parser.parse_args()
    names = os.environ.get("CI_BENCHMARK_PACKAGES", "").split()
    selected = []
    if names:
        metadata = json.loads(
            subprocess.check_output(
                ["cargo", "metadata", "--locked", "--no-deps", "--format-version=1"],
                text=True,
            )
        )
        selected = package_args(names, metadata)
    command = "targeted-unit-tests" + ("-archive" if args.mode == "archive" else "")
    forwarded = args.args[1:] if args.args[:1] == ["--"] else args.args
    return subprocess.call(["cargo", "x", *selected, command, "-vvv", *forwarded])


if __name__ == "__main__":
    raise SystemExit(main())
