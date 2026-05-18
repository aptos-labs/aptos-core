#!/usr/bin/env bash
#
# Deploy the Move book to the Aptos Labs org-level GitHub Pages site.
#
# Builds the mdbook output and pushes it into a subdirectory of the
# `aptos-labs/aptos-labs.github.io` repository. After the push, GitHub Pages
# rebuilds automatically and the book becomes available at
#
#     https://aptos-labs.github.io/<SUBPATH>/
#
# The org Pages repo also hosts origin-level files (`index.html`, `robots.txt`,
# `sitemap.xml`, `llms.txt`); this script does not touch them — it only
# replaces `<SUBPATH>/`.
#
# By default this script refuses to deploy a HEAD that hasn't been merged into
# the upstream branch — otherwise the commit hash baked into the published page
# would point at a private commit and be useless for anyone who lands on it.
# Pass --debug to bypass that check (e.g., when iterating on the book locally
# before the source PR has merged).
#
# Configuration via environment variables:
#   PAGES_REPO       git URL or local path to aptos-labs.github.io
#                    (default: https://github.com/aptos-labs/aptos-labs.github.io.git)
#   BRANCH           branch to push (default: main)
#   SUBPATH          subdirectory under the org repo (default: move-book)
#   UPSTREAM_REMOTE  remote to validate HEAD against
#                    (auto: `upstream` if present, else `origin`)
#   UPSTREAM_BRANCH  branch on that remote (default: main)
#   DRY_RUN          if non-empty, build and stage but skip the push
#
# Usage:
#   ./deploy.sh                                 # build, commit, push
#   ./deploy.sh --debug                         # skip the merged-upstream check
#   DRY_RUN=1 ./deploy.sh                       # stage only; no push
#   PAGES_REPO=~/aptos-labs.github.io ./deploy.sh   # use a local checkout

set -euo pipefail

DEBUG=""
for arg in "$@"; do
  case "$arg" in
    --debug) DEBUG=1 ;;
    -h|--help)
      sed -nE 's/^# ?//p' "$0" | sed -n '2,40p'
      exit 0
      ;;
    *)
      echo "error: unknown argument: $arg (try --help)" >&2
      exit 1
      ;;
  esac
done

PAGES_REPO="${PAGES_REPO:-https://github.com/aptos-labs/aptos-labs.github.io.git}"
BRANCH="${BRANCH:-main}"
# `${SUBPATH-move-book}` defaults only when SUBPATH is unset; an explicit
# empty SUBPATH= falls through to the guard below and is rejected. We never
# want to publish at the org root because that would clobber the landing
# page (`index.html`, `robots.txt`, etc.) the org repo already serves.
SUBPATH="${SUBPATH-move-book}"
UPSTREAM_BRANCH="${UPSTREAM_BRANCH:-main}"

# Pick a remote pointing at the canonical repo. By convention people who
# work from a fork name it `upstream`; people who clone aptos-core directly
# only have `origin`. Prefer the former; fall back to the latter. An
# explicit `UPSTREAM_REMOTE=<name>` overrides both.
if [ -z "${UPSTREAM_REMOTE:-}" ]; then
  for candidate in upstream origin; do
    if git -C "$(dirname "${BASH_SOURCE[0]}")" remote get-url "$candidate" >/dev/null 2>&1; then
      UPSTREAM_REMOTE="$candidate"
      break
    fi
  done
fi
UPSTREAM_REMOTE="${UPSTREAM_REMOTE:-upstream}"

if [ -z "$SUBPATH" ]; then
  echo "error: SUBPATH must be non-empty so the book does not overwrite the org landing page." >&2
  exit 1
fi

BOOK_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORK_DIR="$(mktemp -d -t move-book-deploy.XXXXXX)"

cleanup() {
  if [ -d "$WORK_DIR" ]; then
    rm -rf "$WORK_DIR"
  fi
}
trap cleanup EXIT

command -v mdbook >/dev/null || {
  echo "error: mdbook not found in PATH" >&2
  exit 1
}

# Refuse to deploy a HEAD that isn't merged into upstream, unless --debug.
# A commit that exists only locally would render a useless SHA on the landing
# page (no one else can resolve it), so we require it to be reachable from
# upstream/<branch> first.
if [ -z "$DEBUG" ]; then
  if ! git -C "$BOOK_DIR" remote get-url "$UPSTREAM_REMOTE" >/dev/null 2>&1; then
    echo "error: remote '$UPSTREAM_REMOTE' is not configured." >&2
    echo "       Set UPSTREAM_REMOTE=<name> to override, or pass --debug to skip the check." >&2
    exit 1
  fi
  echo "==> Verifying HEAD is merged into $UPSTREAM_REMOTE/$UPSTREAM_BRANCH"
  git -C "$BOOK_DIR" fetch --quiet "$UPSTREAM_REMOTE" "$UPSTREAM_BRANCH"
  HEAD_SHA="$(git -C "$BOOK_DIR" rev-parse HEAD)"
  if ! git -C "$BOOK_DIR" merge-base --is-ancestor "$HEAD_SHA" "$UPSTREAM_REMOTE/$UPSTREAM_BRANCH"; then
    echo "error: HEAD ($(git -C "$BOOK_DIR" rev-parse --short HEAD)) is not in $UPSTREAM_REMOTE/$UPSTREAM_BRANCH." >&2
    echo "       Deploying would bake a private commit hash into the published page." >&2
    echo "       Merge the source PR first, or pass --debug to deploy anyway." >&2
    exit 1
  fi
fi

echo "==> Building book"
mdbook build "$BOOK_DIR"

# Stamp every page with the deploy time and source commit. The
# placeholder "(local build)" lives in theme/build-stamp.js, which mdbook
# loads on every rendered page; substituting in that one file updates the
# footer everywhere. Local mdbook builds leave the placeholder untouched.
BUILD_TIME="$(date -u +'%Y-%m-%d %H:%M UTC')"
BUILD_SHA="$(git -C "$BOOK_DIR" rev-parse --short HEAD 2>/dev/null || echo unknown)"
if [ -n "$DEBUG" ]; then
  BUILD_LABEL="$BUILD_TIME (debug build, aptos-core $BUILD_SHA)"
else
  BUILD_LABEL="$BUILD_TIME (aptos-core $BUILD_SHA)"
fi
grep -rl '(local build)' "$BOOK_DIR/html/" 2>/dev/null \
  | xargs -r sed -i "s|(local build)|$BUILD_LABEL|"

echo "==> Cloning $PAGES_REPO ($BRANCH) into temporary directory"
git clone --depth=1 --branch "$BRANCH" "$PAGES_REPO" "$WORK_DIR"

TARGET="$WORK_DIR/$SUBPATH"
echo "==> Replacing $SUBPATH/ with fresh build"
rm -rf "$TARGET"
mkdir -p "$TARGET"
cp -r "$BOOK_DIR/html/." "$TARGET/"

cd "$WORK_DIR"
git add -A "$SUBPATH"

if git diff --cached --quiet; then
  echo "==> No changes to deploy."
  exit 0
fi

git commit -m "deploy move book from aptos-core $BUILD_SHA ($BUILD_TIME)${DEBUG:+ [debug]}"

if [ -n "${DRY_RUN:-}" ]; then
  echo "==> DRY_RUN set; skipping push. Staged commit lives in $WORK_DIR (cleaned on exit)."
  exit 0
fi

echo "==> Pushing to $PAGES_REPO ($BRANCH)"
git push origin "$BRANCH"

echo "==> Done. Pages will rebuild within a minute or two."
echo "    https://aptos-labs.github.io/$SUBPATH/"
