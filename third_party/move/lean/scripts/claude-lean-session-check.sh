#!/usr/bin/env bash
# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0
#
# SessionStart hook: report which Verex packages of this worktree are built.
# The Lean language server resolves a file's imports from built `.olean`
# files and never builds them itself, so in an unbuilt worktree every
# LSP-based query answers with import errors. Stating that once, at the start,
# saves an agent from diagnosing it as a code problem.

set -uo pipefail

LEAN_TREE="third_party/move/lean"
PACKAGES=(verex-ir verex-move verex-rust verex-e2e-tests)

repo_root="$(git rev-parse --show-toplevel 2>/dev/null)" || exit 0
[[ -d "$repo_root/$LEAN_TREE" ]] || exit 0

built=() unbuilt=()
for pkg in "${PACKAGES[@]}"; do
  pkg_dir="$repo_root/$LEAN_TREE/$pkg"
  [[ -d "$pkg_dir" ]] || continue
  if [[ -n "$(find "$pkg_dir/.lake/" -name '*.olean' -print -quit 2>/dev/null)" ]]; then
    built+=("$pkg")
  else
    unbuilt+=("$pkg")
  fi
done

(( ${#unbuilt[@]} == 0 )) && exit 0

context="Verex Lean packages not built in this worktree: ${unbuilt[*]}."
if (( ${#built[@]} > 0 )); then
  context+=" Built: ${built[*]}."
fi
context+=" The Lean language server reads imports from built .olean files and"
context+=" does not build them, so lean_diagnostic_messages, lean_goal and"
context+=" lean_hover_info on a file of an unbuilt package report import errors"
context+=" rather than real problems. Build the package you are working in"
context+=" (cd ${LEAN_TREE}/<pkg> && lake build) before relying on them."

jq -n --arg c "$context" \
  '{hookSpecificOutput:{hookEventName:"SessionStart",additionalContext:$c}}'
