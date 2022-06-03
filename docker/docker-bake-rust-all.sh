#!/bin/bash
# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

# This script docker bake to build all the rust-based docker images
# You need to execute this from the repository root as working directory
# E.g. docker/docker-bake-rust-all.sh

set -ex

export GIT_REV=$(git rev-parse --short=8 HEAD)
export GIT_SHA1=$(git rev-parse HEAD)
export BUILD_DATE="$(date -u +'%Y-%m-%dT%H:%M:%SZ')"
export GIT_BRANCH="${GIT_BRANCH:-$(git rev-parse --abbrev-ref HEAD)}"
docker buildx bake --push --progress=plain --file docker/docker-bake-rust-all.hcl $IMAGE_TARGET
