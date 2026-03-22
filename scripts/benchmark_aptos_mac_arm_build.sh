#!/usr/bin/env bash
# Compare `cargo build -p aptos` time with vs without `-C target-cpu=native` on Apple Silicon.
# Requires: macOS on arm64, repo root as cwd.
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel 2>/dev/null || true)"
if [[ -z "${ROOT}" ]]; then
  echo "Run from a git checkout of aptos-core."
  exit 1
fi
cd "${ROOT}"

if [[ "$(uname -s)" != Darwin ]] || [[ "$(uname -m)" != arm64 ]]; then
  echo "This benchmark is intended for macOS on Apple Silicon (arm64). Skipping."
  exit 0
fi

PKG=aptos
# Disable incremental so runs are comparable (optional; comment out for incremental-style timing).
export CARGO_INCREMENTAL=0

run_timed() {
  local label=$1
  shift
  local start end
  start=$(date +%s)
  "$@"
  end=$(date +%s)
  echo "${label}: $((end - start))s wall time"
}

echo "=== Baseline: force generic CPU (RUSTFLAGS after config; last target-cpu wins) ==="
cargo clean -p "${PKG}"
run_timed "generic" env RUSTFLAGS="-C target-cpu=generic ${RUSTFLAGS:-}" cargo build -p "${PKG}"

echo ""
echo "=== With native: use repo .cargo/config.toml (target-cpu=native for aarch64-apple-darwin) ==="
cargo clean -p "${PKG}"
run_timed "native" cargo build -p "${PKG}"
