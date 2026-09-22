#!/usr/bin/env bash
# Build an arXiv Submission 1.5 tarball for the inference paper.
#
# Produces:
#   arxiv/                    staging directory (flat layout)
#   arxiv-submission.tar.gz   final upload-ready bundle
#
# Drives main-arxiv.tex (neutral IEEEtran class, no FMCAD branding).
# main.tex (FMCAD class) is left alone; both build paths coexist.

set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")"

KEEP_VERIFY=0
for arg in "$@"; do
    case "$arg" in
        --keep-verify) KEEP_VERIFY=1 ;;
        *) echo "unknown flag: $arg" >&2; exit 2 ;;
    esac
done

require() {
    command -v "$1" >/dev/null 2>&1 || { echo "missing tool: $1" >&2; exit 2; }
}
require latexmk
require pdflatex
require pdfinfo
require tar

# -- 1. Drift check: shared body of main.tex and main-arxiv.tex must match.
body_of() {
    awk '/^\\begin\{document\}/{flag=1} flag' "$1"
}
if ! diff -q <(body_of main.tex) <(body_of main-arxiv.tex) >/dev/null; then
    echo "drift between main.tex and main-arxiv.tex (body after \\begin{document} differs):" >&2
    diff -u <(body_of main.tex) <(body_of main-arxiv.tex) | head -40 >&2 || true
    echo "fix one of them so the post-preamble content matches, then re-run." >&2
    exit 1
fi

# -- 2. Refresh the arXiv build via latexmk (refreshes build/main-arxiv.bbl).
echo "building main-arxiv.pdf via latexmk..."
latexmk main-arxiv.tex >/dev/null

if [[ ! -f build/main-arxiv.bbl ]]; then
    echo "expected build/main-arxiv.bbl after latexmk; not found." >&2
    exit 1
fi

page_count() {
    pdfinfo "$1" | awk '/^Pages:/ {print $2}'
}
expected_pages="$(page_count main-arxiv.pdf)"
echo "main-arxiv.pdf is ${expected_pages} pages."

# -- 3. Rebuild staging directory.
rm -rf arxiv arxiv-submission.tar.gz
mkdir arxiv

# -- 4. Stage the explicit manifest.
cp main-arxiv.tex arxiv/main.tex
cp prelude.tex intro.tex example.tex wp.tex skills.tex conclusion.tex arxiv/
cp esz.sty arxiv/
cp build/main-arxiv.bbl arxiv/main.bbl

# -- 5. arXiv-emulation build to verify the staged sources compile with plain
#       pdflatex (no latexmkrc, no aux_dir).
mkdir arxiv/_verify
cp arxiv/*.tex arxiv/*.sty arxiv/main.bbl arxiv/_verify/
(
    cd arxiv/_verify
    for i in 1 2 3; do
        pdflatex -interaction=nonstopmode -halt-on-error main.tex >/dev/null
    done
)
verify_pages="$(page_count arxiv/_verify/main.pdf)"
if [[ "$verify_pages" != "$expected_pages" ]]; then
    echo "arXiv-emulation build produced ${verify_pages} pages, expected ${expected_pages}." >&2
    echo "leaving arxiv/_verify/ in place for inspection." >&2
    exit 1
fi
if [[ "$KEEP_VERIFY" -eq 0 ]]; then
    rm -rf arxiv/_verify
fi

# -- 6. Bundle (flat layout, no leading arxiv/ inside the tarball).
tar -czf arxiv-submission.tar.gz -C arxiv \
    --exclude=_verify \
    --exclude='_verify/*' \
    .

# -- 7. Summary.
size_bytes="$(stat -c %s arxiv-submission.tar.gz 2>/dev/null \
              || stat -f %z arxiv-submission.tar.gz)"
file_count="$(tar -tzf arxiv-submission.tar.gz | grep -vE '/$|^\./$' | wc -l)"
printf 'arxiv-submission.tar.gz  %d bytes  %d files  %s pages\n' \
    "$size_bytes" "$file_count" "$expected_pages"
