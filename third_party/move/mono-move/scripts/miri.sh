#!/usr/bin/env bash
#
# Run the MonoMove test suites under Miri.
#
#   ./scripts/miri.sh setup         install the pinned nightly + miri, build sysroot
#   ./scripts/miri.sh check         verify every package is classified, then exit
#   ./scripts/miri.sh stacked       Stacked Borrows + strict provenance  [default]
#   ./scripts/miri.sh concurrency   many seeds, raised preemption
#   ./scripts/miri.sh tree          Tree Borrows
#
# Passes are never composed: each multiplies the suite by its own factor, and
# Stacked Borrows and Tree Borrows are mutually exclusive. All keep Miri's
# defaults (isolation, leak, validity, weak-memory, data-race).
#
# Run passes sequentially. Miri's sysroot is a machine-global cache independent
# of CARGO_TARGET_DIR; concurrent invocations race on it and fail mid-run with
# "found crate `std` compiled by an incompatible version of rustc".
#
# No randomized-layout pass: `-Zrandomize-layout` changes padding, which breaks
# the deliberate `size_of::<Instr<_>>() == 32` pin in `specializer` at compile
# time.
#
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MONO_MOVE="$(dirname "$HERE")"
REPO_ROOT="$(cd "$MONO_MOVE/../../.." && pwd)"

# Miri is nightly-only, and rust-toolchain.toml pins stable. Pinned by date so
# local and CI runs get the same Miri; bump when rust-toolchain.toml does, so
# this stays ahead of stable.
NIGHTLY="nightly-2026-06-23"

# Miri artifacts are incompatible with the normal build cache; a separate
# directory keeps the two from evicting each other on every switch.
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$REPO_ROOT/target/miri}"

# .cargo/config.toml sets `-C target-cpu=x86-64-v3` for x86_64 linux, putting
# dependencies on AVX2 paths Miri cannot fully interpret. RUSTFLAGS overrides
# every config source, so setting it here turns that off.
export RUSTFLAGS="${RUSTFLAGS:---cfg tokio_unstable}"

# Property-test budget. Miri's isolation hides the environment from the program,
# so each variable must be BOTH set here and forwarded below, or it is silently
# ignored and every property test runs proptest's full default case count, which
# does not finish under Miri.
#
# PROPTEST_DISABLE_FAILURE_PERSISTENCE is separately required: proptest's default
# failure persistence resolves a source path through `env::current_dir`, and
# `getcwd` is unavailable under isolation, which aborts the run outright.
# Forwarding two named variables is the minimal fix; do not reach for
# -Zmiri-disable-isolation.
export PROPTEST_CASES="${PROPTEST_CASES:-2}"
export PROPTEST_DISABLE_FAILURE_PERSISTENCE=1
FORWARD=(
  -Zmiri-env-forward=PROPTEST_CASES
  -Zmiri-env-forward=PROPTEST_DISABLE_FAILURE_PERSISTENCE
)

# Packages Miri runs.
RUN_PACKAGES=(
  mono-move-alloc
  mono-move-core
  mono-move-global-context
  mono-move-natives
  mono-move-runtime
)

# Packages Miri deliberately does not run, and why. Keeping the reason here
# means an exclusion cannot become invisible: `check_coverage` below fails if a
# package under mono-move/ appears in neither list, so a new crate cannot
# silently escape Miri. It can only be deliberately excluded.
SKIP_PACKAGES=(
  "specializer:contains no unsafe code"
  "mono-move-orchestrator:has no tests"
  "mono-move-loader:tests reach the Move compiler via mono-move-testsuite"
  "mono-move-testsuite:differential suite compiles Move source per case"
  "mono-move-output:1 test against 3 unsafe sites"
  "mono-move-aptos-state-view-providers:has no tests"
  "mono-move-aptos-transaction-executor:e2e tests need genesis and a 64MiB arena"
  "mono-move-replay-benchmark:links rocksdb, jemalloc, and zstd"
)

# Fail if a package under mono-move/ is in neither list. Without this, a new
# crate silently escapes Miri instead of being deliberately excluded.
check_coverage() {
  local classified actual
  classified="$(
    printf '%s\n' "${RUN_PACKAGES[@]}"
    printf '%s\n' "${SKIP_PACKAGES[@]%%:*}"
  )"
  actual="$(
    cargo metadata --format-version 1 --no-deps --manifest-path "$REPO_ROOT/Cargo.toml" \
      | python3 -c '
import json, sys
meta = json.load(sys.stdin)
for pkg in meta["packages"]:
    if "/mono-move/" in pkg["manifest_path"]:
        print(pkg["name"])
'
  )"
  local missing
  missing="$(comm -13 <(sort -u <<<"$classified") <(sort -u <<<"$actual"))"
  if [[ -n "$missing" ]]; then
    echo "error: package(s) under mono-move/ are neither run nor skipped:" >&2
    sed 's/^/  /' <<<"$missing" >&2
    echo "Add each to RUN_PACKAGES or to SKIP_PACKAGES with a reason." >&2
    return 1
  fi
  echo "package coverage ok: ${#RUN_PACKAGES[@]} run, ${#SKIP_PACKAGES[@]} skipped"
}

run_miri() {
  local label="$1"
  shift
  local flags=("$@")
  echo
  echo "=== $label ==="
  echo "MIRIFLAGS: ${flags[*]} ${FORWARD[*]}"
  local package_args=()
  for pkg in "${RUN_PACKAGES[@]}"; do
    package_args+=(-p "$pkg")
  done
  MIRIFLAGS="${flags[*]} ${FORWARD[*]}" \
    cargo "+$NIGHTLY" miri test "${package_args[@]}" --locked --no-fail-fast
}

