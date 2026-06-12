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
#   PLUGIN_TREE    Pre-generated plugin tree. When set, this script does NOT
#                  run the released binary — CI generates the tree in a
#                  separate, credential-free step. Without it, builds from source.
#   RELEASE_VERSION  Version to publish. Without it, read from Cargo.toml.
#   APTOS_CORE_SHA   SHA to record in the commit. Without it, uses HEAD.

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
FLOW_DIR="$REPO_ROOT/aptos-move/flow"
APTOS_AI="${1:-$HOME/aptos-ai}"

for cmd in gh git; do
    command -v "$cmd" >/dev/null 2>&1 || { echo "ERROR: $cmd not on PATH" >&2; exit 1; }
done

# Prefer the version resolved upstream (preflight); fall back to the crate.
VERSION="${RELEASE_VERSION:-}"
if [ -z "$VERSION" ]; then
    VERSION="$(awk -F'"' '/^\[/{s=$0} s=="[package]" && /^version *=/ {print $2; exit}' "$FLOW_DIR/Cargo.toml")"
fi
[ -n "$VERSION" ] || { echo "ERROR: could not determine aptos-move-flow version" >&2; exit 1; }

[ -d "$APTOS_AI/.git" ] || { echo "ERROR: $APTOS_AI is not a git repo" >&2; exit 1; }
[ -z "$(git -C "$APTOS_AI" status --porcelain)" ] \
    || { echo "ERROR: $APTOS_AI has uncommitted changes" >&2; exit 1; }

# Resolve the plugin tree. CI generates it in a separate, credential-free step
# (so the released binary never runs alongside the aptos-ai token) and passes
# it via PLUGIN_TREE. Locally we build and run from source.
if [ -n "${PLUGIN_TREE:-}" ]; then
    PLUGIN_SRC="$PLUGIN_TREE"
else
    command -v cargo >/dev/null 2>&1 \
        || { echo "ERROR: cargo not on PATH (needed to build from source; set PLUGIN_TREE to skip)" >&2; exit 1; }
    (cd "$REPO_ROOT" && cargo build -p aptos-move-flow --profile=ci)
    TMP=$(mktemp -d)
    trap 'rm -rf "$TMP"' EXIT
    "$REPO_ROOT/target/ci/move-flow" plugin "$TMP/plugin"
    PLUGIN_SRC="$TMP/plugin"
fi
[ -f "$PLUGIN_SRC/.claude-plugin/plugin.json" ] \
    || { echo "ERROR: plugin tree at $PLUGIN_SRC missing .claude-plugin/plugin.json" >&2; exit 1; }

cd "$APTOS_AI"
git checkout main
git pull --ff-only origin main
rm -rf plugins/move-flow
mkdir -p plugins/move-flow
cp -r "$PLUGIN_SRC/." plugins/move-flow/

BRANCH="move-flow/v${VERSION}"
# Populate the remote-tracking ref so --force-with-lease has a baseline.
git fetch origin "$BRANCH" 2>/dev/null || true

# Only an in-repo PR counts as "existing". A fork PR sharing this head branch
# name (isCrossRepository == true) must not suppress opening the real PR.
EXISTING_PR=$(gh pr list --repo aptos-labs/aptos-ai --head "$BRANCH" --state open \
    --json url,isCrossRepository \
    --jq 'map(select(.isCrossRepository == false)) | .[0].url // empty' 2>/dev/null || true)

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
