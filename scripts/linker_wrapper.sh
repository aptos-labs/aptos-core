#!/usr/bin/env bash
# Copyright (c) Aptos Foundation
# SPDX-License-Identifier: Apache-2.0

set -euo pipefail

# Supported values:
#   - system (default): clang default linker
#   - mold:             clang -fuse-ld=mold
#   - lld:              clang -fuse-ld=lld
LINKER_FLAVOR="${APTOS_LINKER:-system}"

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
