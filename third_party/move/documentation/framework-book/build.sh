#!/usr/bin/env bash
# Build the Aptos Framework Book.
#
#   1. Run the builder to generate per-module .md files into src/<pkg>/ and
#      assemble src/SUMMARY.md from src/SUMMARY.template.md.
#   2. Run mdbook to render src/ into html/.
set -euo pipefail
cd "$(dirname "$0")"
cargo run --release -p aptos-framework-book-builder
mdbook build
