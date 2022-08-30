#!/bin/bash
# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

# This script docker bake to build all the rust-based docker images
# You need to execute this from the repository root as working directory
# E.g. docker/docker-bake-rust-all.sh
# If you want to build a specific target only, run:
#  docker/docker-bake-rust-all.sh <target>
# E.g. docker/docker-bake-rust-all.sh indexer

set -ex

export GIT_SHA=$(git rev-parse HEAD)
export GIT_BRANCH=$(git symbolic-ref --short HEAD)
export GIT_TAG=$(git tag -l --contains HEAD)
export BUILD_DATE="$(date -u +'%Y-%m-%dT%H:%M:%SZ')"
export BUILT_VIA_BUILDKIT="true"
export PROFILE=${PROFILE:-release}
export FEATURES=${FEATURES:-""}

if [ "$CI" == "true" ]; then
  # builder target is the one that builds the rust binaries and is the most expensive.
  # Its output is used by all the other targets that follow.
  # This will also push the builder image as an image to GCP (+ inline cache manifests) even though we don't use this image directly
  TARGET_REGISTRY=gcp docker buildx bake --progress=plain --file docker/docker-bake-rust-all.hcl builder --push
  # build and push the actual images that we use (+ inline cache manifests)
  TARGET_REGISTRY=gcp docker buildx bake --progress=plain --file docker/docker-bake-rust-all.hcl all --push
  # push everything also to AWS - this step will literally just reuse the layers from the previous step so should be kinda fast
  TARGET_REGISTRY=aws docker buildx bake --progress=plain --file docker/docker-bake-rust-all.hcl all --push
else
  BUILD_TARGET="${1:-all}"
  TARGET_REGISTRY=local docker buildx bake --file docker/docker-bake-rust-all.hcl $BUILD_TARGET
fi
