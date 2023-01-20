#!/bin/bash
# Copyright (c) The Move Contributors
# SPDX-License-Identifier: Apache-2.0
#
# A script to package the move binary into a zip package.

rm -rf move-artifacts/*
mkdir -p move-artifacts/
BINS="move move-analyzer move-bytecode-viewer move-disassembler move-prover move-to-yul prover-lab"

for BIN in $BINS; do
  cp -v target/release/$BIN move-artifacts/
done

cp -v README.md move-artifacts/

if [ "$1" == "windows-latest" ]; then
  7z a -r move-$1.zip move-artifacts
else
  zip -r move-$1.zip move-artifacts
fi
