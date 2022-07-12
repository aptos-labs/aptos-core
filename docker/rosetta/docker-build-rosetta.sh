#!/bin/bash
# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

# This script is meant to build the rosetta docker image.
# Run it via `docker/rosetta/docker-build-rosetta.sh`
set -ex

export GIT_SHA="${GIT_SHA:-$(git rev-parse HEAD)}"

docker buildx build --file docker/rosetta/rosetta.Dockerfile --build-arg=GIT_SHA=$GIT_SHA -t aptos-core:rosetta-$GIT_SHA -t aptos-core:rosetta-latest --load .
