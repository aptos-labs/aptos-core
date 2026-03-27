#!/usr/bin/env bash
# Copyright (c) Aptos Foundation
# SPDX-License-Identifier: Apache-2.0

set -euo pipefail

# Supported values:
#   - mold:             clang -fuse-ld=mold
#   - lld:              clang -fuse-ld=lld
#   - system:           clang default linker
#
# If APTOS_LINKER is unset, auto-select in this order:
#   1) mold (if available)
#   2) lld/ld.lld (if available)
#   3) system linker
if [[ -n "${APTOS_LINKER:-}" ]]; then
  LINKER_FLAVOR="${APTOS_LINKER}"
else
  if command -v mold >/dev/null 2>&1; then
    LINKER_FLAVOR="mold"
  elif command -v lld >/dev/null 2>&1 || command -v ld.lld >/dev/null 2>&1; then
    LINKER_FLAVOR="lld"
  else
    LINKER_FLAVOR="system"
  fi
fi

case "${LINKER_FLAVOR}" in
lld)
  exec clang -fuse-ld=lld "$@"
  ;;
mold)
  exec clang -fuse-ld=mold "$@"
  ;;
system)
  exec clang "$@"
  ;;
*)
  echo "Unsupported APTOS_LINKER='${LINKER_FLAVOR}'. Use one of: mold, lld, system." >&2
  exit 2
  ;;
esac
