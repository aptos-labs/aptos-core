#!/usr/bin/env bash
# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0
#
# List the e2e baselines whose count of error lines differs from HEAD, and
# the new ones with theirs. `UB=1 lake test` accepts every output, so a
# regression shows up only here: a baseline that gained errors.
#
# Usage: scripts/exp-error-delta.sh [revision]   # default: HEAD
set -euo pipefail

REVISION="${1:-HEAD}"
cd "$(dirname "$0")/../leaner-e2e-tests"
PREFIX="$(git rev-parse --show-prefix)"

git diff --name-only "$REVISION" -- . | { grep '\.exp' || true; } | while read -r path; do
  file="${path#"$PREFIX"}"
  before="$(git show "$REVISION:$path" 2>/dev/null | grep -c error || true)"
  if [ -f "$file" ]; then after="$(grep -c error "$file" || true)"; else after=removed; fi
  if [ "$before" != "$after" ]; then echo "$file: $before -> $after"; fi
done
git ls-files --others --exclude-standard . | { grep '\.exp' || true; } | while read -r file; do
  echo "$file: new, $(grep -c error "$file" || true)"
done
