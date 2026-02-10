#!/bin/bash

# Get the directory where this script is located
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DOC_DIR="$SCRIPT_DIR/../../../doc"

cd "$DOC_DIR"

for f in sigma_protocol_key_rotation; do
    pandoc $f.md -s --mathjax -o $f.html
done
