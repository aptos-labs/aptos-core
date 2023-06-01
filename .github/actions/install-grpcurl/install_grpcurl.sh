#!/bin/bash

set -ex

# A quick script that installs grpcurl if it's not already installed.

if ! command -v grpcurl &>/dev/null; then
    wget https://github.com/fullstorydev/grpcurl/releases/download/v1.8.7/grpcurl_1.8.7_linux_x86_64.tar.gz
    sha=$(shasum -a 256 grpcurl_1.8.7_linux_x86_64.tar.gz | awk '{ print $1 }')
    [ "$sha" != "b50a9c9cdbabab03c0460a7218eab4a954913d696b4d69ffb720f42d869dbdd5" ] && echo "shasum mismatch" && exit 1
    tar -xvf grpcurl_1.8.7_linux_x86_64.tar.gz
    chmod +x grpcurl
    ./grpcurl -version
fi

echo "grpcurl is installed"
