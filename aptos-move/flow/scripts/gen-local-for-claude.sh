#!/usr/bin/env bash
# Copyright (c) Aptos Foundation
# Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE
#
# Regenerate the local gen/claude/ plugin directory.

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
CRATE_DIR="$REPO_ROOT/aptos-move/flow"
OUTPUT_DIR="$CRATE_DIR/gen/claude"

# Parse arguments
LOG_ARGS=()
PROFILE_ARGS=()
while [[ $# -gt 0 ]]; do
    case "$1" in
        --log)
            LOG_ARGS=(--log "$2")
            shift 2
            ;;
        --debug)
            PROFILE_ARGS=(--debug)
            shift
            ;;
        *)
            echo "Usage: $0 [--log <file>] [--debug]" >&2
            exit 1
            ;;
    esac
done

# Install move-flow
echo "Installing move-flow ..."
cargo install --path "$CRATE_DIR" --locked "${PROFILE_ARGS[@]}"

# Generate plugin output
echo "Generating plugin files in $OUTPUT_DIR..."
move-flow plugin "$OUTPUT_DIR" "${LOG_ARGS[@]}"

echo "Done. Run Claude with:"
echo "  claude --plugin $OUTPUT_DIR"
