#!/usr/bin/env bash
# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0
#
# PostToolUse guard for Lean edits made by an agent. It reads the hook payload
# on stdin and checks only what can be decided from the text of one file: the
# tree's licence header, the prohibition on new admissions, and the files that
# are generated or deprecated and so must not be hand-edited.
#
# It deliberately does NOT elaborate or typecheck. A cold `lake env lean` run
# costs tens of seconds to minutes per edit here, and during a cross-module
# change the intermediate states are expected to be broken, so a per-edit
# typecheck would both tax and mislead. Typechecking belongs in the editing
# loop (the language server) and at a generation boundary (the Stop hook).

set -uo pipefail

STATE_DIR="${XDG_CACHE_HOME:-$HOME/.cache}/verex-claude-hooks"

payload="$(cat)"
file="$(printf '%s' "$payload" | jq -r '
  .tool_response.filePath // .tool_input.file_path // empty')"

[[ -n "$file" && "$file" == *.lean ]] || exit 0
[[ "$file" == */.lake/* ]] && exit 0
[[ -f "$file" ]] || exit 0

# Only files inside the repository are governed by the tree's rules. A scratch
# file outside it is experiment, not source: the leaf-debug recipe deliberately
# writes scratch proof files with `all_goals sorry`.
repo_root="$(git -C "$(dirname "$file")" rev-parse --show-toplevel 2>/dev/null)" || exit 0
[[ -n "$repo_root" ]] || exit 0

session="$(printf '%s' "$payload" | jq -r '.session_id // empty')"
if [[ -n "$session" ]]; then
  mkdir -p "$STATE_DIR"
  printf '%s\n' "$file" >>"$STATE_DIR/touched-$session"
fi

problems=()

case "$file" in
  */third_party/move/lean/v0/*)
    problems+=("\`v0/\` is the deprecated pre-verex tree: reference only. Do not add\
 functionality or tests there (third_party/move/lean/CLAUDE.md, \"Rules of the tree\").")
    ;;
esac

case "$file" in
  */.verex/*|*/build/verex/*|*/build/verex-verify.lean)
    problems+=("This is a generated rendering, not source. Change it through the\
 owning generator instead of editing it by hand.")
    ;;
esac

# The Lean tree carries the Apache header, never the Innovation-Enabling one
# used elsewhere in aptos-core.
if [[ "$file" == */third_party/move/lean/* || "$file" == *.proof.lean ]]; then
  head_text="$(head -8 "$file")"
  if ! grep -qF 'Copyright © Aptos Foundation' <<<"$head_text" \
     || ! grep -qF 'SPDX-License-Identifier: Apache-2.0' <<<"$head_text"; then
    problems+=("Missing the Apache header. Every file in this tree starts with\
 \`-- Copyright © Aptos Foundation\` and \`-- SPDX-License-Identifier: Apache-2.0\`.")
  fi
  if grep -qF 'Innovation-Enabling' <<<"$head_text"; then
    problems+=("This tree uses the Apache header, not the Innovation-Enabling one.")
  fi
fi

# New admissions. Counted against the committed version so that pre-existing
# ones in a file are not reported as the agent's.
count_admissions() { grep -cE '(^|[^[:alnum:]_])(sorry|native_decide|axiom)([^[:alnum:]_]|$)' || true; }
now="$(count_admissions <"$file")"
rel="$(realpath --relative-to="$repo_root" "$file" 2>/dev/null)"
before="$(git -C "$repo_root" show "HEAD:$rel" 2>/dev/null | count_admissions)"
[[ -n "$before" ]] || before=0
if (( now > before )); then
  problems+=("This edit adds $(( now - before )) admission(s) (\`sorry\`, \`axiom\` or\
 \`native_decide\`). The tree forbids them unless the design models the assumption\
 and the change was requested — raise it instead of admitting it.")
fi

(( ${#problems[@]} == 0 )) && exit 0

{
  echo "Lean edit guard — ${file}:"
  for p in "${problems[@]}"; do echo "  - ${p}"; done
} >&2
exit 2
