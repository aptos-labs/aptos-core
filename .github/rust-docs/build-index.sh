#!/usr/bin/env bash
#
# Generate a top-level index.html for the rustdoc output produced by
# `cargo doc --workspace --no-deps`.
#
# Cargo writes per-crate HTML directories under target/doc/ but doesn't write
# its own root index.html on stable (the `--enable-index-page` rustdoc flag is
# nightly-only). This script enumerates the crate directories that actually
# contain a rendered `index.html` and emits a simple landing page that links
# to each one. It also stamps the page with the build SHA and timestamp passed
# in via the environment so the deployed site advertises its provenance.
#
# Inputs (env):
#   DOC_DIR     directory containing rustdoc output (default: target/doc)
#   BUILD_SHA   full git sha of the source commit (default: from `git`)
#   BUILD_TIME  human-readable build timestamp (default: now in UTC)
#   REPO_URL    https URL of the source repo, used to deep-link the SHA
#               (default: https://github.com/aptos-labs/aptos-core)
#
# Output:
#   $DOC_DIR/index.html

set -euo pipefail

DOC_DIR="${DOC_DIR:-target/doc}"
BUILD_SHA="${BUILD_SHA:-$(git rev-parse HEAD 2>/dev/null || echo unknown)}"
BUILD_SHORT="$(printf '%s' "$BUILD_SHA" | cut -c1-12)"
BUILD_TIME="${BUILD_TIME:-$(date -u +'%Y-%m-%d %H:%M UTC')}"
REPO_URL="${REPO_URL:-https://github.com/aptos-labs/aptos-core}"

if [ ! -d "$DOC_DIR" ]; then
  echo "error: doc directory '$DOC_DIR' does not exist" >&2
  exit 1
fi

# Enumerate every immediate subdirectory of $DOC_DIR that has its own
# rustdoc-generated index.html. Cargo creates one such directory per crate
# (using the crate's library name with hyphens converted to underscores), so
# this set is exactly the list of crates whose docs we successfully built.
mapfile -t CRATES < <(
  find "$DOC_DIR" -mindepth 2 -maxdepth 2 -name index.html -printf '%h\n' \
    | xargs -r -n1 basename \
    | LC_ALL=C sort -u
)

if [ "${#CRATES[@]}" -eq 0 ]; then
  echo "error: no crate documentation found under $DOC_DIR" >&2
  exit 1
fi

echo "==> Generating index.html for ${#CRATES[@]} crates"

INDEX="$DOC_DIR/index.html"

{
  cat <<HTML_HEAD
<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>aptos-core Rust API documentation</title>
  <style>
    :root {
      color-scheme: light dark;
      --fg: #1f2328;
      --muted: #57606a;
      --bg: #ffffff;
      --accent: #0969da;
      --border: #d0d7de;
    }
    @media (prefers-color-scheme: dark) {
      :root {
        --fg: #e6edf3;
        --muted: #8b949e;
        --bg: #0d1117;
        --accent: #58a6ff;
        --border: #30363d;
      }
    }
    body {
      font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Helvetica, Arial, sans-serif;
      color: var(--fg);
      background: var(--bg);
      margin: 0;
      padding: 2rem 1.5rem 4rem;
    }
    main { max-width: 64rem; margin: 0 auto; }
    h1 { margin: 0 0 0.5rem; font-size: 2rem; }
    p { color: var(--muted); margin: 0.25rem 0 1rem; }
    .meta {
      font-size: 0.9rem;
      color: var(--muted);
      border-bottom: 1px solid var(--border);
      padding-bottom: 1rem;
      margin-bottom: 1.5rem;
    }
    .meta a { color: var(--accent); text-decoration: none; }
    .meta a:hover { text-decoration: underline; }
    .filter {
      width: 100%;
      box-sizing: border-box;
      padding: 0.5rem 0.75rem;
      font-size: 1rem;
      border: 1px solid var(--border);
      border-radius: 6px;
      background: var(--bg);
      color: var(--fg);
      margin-bottom: 1rem;
    }
    ul.crates {
      list-style: none;
      padding: 0;
      margin: 0;
      display: grid;
      grid-template-columns: repeat(auto-fill, minmax(18rem, 1fr));
      gap: 0.25rem 1rem;
    }
    ul.crates li { padding: 0.2rem 0; }
    ul.crates a { color: var(--accent); text-decoration: none; font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace; }
    ul.crates a:hover { text-decoration: underline; }
    footer { margin-top: 3rem; font-size: 0.8rem; color: var(--muted); }
  </style>
</head>
<body>
<main>
  <h1>aptos-core Rust API documentation</h1>
  <p>Auto-generated <code>cargo doc</code> output for every crate in the
     <a href="${REPO_URL}">aptos-core</a> workspace.</p>
  <div class="meta">
    Built from
    <a href="${REPO_URL}/commit/${BUILD_SHA}"><code>${BUILD_SHORT}</code></a>
    at ${BUILD_TIME}.
    Other documentation:
    <a href="https://aptos.dev">aptos.dev</a> ·
    <a href="https://aptos-labs.github.io/move-book/">Move book</a> ·
    <a href="https://aptos-labs.github.io/framework-book/">Aptos framework book</a>.
  </div>
  <input class="filter" type="search" placeholder="Filter crates&hellip;" oninput="filterCrates(this.value)" autofocus>
  <ul class="crates" id="crates">
HTML_HEAD

  for crate in "${CRATES[@]}"; do
    # Each crate dir always contains an index.html; link straight to it.
    printf '    <li><a href="%s/index.html">%s</a></li>\n' "$crate" "$crate"
  done

  cat <<HTML_TAIL
  </ul>
  <footer>
    aptos-core ${BUILD_SHORT} · ${#CRATES[@]} crates · built ${BUILD_TIME}
  </footer>
</main>
<script>
  // Trivial client-side filter so users can find a crate without scrolling.
  function filterCrates(needle) {
    const q = needle.trim().toLowerCase();
    for (const li of document.querySelectorAll('#crates li')) {
      const name = li.textContent.toLowerCase();
      li.style.display = !q || name.includes(q) ? '' : 'none';
    }
  }
</script>
</body>
</html>
HTML_TAIL
} > "$INDEX"

echo "==> Wrote $INDEX"
