#!/bin/bash

# Get the directory where this script is located
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DOC_DIR="$SCRIPT_DIR/../../../doc"

cd "$DOC_DIR"

for f in sigma_protocol_key_rotation sigma_protocol_transfer sigma_protocol_registration sigma_protocol_withdraw; do
    echo "$f.md --> $f.html"
    pandoc $f.md -s --mathjax -o $f.html
done
