#!/bin/bash
# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

# This script docker bake to build all the rust-based docker images
# You need to execute this from the repository root as working directory
# E.g. docker/docker-bake-rust-all.sh

set -e

## TODO(christian): add `--progress=plain` as soon as circleci supports a docker version that includes https://github.com/moby/buildkit/pull/2763
export GIT_REV=$(git rev-parse --short=8 HEAD)
export GIT_SHA1=$(git rev-parse HEAD)
export BUILD_DATE="$(date -u +'%Y-%m-%dT%H:%M:%SZ')"
docker buildx bake --push --file docker/docker-bake-rust-all.hcl $IMAGE_TARGET
