#!/bin/bash

# This script checks the compatibility of the protos in this repository under current branch against
# the protos in the aptos-core repository under the specified tag.
# The goal is to provide a way for developers to check if their changes to the protos are compatible with
# the version of aptos-core that they are targeting.

set -ex

# Change to the protos directory
cd "$(git rev-parse --show-toplevel)/protos"

APTOS_CORE_VERSION="${1:?missing commit or tag to compare against}"

repo_url="https://github.com/aptos-labs/aptos-core.git#tag=$APTOS_CORE_VERSION,subdir=protos/proto"
buf breaking --against "$repo_url" --verbose
