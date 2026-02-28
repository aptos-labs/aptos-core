#!/usr/bin/env bash
# Copyright (c) Aptos Foundation
# Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE
#
# Regenerate the local gen/claude/ plugin directory.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
OUTPUT_DIR="$SCRIPT_DIR/gen/claude"

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
cargo install --path "$SCRIPT_DIR" --locked "${PROFILE_ARGS[@]}"

# Generate plugin output
echo "Generating plugin files in $OUTPUT_DIR..."
move-flow plugin "$OUTPUT_DIR" "${LOG_ARGS[@]}"

echo "Done."
