#!/usr/bin/env bash
#
# Publish a freshly built rustdoc tree to the gh-pages branch.
#
# The deploy is implemented as a single orphan commit so the gh-pages branch
# stays small even though the rustdoc payload is large (the workspace has
# hundreds of crates). Anything in $PRESERVE_PATHS is carried over from the
# previous gh-pages tip; everything else from the old tree is dropped — that
# is how the leftover Diem-era rustdoc gets cleaned up.
#
# Inputs (env):
#   DOC_DIR          rustdoc tree to publish (default: target/doc)
#   GITHUB_REPOSITORY  owner/repo (default: aptos-labs/aptos-core)
#   GH_PAGES_BRANCH  destination branch (default: gh-pages)
#   PRESERVE_PATHS   space-separated list of paths under gh-pages root to
#                    carry over from the previous tip (default: "move-book")
#   COMMIT_USER_NAME, COMMIT_USER_EMAIL
#                    git author/committer identity for the deploy commit
#   GITHUB_TOKEN     token with write access to the repo (required unless
#                    DRY_RUN=1 is set)
#   GITHUB_SHA       sha of the source commit being deployed (default: HEAD)
#   DRY_RUN          if non-empty, build the commit but skip the push
#
# Usage:
#   GITHUB_TOKEN=... bash .github/rust-docs/publish.sh
#   DRY_RUN=1 bash .github/rust-docs/publish.sh

set -euo pipefail

DOC_DIR="${DOC_DIR:-target/doc}"
GITHUB_REPOSITORY="${GITHUB_REPOSITORY:-aptos-labs/aptos-core}"
GH_PAGES_BRANCH="${GH_PAGES_BRANCH:-gh-pages}"
PRESERVE_PATHS="${PRESERVE_PATHS:-move-book}"
COMMIT_USER_NAME="${COMMIT_USER_NAME:-github-actions[bot]}"
COMMIT_USER_EMAIL="${COMMIT_USER_EMAIL:-41898282+github-actions[bot]@users.noreply.github.com}"
GITHUB_SHA="${GITHUB_SHA:-$(git rev-parse HEAD 2>/dev/null || echo unknown)}"
GITHUB_SHA_SHORT="$(printf '%s' "$GITHUB_SHA" | cut -c1-12)"
BUILD_TIME="$(date -u +'%Y-%m-%d %H:%M UTC')"

if [ ! -d "$DOC_DIR" ]; then
  echo "error: doc directory '$DOC_DIR' does not exist" >&2
  exit 1
fi

if [ -z "${GITHUB_TOKEN:-}" ] && [ -z "${DRY_RUN:-}" ]; then
  echo "error: GITHUB_TOKEN must be set (or pass DRY_RUN=1)" >&2
  exit 1
fi

DOC_DIR_ABS="$(cd "$DOC_DIR" && pwd)"
WORK_DIR="$(mktemp -d -t rustdoc-deploy.XXXXXX)"
PRESERVE_DIR="$(mktemp -d -t rustdoc-preserve.XXXXXX)"

cleanup() {
  rm -rf "$WORK_DIR" "$PRESERVE_DIR"
}
trap cleanup EXIT

if [ -n "${GITHUB_TOKEN:-}" ]; then
  CLONE_URL="https://x-access-token:${GITHUB_TOKEN}@github.com/${GITHUB_REPOSITORY}.git"
else
  CLONE_URL="https://github.com/${GITHUB_REPOSITORY}.git"
fi

echo "==> Fetching $GH_PAGES_BRANCH from $GITHUB_REPOSITORY to harvest preserved paths"
# Shallow clone is enough; we only need the current tip of gh-pages so we can
# copy out the directories we want to keep. If the branch doesn't exist yet
# (fresh-pages repo) we still create the destination from scratch.
if git clone --depth=1 --branch "$GH_PAGES_BRANCH" "$CLONE_URL" "$WORK_DIR" 2>/dev/null; then
  echo "==> Harvesting preserved paths: ${PRESERVE_PATHS}"
  for path in $PRESERVE_PATHS; do
    if [ -e "$WORK_DIR/$path" ]; then
      mkdir -p "$PRESERVE_DIR/$(dirname "$path")"
      cp -a "$WORK_DIR/$path" "$PRESERVE_DIR/$path"
      echo "    kept $path"
    else
      echo "    (not present on $GH_PAGES_BRANCH) $path"
    fi
  done
  rm -rf "$WORK_DIR"
else
  echo "==> $GH_PAGES_BRANCH does not exist yet; will create it"
fi

# Build the new tree as an orphan checkout. Using a fresh `git init` rather
# than reusing the cloned working tree means the deploy commit has no parent,
# which keeps the branch small even though we ship hundreds of MB of HTML.
echo "==> Staging new gh-pages tree"
mkdir -p "$WORK_DIR"
cd "$WORK_DIR"
git init --quiet --initial-branch="$GH_PAGES_BRANCH"
git config user.name "$COMMIT_USER_NAME"
git config user.email "$COMMIT_USER_EMAIL"
git remote add origin "$CLONE_URL"

# Copy the rustdoc output into the worktree root.
cp -a "$DOC_DIR_ABS/." .

# Restore preserved paths on top of the rustdoc tree. We restore *after* the
# copy so a preserved path always wins over any same-named rustdoc artifact;
# in practice rustdoc never produces e.g. a top-level `move-book/` directory,
# so there is no real collision risk, but be explicit anyway.
if [ -d "$PRESERVE_DIR" ]; then
  cp -a "$PRESERVE_DIR/." . 2>/dev/null || true
fi

# `.nojekyll` tells GitHub Pages to skip Jekyll processing — required because
# rustdoc emits files and directories whose names start with an underscore
# (e.g. `_static/`), which Jekyll would otherwise hide.
touch .nojekyll

echo "==> Committing and pushing"
git add -A
if git diff --cached --quiet; then
  echo "==> No changes to deploy."
  exit 0
fi
git commit --quiet -m "publish rustdoc from aptos-core ${GITHUB_SHA_SHORT} (${BUILD_TIME})"

if [ -n "${DRY_RUN:-}" ]; then
  echo "==> DRY_RUN set; skipping push."
  echo "    Staged commit lives in $WORK_DIR (cleaned on exit)."
  exit 0
fi

# Force-push: each deploy is a single orphan commit, so the branch's history
# is intentionally rewritten on every run.
git push --force origin "HEAD:${GH_PAGES_BRANCH}"

echo "==> Done."
echo "    https://${GITHUB_REPOSITORY%/*}.github.io/${GITHUB_REPOSITORY#*/}/"
