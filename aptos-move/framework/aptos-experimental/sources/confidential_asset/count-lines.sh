#!/bin/bash
# Counts Move lines in the confidential_asset code, separated by sources and tests.

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
APTOS_CORE_DIR="$(cd "$ROOT_DIR/../../.." && pwd)"

total_sources_lines=0
total_tests_lines=0

echo "=== Move sources ==="
sources_lines=0
for f in $(find "$ROOT_DIR/sources/confidential_asset" -name '*.move' | sort); do
    n=$(wc -l < "$f")
    printf "%6d  %s\n" "$n" "${f#$ROOT_DIR/}"
    sources_lines=$((sources_lines + n))
done
printf "%6d  TOTAL (sources)\n" "$sources_lines"
total_sources_lines=$((total_sources_lines + sources_lines))

echo ""
echo "=== Move tests ==="
tests_lines=0
for f in $(find "$ROOT_DIR/tests/confidential_asset" -name '*.move' | sort); do
    n=$(wc -l < "$f")
    printf "%6d  %s\n" "$n" "${f#$ROOT_DIR/}"
    tests_lines=$((tests_lines + n))
done
printf "%6d  TOTAL (tests)\n" "$tests_lines"
total_tests_lines=$((total_tests_lines + tests_lines))

echo ""
echo "=== e2e-move-tests/ ==="
tests_lines=0
for f in "$APTOS_CORE_DIR/aptos-move/e2e-move-tests/src/tests/confidential_asset.rs"; do
    n=$(wc -l < "$f")
    printf "%6d  %s\n" "$n" "${f#$ROOT_DIR/}"
    tests_lines=$((tests_lines + n))
done
printf "%6d  TOTAL (tests)\n" "$tests_lines"
total_tests_lines=$((total_tests_lines + tests_lines))

echo ""
echo "=== move-examples/ ==="
tests_lines=0
for f in $(find "$APTOS_CORE_DIR/aptos-move/move-examples/confidential_asset/tests/" -name '*.move' | sort); do
    n=$(wc -l < "$f")
    printf "%6d  %s\n" "$n" "${f#$ROOT_DIR/}"
    tests_lines=$((tests_lines + n))
done
printf "%6d  TOTAL (tests)\n" "$tests_lines"
total_tests_lines=$((total_tests_lines + tests_lines))

echo ""
total=$((sources_lines + tests_lines))
echo "=== Summary ==="
printf "%6d  sources\n" "$total_sources_lines"
printf "%6d  tests\n" "$total_tests_lines"
printf "%6d  TOTAL\n" "$total"
