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
export NORMALIZED_GIT_BRANCH_OR_PR=$(printf "$TARGET_CACHE_ID" | sed -e 's/[^a-zA-Z0-9]/-/g')

export PROFILE=${PROFILE:-release}
export FEATURES=${FEATURES:-""}
export NORMALIZED_FEATURES_LIST=$(printf "$FEATURES" | sed -e 's/[^a-zA-Z0-9]/_/g')
if [ "$PROFILE" = "release" ]; then
  # Do not prefix image tags if we're building the default profile "release"
  profile_prefix=""
else
  # Builds for profiles other than "release" should be tagged with their profile name
  profile_prefix="${PROFILE}_"
fi

if [ -n "$FEATURES" ]; then
  export IMAGE_TAG_PREFIX="${profile_prefix}${NORMALIZED_FEATURES_LIST}_"
else
  export IMAGE_TAG_PREFIX="${profile_prefix}"
fi

if [ "$CI" == "true" ]; then
  TARGET_REGISTRY=remote docker buildx bake --progress=plain --file docker/docker-bake-rust-all.hcl all --push
  REGISTRY_BASE="$GCP_DOCKER_ARTIFACT_REPO" SOURCE_TAG="cache-${IMAGE_TAG_PREFIX}${NORMALIZED_GIT_BRANCH_OR_PR}" TARGET_TAG="cache-${IMAGE_TAG_PREFIX}${GIT_SHA}" ./docker/retag-rust-images.sh
else
  BUILD_TARGET="${1:-all}"
  TARGET_REGISTRY=local docker buildx bake --file docker/docker-bake-rust-all.hcl $BUILD_TARGET
fi
