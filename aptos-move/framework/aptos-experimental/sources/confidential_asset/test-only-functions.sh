#!/bin/bash
# Lists all #[test_only] functions and #[test] functions remaining in sources/confidential_asset/.
# Excludes .spec.move files, friend declarations, and use statements.

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

echo "=== #[test_only] functions ==="
grep -rn '#\[test_only\]' "$SCRIPT_DIR" --include='*.move' -A4 \
    | grep -v '\.spec\.' \
    | grep 'fun ' \
    | sed "s|$SCRIPT_DIR/||"

echo ""
echo "=== #[test] functions ==="
grep -rn '#\[test\]$' "$SCRIPT_DIR" --include='*.move' -A4 \
    | grep -v '\.spec\.' \
    | grep 'fun ' \
    | sed "s|$SCRIPT_DIR/||"

echo ""
echo "=== #[test_only] structs/consts ==="
grep -rn '#\[test_only\]' "$SCRIPT_DIR" --include='*.move' -A1 \
    | grep -v '\.spec\.' \
    | grep 'struct \|const ' \
    | sed "s|$SCRIPT_DIR/||"
