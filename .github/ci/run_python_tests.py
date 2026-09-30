#!/usr/bin/env python3
"""Run the Python hardening tests in isolated processes with this interpreter."""

import argparse
import os
import subprocess
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
SUITES = {
    "central": (".", ("-m", "unittest", "discover", "-s", ".github/ci/tests", "-t", ".github/ci")),
    "micro-report": (".", ("third_party/move/mono-move/testsuite/benches/perf/test_pr_ci_report.py",)),
    "e2e-report": (".", ("third_party/move/mono-move/testsuite/e2e-perf/test_pr_ci_report.py",)),
    "forge": ("testsuite", ("-m", "unittest", "forge_test")),
    "faucet-images": ("crates/aptos-faucet/integration-tests", ("-m", "unittest", "test_common_images")),
    "e2e-images": ("crates/aptos/e2e", ("-m", "unittest", "test_common_images")),
    "offline-images": ("crates/aptos/e2e", ("-m", "unittest", "test_offline_images")),
}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--suite", choices=SUITES, help="Run one group; the default runs all seven")
    args = parser.parse_args()
    # Check dependencies and the selected profile before starting any suite.
    from tests.property_support import configure_profiles

    configure_profiles()
    env = dict(os.environ)
    paths = [str(ROOT / ".github/ci")]
    if env.get("PYTHONPATH"):
        paths.append(env["PYTHONPATH"])
    env["PYTHONPATH"] = os.pathsep.join(paths)
    failed = False
    for name in (SUITES if args.suite is None else (args.suite,)):
        cwd, arguments = SUITES[name]
        print(f"\n=== Python hardening tests: {name} ===", flush=True)
        result = subprocess.run([sys.executable, *arguments], cwd=ROOT / cwd, env=env, check=False)
        failed |= result.returncode != 0
    return int(failed)


if __name__ == "__main__":
    sys.exit(main())
