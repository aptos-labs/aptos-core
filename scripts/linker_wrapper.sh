#!/usr/bin/env bash
# Copyright (c) Aptos Foundation
# SPDX-License-Identifier: Apache-2.0

set -euo pipefail

# Supported values:
#   - mold (default): clang -fuse-ld=mold
#   - lld:            clang -fuse-ld=lld
#   - system:         clang default linker
LINKER_FLAVOR="${APTOS_LINKER:-mold}"

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
