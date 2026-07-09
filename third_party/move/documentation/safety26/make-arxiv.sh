#!/usr/bin/env bash
# Build an arXiv-ready submission tarball for
# "Defense-in-Depth Runtime Safety in Move".
#
# Produces:
#   arxiv/                    flat staging directory (arXiv AutoTeX layout)
#   arxiv-submission.tar.gz   upload-ready bundle
#
# The paper targets Springer LNCS and is submitted to arXiv with the same
# llncs class, so there is a single main.tex (no neutral-class variant, unlike
# the inference-paper-26 example which had to strip FMCAD branding).
#
# The file manifest is derived automatically from the LaTeX recorder output
# (build/main.fls): every source file the real build reads from this directory
# is staged. Commented-out \input lines and unused assets are therefore never
# shipped, and new section files are picked up without editing this script.
#
# The pre-generated bibliography (build/main.bbl) is staged as main.bbl so the
# arXiv build needs no bibtex pass; biblio.bib is intentionally not shipped.
#
# Usage: ./make-arxiv.sh [--keep-verify]
#   --keep-verify   leave the arxiv/_verify/ emulation build in place.

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
require tar

# Page count parsed from a LaTeX .log (this environment has no pdfinfo).
pages_from_log() {
    grep -oE 'Output written on [^ ]+ \([0-9]+ page' "$1" 2>/dev/null \
        | tail -1 | grep -oE '[0-9]+' | tail -1
}

# -- 1. Refresh the build (regenerates build/main.fls, build/main.bbl, main.pdf).
echo "building main.pdf via latexmk..."
latexmk main.tex >/dev/null

[[ -f build/main.fls ]] || { echo "missing build/main.fls (recorder output)" >&2; exit 1; }
[[ -f build/main.bbl ]] || { echo "missing build/main.bbl (run bibtex/latexmk)" >&2; exit 1; }

expected_pages="$(pages_from_log build/main.log)"
[[ -n "$expected_pages" ]] || { echo "could not read page count from build/main.log" >&2; exit 1; }
echo "main.pdf is ${expected_pages} pages."

# -- 2. Derive the source manifest from the recorder file. Keep INPUT files that
#       live in this directory; drop system (texmf) files and build/ artifacts
#       (.aux/.out/.bbl are handled separately or not needed by arXiv).
mapfile -t sources < <(
    awk '/^INPUT /{print $2}' build/main.fls \
        | sed 's#^\./##' \
        | grep -vE '^/|texmf|texlive|^build/' \
        | sort -u
)
[[ ${#sources[@]} -gt 0 ]] || { echo "no local sources found in build/main.fls" >&2; exit 1; }

# -- 3. Rebuild staging directory (flat layout; all sources live in the root).
rm -rf arxiv arxiv-submission.tar.gz
mkdir arxiv
for f in "${sources[@]}"; do
    [[ -f "$f" ]] || { echo "manifest lists missing file: $f" >&2; exit 1; }
    cp "$f" "arxiv/$f"
done
cp build/main.bbl arxiv/main.bbl
echo "staged ${#sources[@]} source files + main.bbl"

# -- 4. arXiv emulation: compile the staged tree with plain pdflatex (no
#       latexmkrc, no aux_dir, no bibtex) and confirm the page count matches.
mkdir arxiv/_verify
for f in "${sources[@]}"; do cp "arxiv/$f" arxiv/_verify/; done
cp arxiv/main.bbl arxiv/_verify/
(
    cd arxiv/_verify
    for i in 1 2 3; do
        pdflatex -interaction=nonstopmode -halt-on-error main.tex >verify.log 2>&1 || {
            echo "pdflatex failed in arXiv-emulation build:" >&2
            tail -20 verify.log >&2
            exit 1
        }
    done
)
verify_pages="$(pages_from_log arxiv/_verify/main.log)"
if [[ "$verify_pages" != "$expected_pages" ]]; then
    echo "arXiv-emulation build produced ${verify_pages:-?} pages, expected ${expected_pages}." >&2
    echo "leaving arxiv/_verify/ in place for inspection." >&2
    exit 1
fi
[[ "$KEEP_VERIFY" -eq 1 ]] || rm -rf arxiv/_verify

# -- 5. Bundle (flat layout; no leading arxiv/ inside the tarball).
tar -czf arxiv-submission.tar.gz -C arxiv --exclude=_verify --exclude='_verify/*' .

# -- 6. Summary.
size_bytes="$(stat -c %s arxiv-submission.tar.gz 2>/dev/null || stat -f %z arxiv-submission.tar.gz)"
file_count="$(tar -tzf arxiv-submission.tar.gz | grep -vE '/$' | wc -l | tr -d ' ')"
printf 'arxiv-submission.tar.gz  %s bytes  %s files  %s pages\n' \
    "$size_bytes" "$file_count" "$expected_pages"
