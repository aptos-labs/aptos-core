#!/bin/bash
# Copyright (c) Velor
# SPDX-License-Identifier: Apache-2.0

# This script docker bake to build all the rust-based docker images
# You need to execute this from the repository root as working directory
# E.g. docker/docker-bake-rust-all.sh
# If you want to build a specific target only, run:
#   docker/docker-bake-rust-all.sh <target>
# See targets in docker/builder/docker-bake-rust-all.hcl
#   e.g. docker/docker-bake-rust-all.sh forge-images

set -ex

export GIT_SHA=$(git rev-parse HEAD)
export GIT_BRANCH=$(git symbolic-ref --short HEAD)
export GIT_TAG=$(git tag -l --contains HEAD)
export GIT_CREDENTIALS="${GIT_CREDENTIALS:-}"
export BUILD_DATE="$(date -u +'%Y-%m-%dT%H:%M:%SZ')"
export BUILT_VIA_BUILDKIT="true"
export NORMALIZED_GIT_BRANCH_OR_PR=$(printf "$TARGET_CACHE_ID" | sed -e 's/[^a-zA-Z0-9]/-/g')

export PROFILE=${PROFILE:-release}
export FEATURES=${FEATURES:-""}
export NORMALIZED_FEATURES_LIST=$(printf "$FEATURES" | sed -e 's/[^a-zA-Z0-9]/_/g')
export CUSTOM_IMAGE_TAG_PREFIX=${CUSTOM_IMAGE_TAG_PREFIX:-""}
export CARGO_TARGET_DIR="target/${FEATURES:-"default"}"

if [ "$PROFILE" = "release" ]; then
  # Do not prefix image tags if we're building the default profile "release"
  profile_prefix=""
else
  # Builds for profiles other than "release" should be tagged with their profile name
  profile_prefix="${PROFILE}_"
fi

if [ -n "$CUSTOM_IMAGE_TAG_PREFIX" ]; then
  export IMAGE_TAG_PREFIX="${CUSTOM_IMAGE_TAG_PREFIX}_"
else
  export IMAGE_TAG_PREFIX=""
fi

if [ -n "$FEATURES" ]; then
  export IMAGE_TAG_PREFIX="${IMAGE_TAG_PREFIX}${profile_prefix}${NORMALIZED_FEATURES_LIST}_"
else
  export IMAGE_TAG_PREFIX="${IMAGE_TAG_PREFIX}${profile_prefix}"
fi

BUILD_TARGET="${1:-all}"
echo "Building target: ${BUILD_TARGET}"
echo "To build only a specific target, run: docker/builder/docker-bake-rust-all.sh <target>"
echo "E.g. docker/builder/docker-bake-rust-all.sh forge-images"

if [ "$CI" == "true" ]; then
  docker buildx bake --progress=plain --file docker/builder/docker-bake-rust-all.hcl --push $BUILD_TARGET
else
  docker buildx bake --file docker/builder/docker-bake-rust-all.hcl $BUILD_TARGET --load
fi

echo "Build complete. Docker buildx cache usage:"
docker buildx du --verbose --filter type=exec.cachemount
