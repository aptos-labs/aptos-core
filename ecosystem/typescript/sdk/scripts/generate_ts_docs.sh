#!/bin/sh

# This script helps you regenerate the TS docs at https://github.com/aptos-labs/ts-sdk-doc.

DOCS_DIR=/tmp/ts-sdk-doc

set -e

cd "$(dirname "$0")"
cd ..

# Generate the TS docs to a temporary directory.
rm -rf /tmp/generated-ts-docs
typedoc src/index.ts --out /tmp/generated-ts-docs

# Clone the ts-sdk-doc repo.
rm -rf /tmp/ts-sdk-doc
git clone git@github.com:aptos-labs/ts-sdk-doc.git $DOCS_DIR

# Copy the generated docs into the ts-sdk-doc repo.
rm -rf $DOCS_DIR/*
mv /tmp/generated-ts-docs/* $DOCS_DIR

# Copy in a basic README
echo "# TS SDK Docs" > $DOCS_DIR/README.md
echo "" >> $DOCS_DIR/README.md
echo 'Generated from `ecosystem/typescript/sdk/` in [aptos-core](https://github.com/aptos-labs/aptos-core/tree/main/ecosystem/typescript/sdk) using `pnpm generate-ts-docs`.' >> $DOCS_DIR/README.md

# Done!
echo
echo "Generated docs to $DOCS_DIR"
echo "From here, ensure that the changes look good. If so, copy the changes into a checkout of the ts-sdk-doc repo and make a PR!"
