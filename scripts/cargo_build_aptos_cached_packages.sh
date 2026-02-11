#!/bin/sh

# This script rebuilds cached framework artifacts (head.mrb and SDK builder
# files). With --check it additionally verifies that the artifacts are fresh
# (no git diff), which is how CI uses it.

# cd to repo root (directory containing .github/).
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$REPO_ROOT" || exit 1

# Run in check mode if requested.
CHECK_ARG=""
if [ "$1" = "--check" ]; then
    CHECK_ARG="--check"
fi

# Set appropriate script flags
set -e
set -x

# Rebuild cached packages (head.mrb and SDK builder files).
cargo run -p aptos-framework -- update-cached-packages
if [ -n "$CHECK_ARG" ]; then
    if [ -n "$(git status --porcelain -uno aptos-move)" ]; then
      git diff
      echo ""
      echo "ERROR: Cached framework artifacts are out-of-date."
      echo "See aptos-move/framework/cached-packages/README.md for details."
      echo ""
      echo "To fix, run from anywhere in the repo:"
      echo "  scripts/cargo_build_aptos_cached_packages.sh"
      echo ""
      echo "Then commit the updated artifacts."
      exit 1
    fi
fi