# Run one cargo invocation with an explicit flag string and target selection.
run_targets() {
  local flags="$1"
  shift
  MIRIFLAGS="$flags ${FORWARD[*]}" \
    cargo "+$NIGHTLY" miri test --locked --no-fail-fast "$@"
}

# The stacked pass cannot cover every target: `parking_lot`'s contended slow path
# casts an integer to a pointer (parking_lot_core word_lock.rs), which strict
# provenance rejects outright. That is dependency code, so those targets are run
# with the same flags minus strict provenance and the reduced coverage is
# reported rather than the flag being dropped globally.
#
# The listed targets are the ones that lock a shared `parking_lot::Mutex` from
# several threads released together by a `Barrier`. The cast only fires on
# schedules that actually contend the mutex, so a single default-seed run is not
# enough to find a new one; re-derive this list with the strict flags plus
# -Zmiri-many-seeds. As of the last sweep the package's other targets
# (context_tests, subst_tests) and mono-move-alloc were clean across 12 seeds.
STRICT_PROVENANCE_GAP_PKG="mono-move-global-context"
STRICT_PROVENANCE_GAP_TARGETS=(
  identifiers_tests
  module_id_tests
)

# Integration-test target names for a package, from cargo metadata. Discovered
# rather than listed so a newly added target cannot silently skip a pass.
package_test_targets() {
  cargo metadata --format-version 1 --no-deps --manifest-path "$REPO_ROOT/Cargo.toml" \
    | python3 -c '
import json, sys
name = sys.argv[1]
meta = json.load(sys.stdin)
for pkg in meta["packages"]:
    if pkg["name"] == name:
        for target in pkg["targets"]:
            if target["kind"] == ["test"]:
                print(target["name"])
' "$1"
}

run_stacked() {
  local strict="-Zmiri-strict-provenance -Zmiri-symbolic-alignment-check -Zmiri-backtrace=full"
  local relaxed="-Zmiri-symbolic-alignment-check -Zmiri-backtrace=full"
  echo
  echo "=== stacked: Stacked Borrows + strict provenance ==="

  # Resolve targets before running anything, so a stale gap name fails in
  # seconds rather than after the rest of the pass.
  local gap_list seen="" strict_targets=(--lib) relaxed_targets=()
  gap_list="$(printf '%s\n' "${STRICT_PROVENANCE_GAP_TARGETS[@]}")"
  while read -r target; do
    if grep -qxF "$target" <<<"$gap_list"; then
      relaxed_targets+=(--test "$target")
      seen+="$target"$'\n'
    else
      strict_targets+=(--test "$target")
    fi
  done < <(package_test_targets "$STRICT_PROVENANCE_GAP_PKG")

  local stale
  stale="$(comm -23 <(sort -u <<<"$gap_list") <(sort -u <<<"$seen"))"
  if [[ -n "$stale" ]]; then
    echo "error: ${STRICT_PROVENANCE_GAP_PKG} has no target(s) named:" >&2
    sed 's/^/  /' <<<"$stale" >&2
    echo "Update STRICT_PROVENANCE_GAP_TARGETS." >&2
    return 1
  fi

  local package_args=()
  for pkg in "${RUN_PACKAGES[@]}"; do
    [[ "$pkg" == "$STRICT_PROVENANCE_GAP_PKG" ]] && continue
    package_args+=(-p "$pkg")
  done
  run_targets "$strict" "${package_args[@]}"

  run_targets "$strict" -p "$STRICT_PROVENANCE_GAP_PKG" "${strict_targets[@]}"

  echo
  echo "note: these ${STRICT_PROVENANCE_GAP_PKG} targets run WITHOUT strict"
  echo "      provenance: parking_lot casts int->ptr when contended."
  printf '        %s\n' "${STRICT_PROVENANCE_GAP_TARGETS[@]}"
  run_targets "$relaxed" -p "$STRICT_PROVENANCE_GAP_PKG" "${relaxed_targets[@]}"
}

# Concurrency exploration is scoped to the targets that actually spawn threads,
# and excludes the property-test targets: seeds and property cases multiply, and
# the product does not finish under Miri.
run_concurrency() {
  local flags="$*"
  echo
  echo "=== concurrency: many seeds ==="
  run_targets "$flags" -p mono-move-alloc
  run_targets "$flags" -p mono-move-global-context
}

case "${1:-stacked}" in
  setup)
    rustup toolchain install "$NIGHTLY" --component miri,rust-src
    cargo "+$NIGHTLY" miri setup
    ;;
  check)
    check_coverage
    ;;
  stacked)
    check_coverage
    run_stacked
    ;;
  concurrency)
    check_coverage
    run_concurrency -Zmiri-many-seeds=0..16 -Zmiri-preemption-rate=0.1 \
      -Zmiri-symbolic-alignment-check
    ;;
  tree)
    check_coverage
    run_miri "tree: Tree Borrows" \
      -Zmiri-tree-borrows -Zmiri-tree-borrows-implicit-writes \
      -Zmiri-symbolic-alignment-check -Zmiri-backtrace=full
    ;;
  *)
    echo "unknown pass: $1" >&2
    sed -n '5,9p' "${BASH_SOURCE[0]}" >&2
    exit 2
    ;;
esac
