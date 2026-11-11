#!/usr/bin/env bash
# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0
#
# Stop hook: when a turn changed Lean sources, build the modules it touched and
# report the result. This is the generation boundary the language server cannot
# supply: a file's diagnostics are only trustworthy while nothing it imports has
# been edited, so the one honest check of a cross-module change is a build.
#
# It reports and never blocks. A turn boundary is not a task boundary — an agent
# may stop to ask a question in the middle of a deliberately broken refactor —
# and a blocking hook would force it onward exactly then. Set
# VEREX_STOP_BUILD_BLOCKING=1 to turn a failure into a blocking error instead.
#
# Scope: only the files this session's own edits touched, recorded by
# claude-lean-edit-guard.sh. `git status` would include other agents' and the
# user's work; this worktree is shared.
#
# What it proves: that the touched modules elaborate, together with their
# imports. It does NOT rebuild their importers, so a change that breaks a caller
# of an edited module is not caught here — the package suites are that gate.

set -uo pipefail

LEAN_TREE="third_party/move/lean"
STATE_DIR="${XDG_CACHE_HOME:-$HOME/.cache}/verex-claude-hooks"
CORE_PIN="${VEREX_CORE_PIN:-0-17}"

note() { jq -n --arg m "$1" '{systemMessage:$m}'; }

payload="$(cat)"
session="$(printf '%s' "$payload" | jq -r '.session_id // empty')"
[[ -n "$session" ]] || exit 0

touched_list="$STATE_DIR/touched-$session"
[[ -s "$touched_list" ]] || exit 0

repo_root="$(git rev-parse --show-toplevel 2>/dev/null)" || exit 0

mapfile -t changed < <(sort -u "$touched_list" | while read -r f; do
  [[ -f "$f" ]] || continue
  rel="$(realpath --relative-to="$repo_root" "$f" 2>/dev/null)" || continue
  case "$rel" in
    "$LEAN_TREE"/v0/*) ;;
    "$LEAN_TREE"/*) printf '%s\n' "$rel" ;;
  esac
done)
(( ${#changed[@]} == 0 )) && exit 0

# One report per tree state: the same content is never reported twice.
mkdir -p "$STATE_DIR"
state="$(for f in "${changed[@]}"; do
  printf '%s %s\n' "$f" "$(sha1sum "$repo_root/$f" 2>/dev/null | cut -d' ' -f1)"
done | sha1sum | cut -d' ' -f1)"
stamp="$STATE_DIR/stop-build-$state"
[[ -f "$stamp" ]] && exit 0

# Never build beside another build or a running suite. The language server's own
# `lean --server` / `lean --worker` processes are not builds and must not count,
# or this hook would be disabled the moment the editor is in use.
busy=""
pgrep -x lake >/dev/null 2>&1 && busy="lake"
pgrep -x verex-move >/dev/null 2>&1 && busy="verex-move"
if pgrep -a -x lean 2>/dev/null | grep -qv -e '--server' -e '--worker'; then
  busy="lean"
fi
if [[ -n "$busy" ]]; then
  note "Lean stop-build skipped: a ${busy} process is already running."
  exit 0
fi

declare -A targets=()
for f in "${changed[@]}"; do
  rest="${f#"$LEAN_TREE"/}"
  pkg="${rest%%/*}"
  inner="${rest#*/}"
  [[ -f "$repo_root/$LEAN_TREE/$pkg/lakefile.toml" \
     || -f "$repo_root/$LEAN_TREE/$pkg/lakefile.lean" ]] || continue
  module="${inner%.lean}"
  targets["$pkg"]+="${module//\//.} "
done
(( ${#targets[@]} == 0 )) && exit 0

failed=() built=() skipped=()
log="$(mktemp)"
trap 'rm -f "$log"' EXIT

for pkg in "${!targets[@]}"; do
  pkg_dir="$repo_root/$LEAN_TREE/$pkg"
  # A package that was never built needs a full cold build; that is the user's
  # decision, not a side effect of finishing a turn.
  if [[ -z "$(find "$pkg_dir/.lake" -name '*.olean' -print -quit 2>/dev/null)" ]]; then
    skipped+=("$pkg (not built yet)")
    continue
  fi
  read -r -a mods <<<"${targets[$pkg]}"
  if (cd "$pkg_dir" && taskset -c "$CORE_PIN" lake build "${mods[@]}") >>"$log" 2>&1; then
    built+=("$pkg: ${mods[*]}")
  else
    failed+=("$pkg: ${mods[*]}")
  fi
done

if (( ${#failed[@]} == 0 )); then
  summary=""
  (( ${#built[@]} > 0 )) && summary="${#built[@]} module set(s) elaborate."
  if (( ${#skipped[@]} > 0 )); then
    summary+="${summary:+ }Not checked: ${skipped[*]}."
  fi
  [[ -n "$summary" ]] && note "Lean stop-build: $summary"
  exit 0
fi

: >"$stamp"
message="Lean stop-build FAILED — this turn changed Lean sources that do not elaborate: ${failed[*]}"
if [[ "${VEREX_STOP_BUILD_BLOCKING:-}" == "1" ]]; then
  { echo "$message"; echo; tail -40 "$log"; } >&2
  exit 2
fi
jq -n --arg m "$message" --arg log "$(tail -20 "$log")" \
  '{systemMessage:($m + "\n" + $log)}'
exit 0
