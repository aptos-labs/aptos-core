#!/usr/bin/env bash
set -euo pipefail

#############################################
# CONFIG
#############################################

SCRIPT_DIR="$(dirname $0)"

BIN_PATH=$1
TRACE_SCRIPT="$SCRIPT_DIR/trace.d"
FOLD_SCRIPT="$SCRIPT_DIR/fold.awk"

OUT_RAW=$(mktemp /tmp/dtrace-raw.XXXXXX)
OUT_FOLDED=$(mktemp /tmp/dtrace-folded.XXXXXX)
OUT_SVG="flame.svg"

#############################################
# LOGGING
#############################################

red()    { printf '\033[31m%s\033[0m\n' "$*" >&2; }
green()  { printf '\033[32m%s\033[0m\n' "$*" >&2; }
yellow() { printf '\033[33m%s\033[0m\n' "$*" >&2; }
blue()   { printf '\033[34m%s\033[0m\n' "$*" >&2; }

log()    { blue "==> $*"; }

#############################################
# TOOL CHECKING
#############################################

require() {
    if ! command -v "$1" >/dev/null 2>&1; then
        red "Error: '$1' not found in PATH"
        exit 1
    fi
}

require awk
require flamegraph.pl

#############################################
# CLEANUP
#############################################

TARGET_PID=""

cleanup() {
    # Kill the benchmark program if it's still alive
    if [[ -n "${TARGET_PID:-}" ]] && kill -0 "$TARGET_PID" 2>/dev/null; then
        yellow "Stopping benchmark (PID $TARGET_PID)…"
        kill "$TARGET_PID" 2>/dev/null || true
        wait "$TARGET_PID" 2>/dev/null || true
    fi

    # Remove temporary files
    [[ -f "${OUT_RAW:-}" ]] && rm -f "$OUT_RAW"
    [[ -f "${OUT_FOLDED:-}" ]] && rm -f "$OUT_FOLDED"
}
trap cleanup EXIT

#############################################
# MAIN WORKFLOW
#############################################

log "Starting benchmark with tracing…"
sudo dtrace -q -s "$TRACE_SCRIPT" -c "$BIN_PATH" > "$OUT_RAW"

log "Folding stack output…"
awk -f "$FOLD_SCRIPT" "$OUT_RAW" > "$OUT_FOLDED"

log "Generating flamegraph…"
flamegraph.pl "$OUT_FOLDED" > "$OUT_SVG"

green "Done!"
green "Flamegraph written to: $OUT_SVG"
