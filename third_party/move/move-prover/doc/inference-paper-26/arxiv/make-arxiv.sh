#!/usr/bin/env bash
# Build an arXiv Submission 1.5 tarball for the inference paper.
#
# Produces:
#   stage/                    staging directory (flat layout)
#   arxiv-submission.tar.gz   final upload-ready bundle
#
# Drives main.tex, the single driver of this directory (NeurIPS style, preprint
# mode).

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

# -- 1. Refresh the build via latexmk (also refreshes build/main.bbl).
echo "building main.pdf via latexmk..."
latexmk main.tex >/dev/null

if [[ ! -f build/main.bbl ]]; then
    echo "expected build/main.bbl after latexmk; not found." >&2
    exit 1
fi

page_count() {
    pdfinfo "$1" | awk '/^Pages:/ {print $2}'
}
expected_pages="$(page_count main.pdf)"
echo "main.pdf is ${expected_pages} pages."

# -- 2. Rebuild staging directory.
rm -rf stage arxiv-submission.tar.gz
mkdir stage

# -- 3. Stage the explicit manifest.
cp main.tex stage/main.tex
cp prelude.tex intro.tex example.tex wp.tex skills.tex conclusion.tex stage/
cp esz.sty neurips_2026_vericode.sty stage/
cp build/main.bbl stage/main.bbl

# -- 4. arXiv-emulation build to verify the staged sources compile with plain
#       pdflatex (no latexmkrc, no aux_dir).
mkdir stage/_verify
cp stage/*.tex stage/*.sty stage/main.bbl stage/_verify/
(
    cd stage/_verify
    for i in 1 2 3; do
        pdflatex -interaction=nonstopmode -halt-on-error main.tex >/dev/null
    done
)
verify_pages="$(page_count stage/_verify/main.pdf)"
if [[ "$verify_pages" != "$expected_pages" ]]; then
    echo "arXiv-emulation build produced ${verify_pages} pages, expected ${expected_pages}." >&2
    echo "leaving stage/_verify/ in place for inspection." >&2
    exit 1
fi
if [[ "$KEEP_VERIFY" -eq 0 ]]; then
    rm -rf stage/_verify
fi

# -- 5. Bundle (flat layout, no leading stage/ inside the tarball).
tar -czf arxiv-submission.tar.gz -C stage \
    --exclude=_verify \
    --exclude='_verify/*' \
    .

# -- 6. Summary.
size_bytes="$(stat -c %s arxiv-submission.tar.gz 2>/dev/null \
              || stat -f %z arxiv-submission.tar.gz)"
file_count="$(tar -tzf arxiv-submission.tar.gz | grep -vE '/$|^\./$' | wc -l)"
printf 'arxiv-submission.tar.gz  %d bytes  %d files  %s pages\n' \
    "$size_bytes" "$file_count" "$expected_pages"
