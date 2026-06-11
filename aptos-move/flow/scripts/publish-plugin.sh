#!/bin/bash
# Copyright (c) Aptos Foundation
# Licensed pursuant to the Innovation-Enabling Source Code License, available at
# https://github.com/aptos-labs/aptos-core/blob/main/LICENSE
#
# Publishes the MoveFlow Claude Code plugin to aptos-labs/aptos-ai.
#
# Usage: publish-plugin.sh [aptos-ai-repo-path]
#
# Env (optional, set by CI):
#   MOVE_FLOW_BIN    Prebuilt move-flow binary. Without it, builds from source.
#   APTOS_CORE_SHA   SHA to record in the commit. Without it, uses HEAD.

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
FLOW_DIR="$REPO_ROOT/aptos-move/flow"
APTOS_AI="${1:-$HOME/aptos-ai}"

for cmd in gh git; do
    command -v "$cmd" >/dev/null 2>&1 || { echo "ERROR: $cmd not on PATH" >&2; exit 1; }
done

# Read the first `version = "..."` in the [package] section. Faster than
# cargo metadata; aptos-move-flow's Cargo.toml uses an inline literal.
VERSION="$(awk -F'"' '/^\[/{s=$0} s=="[package]" && /^version *=/ {print $2; exit}' "$FLOW_DIR/Cargo.toml")"
[ -n "$VERSION" ] || { echo "ERROR: could not read aptos-move-flow version" >&2; exit 1; }

[ -d "$APTOS_AI/.git" ] || { echo "ERROR: $APTOS_AI is not a git repo" >&2; exit 1; }
[ -z "$(git -C "$APTOS_AI" status --porcelain)" ] \
    || { echo "ERROR: $APTOS_AI has uncommitted changes" >&2; exit 1; }

if [ -n "${MOVE_FLOW_BIN:-}" ]; then
    [ -x "$MOVE_FLOW_BIN" ] || { echo "ERROR: MOVE_FLOW_BIN=$MOVE_FLOW_BIN not executable" >&2; exit 1; }
    MOVE_FLOW="$MOVE_FLOW_BIN"
else
    command -v cargo >/dev/null 2>&1 \
        || { echo "ERROR: cargo not on PATH (needed to build from source; set MOVE_FLOW_BIN to skip)" >&2; exit 1; }
    (cd "$REPO_ROOT" && cargo build -p aptos-move-flow --profile=ci)
    MOVE_FLOW="$REPO_ROOT/target/ci/move-flow"
fi

TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT
"$MOVE_FLOW" plugin "$TMP/plugin"
[ -f "$TMP/plugin/.claude-plugin/plugin.json" ] \
    || { echo "ERROR: plugin generation failed" >&2; exit 1; }

cd "$APTOS_AI"
git checkout main
git pull --ff-only origin main
rm -rf plugins/move-flow
mkdir -p plugins/move-flow
cp -r "$TMP/plugin/." plugins/move-flow/

BRANCH="move-flow/v${VERSION}"
# Populate the remote-tracking ref so --force-with-lease has something to
# compare against; silent when the branch is new.
git fetch origin "$BRANCH" 2>/dev/null || true
EXISTING_PR=$(gh pr list --repo aptos-labs/aptos-ai --head "$BRANCH" --state open \
    --json url --jq '.[0].url // empty' 2>/dev/null || true)

git checkout -B "$BRANCH"
git add plugins/move-flow/
if git diff --cached --quiet; then
    echo "Plugin already up to date."
    exit 0
fi

SHA_SHORT="${APTOS_CORE_SHA:-$(git -C "$REPO_ROOT" rev-parse HEAD)}"
SHA_SHORT="${SHA_SHORT:0:12}"

git commit -m "Update move-flow plugin to v${VERSION}

Generated from aptos-core commit ${SHA_SHORT}."
git push -u origin "$BRANCH" --force-with-lease

if [ -z "$EXISTING_PR" ]; then
    gh pr create --repo aptos-labs/aptos-ai \
        --title "Update move-flow plugin to v${VERSION}" \
        --body "Regenerated move-flow plugin from aptos-core \`${SHA_SHORT}\` (v${VERSION})."
else
    echo "Updated existing PR: $EXISTING_PR"
fi
