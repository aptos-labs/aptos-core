#!/bin/bash
# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

# Builds a test validator (aptos-node) image by copying from a source image, replacing the binary, and pushing to a target
# Expects SOURCE_IMAGE, TARGET_IMAGE, SOURCE_APTOS_NODE_BINARY to be set
# For MacOS users, check https://betterprogramming.pub/cross-compiling-rust-from-mac-to-linux-7fad5a454ab1

if [ -z "$SOURCE_IMAGE" ]; then
    echo "SOURCE_IMAGE not set"
    exit 1
fi
if [ -z "$TARGET_IMAGE" ]; then
    echo "TARGET_IMAGE not set"
    exit 1
fi
if [ -z "$SOURCE_APTOS_NODE_BINARY" ]; then
    echo "SOURCE_APTOS_NODE_BINARY not set"
    exit 1
fi

tempdir=$(mktemp -d)
echo "Using tempdir $tempdir"
cp $SOURCE_APTOS_NODE_BINARY $tempdir # copy the binary into docker dir for context
cat <<EOF >$tempdir/Dockerfile
FROM $SOURCE_IMAGE
# Override the aptos-node binary at its installed location
COPY $(basename $SOURCE_APTOS_NODE_BINARY) /usr/local/bin/aptos-node
EOF

docker build -t $TARGET_IMAGE $tempdir
docker push $TARGET_IMAGE
