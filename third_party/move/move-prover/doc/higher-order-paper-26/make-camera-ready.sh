#!/usr/bin/env bash
# Build the FMCAD'26 camera-ready bundle for EasyChair (submission 116).
#
# Produces:
#   camera/                staging directory
#     paper116.pdf         camera-ready PDF (reference copy)
#     copyright116.pdf     copyright form; SIGNED copy is used if
#                          copyright/copyright116-signed.pdf exists,
#                          otherwise the pre-filled unsigned form
#     src/                 LaTeX sources (compile with plain pdflatex,
#                          no --shell-escape)
#   fmcad26-paper116.zip   upload-ready bundle
#
# No artifact116.txt is included: the paper has no DOI-registered artifact.

set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")"

require() {
    command -v "$1" >/dev/null 2>&1 || { echo "missing tool: $1" >&2; exit 2; }
}
require latexmk
require pdflatex
require bibtex
require pdfinfo
require zip

# -- 1. Fresh build of the paper.
echo "building main.pdf via latexmk..."
latexmk main.tex >/dev/null
expected_pages="$(pdfinfo main.pdf | awk '/^Pages:/ {print $2}')"
echo "main.pdf is ${expected_pages} pages."

# -- 2. Build the pre-filled copyright form.
(
    cd copyright
    pdflatex -interaction=nonstopmode -halt-on-error fill.tex >/dev/null
)

# -- 3. Stage the bundle.
rm -rf camera fmcad26-paper116.zip
mkdir -p camera/src

cp main.pdf camera/paper116.pdf
if [[ -f copyright/copyright116-signed.pdf ]]; then
    cp copyright/copyright116-signed.pdf camera/copyright116.pdf
    echo "using SIGNED copyright form."
else
    cp copyright/fill.pdf camera/copyright116.pdf
    echo "WARNING: using UNSIGNED pre-filled copyright form;"
    echo "         sign it and save as copyright/copyright116-signed.pdf, then re-run."
fi

cp main.tex prelude.tex intro.tex move.tex encoding.tex validation.tex \
   conclusion.tex fmcad.cls esz.sty biblio.bib camera/src/
cp build/main.bbl camera/src/

# -- 4. Verify the staged sources compile standalone with plain pdflatex
#       (no latexmkrc, no aux_dir, no shell-escape), as production will.
mkdir camera/_verify
cp camera/src/* camera/_verify/
(
    cd camera/_verify
    pdflatex -interaction=nonstopmode -halt-on-error main.tex >/dev/null
    bibtex main >/dev/null
    pdflatex -interaction=nonstopmode -halt-on-error main.tex >/dev/null
    pdflatex -interaction=nonstopmode -halt-on-error main.tex >/dev/null
)
verify_pages="$(pdfinfo camera/_verify/main.pdf | awk '/^Pages:/ {print $2}')"
if [[ "$verify_pages" != "$expected_pages" ]]; then
    echo "standalone build produced ${verify_pages} pages, expected ${expected_pages}." >&2
    echo "leaving camera/_verify/ in place for inspection." >&2
    exit 1
fi
rm -rf camera/_verify

# -- 5. Bundle.
(
    cd camera
    zip -q -r ../fmcad26-paper116.zip paper116.pdf copyright116.pdf src
)

# -- 6. Summary.
size_bytes="$(stat -c %s fmcad26-paper116.zip 2>/dev/null \
              || stat -f %z fmcad26-paper116.zip)"
printf 'fmcad26-paper116.zip  %d bytes  (%s pages)\n' \
    "$size_bytes" "$expected_pages"
