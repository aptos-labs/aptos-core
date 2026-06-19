#!/usr/bin/env python3
# Copyright (c) Aptos Foundation
# Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

"""Extract the bundle name from a release config (e.g. framework-release.yaml).

The config's top-level `name` IS the bundle name (e.g. `aptos-framework-v1.45.1`).

Usage: bundle_name.py <release-config-path>

Emits two GitHub Actions step outputs (appended to the file named by
$GITHUB_OUTPUT, or printed to stdout when run locally):
  - bundle_name: the config's `name` verbatim (the bundle's identity)
  - version:     that name with the `aptos-framework-` prefix stripped (used as
                 the directory name under framework-releases/, e.g. `v1.45.1`)

This script handles framework releases only: it requires `name` to match
`aptos-framework-vX.Y.Z[-suffix]` and exits non-zero otherwise. The strict
pattern also guarantees `version` is a single safe path component (no separators
or shell metacharacters), since it is used as a directory name and interpolated
into the workflow.
"""

import os
import re
import sys

import yaml

# `aptos-framework-` + a version like v1.45.1 or v1.45.1-rc. `[\w.-]` excludes `/`
# and shell metacharacters, so the captured version is path- and shell-safe.
NAME_RE = re.compile(r"aptos-framework-(v\d+\.\d+\.\d+(?:-[\w.-]+)?)")


def main() -> int:
    if len(sys.argv) != 2:
        sys.exit("usage: bundle_name.py <release-config-path>")
    config_path = sys.argv[1]

    try:
        with open(config_path) as f:
            config = yaml.safe_load(f)
    except FileNotFoundError:
        sys.exit(f"release config not found: {config_path}")
    except yaml.YAMLError as e:
        sys.exit(f"failed to parse {config_path}: {e}")

    if not isinstance(config, dict):
        sys.exit(f"release config is not a YAML mapping: {config_path}")

    name = config.get("name")
    if not isinstance(name, str) or not name.strip():
        sys.exit(f"release config is missing a non-empty 'name': {config_path}")
    name = name.strip()

    match = NAME_RE.fullmatch(name)
    if not match:
        sys.exit(
            f"release config 'name' is not a framework release bundle name: {name!r} "
            f"(expected aptos-framework-vX.Y.Z[-suffix])"
        )

    outputs = {
        "bundle_name": name,
        # The framework-releases/ directory uses the version only.
        "version": match.group(1),
    }

    github_output = os.environ.get("GITHUB_OUTPUT")
    if github_output:
        with open(github_output, "a") as f:
            for key, value in outputs.items():
                f.write(f"{key}={value}\n")
    else:
        for key, value in outputs.items():
            sys.stdout.write(f"{key}={value}\n")

    return 0


if __name__ == "__main__":
    sys.exit(main())
