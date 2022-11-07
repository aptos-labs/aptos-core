#!/bin/sh

# This script helps you regenerate the TS docs at https://github.com/aptos-labs/ts-sdk-doc.

set -e

cd "$(dirname "$0")"
cd ..

# Generate the TS docs to a temporary directory.
rm -rf /tmp/generated-ts-docs
typedoc src/index.ts --out /tmp/generated-ts-docs

# Clone the ts-sdk-doc repo.
rm -rf /tmp/ts-sdk-doc
git clone git@github.com:aptos-labs/ts-sdk-doc.git /tmp/ts-sdk-doc

# Copy the generated docs into the ts-sdk-doc repo.
rm -rf /tmp/ts-sdk-doc/*
mv /tmp/generated-ts-docs/* /tmp/ts-sdk-doc

# Done!
echo
echo "Generated docs to /tmp/ts-sdk-doc"
echo "From here, ensure that the changes look good and make a PR if so!"
