#!/usr/bin/env bash
set -euo pipefail
# Native compiler replacement at the same path must not reuse older build outputs.
abi=$(
  { rustc -vV; cargo --version; clang --version; clang++ --version; ld.lld --version;
    for tool in clang clang++ ld.lld; do sha256sum "$(readlink -f "$(command -v "$tool")")"; done
    env | LC_ALL=C sort | grep -E '^(CARGO|RUST|CC|CXX|CFLAGS|CMAKE|PROFILE|FEATURES)' || true;
  } | sha256sum | cut -d' ' -f1
)
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-target}/abi-${abi}"
# Restored Cargo fingerprints can be newer than an older checkout's source files.
marker="$CARGO_TARGET_DIR/.aptos-source-identity"
mkdir -p "$(dirname "$marker")"
identity=$(
  find . -type d \( -name target -o -name .git -o -name dist \) -prune -o -type f \
    \( -name '*.rs' -o -name '*.toml' -o -name '*.lock' -o -name '*.c' -o -name '*.h' -o -name '*.cpp' -o -name '*.sh' \) -print0 |
    sort -z | xargs -0 sha256sum | sha256sum | cut -d' ' -f1
)
if [[ ! -f "$marker" || "$(<"$marker")" != "$identity" ]]; then
  find . -type d \( -name target -o -name .git -o -name dist \) -prune -o -type f \
    \( -name '*.rs' -o -name '*.toml' -o -name '*.lock' -o -name '*.c' -o -name '*.h' -o -name '*.cpp' -o -name '*.sh' \) -exec touch {} +
  printf '%s\n' "$identity" > "$marker"
fi
