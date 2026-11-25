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
NIGHTLY="nightly-2026-11-16"

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

# Packages Miri runs. `#[cfg_attr(miri, ignore = "<reason>")]` on a test
# excludes it from every pass, and the test output reports it as ignored with
# the reason.
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
  "mono-move-replay:links rocksdb; tests spawn worker processes and call libc (dup2, proc_pid_rusage)"
  "mono-move-replay-benchmark:links rocksdb, jemalloc, and zstd"
  "mono-move-replay-common:tests reach the Move compiler via mono-move-testsuite"
  "mono-move-lean-link:unit tests reach no unsafe code; integration tests compile Move source and shell out to cargo and nm"
  "vm-comparison:contains no unsafe code"
  "mono-move-unit-test-backend:unit tests reach no unsafe code; its unsafe sites run only on compiled Move packages"
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
    run_miri "stacked: Stacked Borrows + strict provenance" \
      -Zmiri-strict-provenance -Zmiri-symbolic-alignment-check -Zmiri-backtrace=full
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
