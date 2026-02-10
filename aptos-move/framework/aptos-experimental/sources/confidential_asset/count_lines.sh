#!/bin/bash
# Counts Move lines in the confidential_asset code, separated by sources and tests.

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"

sources_lines=0
tests_lines=0

echo "=== Sources ==="
for f in $(find "$ROOT_DIR/sources/confidential_asset" -name '*.move' | sort); do
    n=$(wc -l < "$f")
    printf "%6d  %s\n" "$n" "${f#$ROOT_DIR/}"
    sources_lines=$((sources_lines + n))
done
printf "%6d  TOTAL (sources)\n" "$sources_lines"

echo ""
echo "=== Tests ==="
for f in $(find "$ROOT_DIR/tests/confidential_asset" -name '*.move' | sort); do
    n=$(wc -l < "$f")
    printf "%6d  %s\n" "$n" "${f#$ROOT_DIR/}"
    tests_lines=$((tests_lines + n))
done
printf "%6d  TOTAL (tests)\n" "$tests_lines"

echo ""
total=$((sources_lines + tests_lines))
echo "=== Summary ==="
printf "%6d  sources\n" "$sources_lines"
printf "%6d  tests\n" "$tests_lines"
printf "%6d  TOTAL\n" "$total"
