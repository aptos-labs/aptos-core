#!/bin/bash
# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

# This script is meant to build the indexer grpc docker images.
# Run it via `docker/indexer-grpc/docker-build-indexer-grpc.sh`
set -ex

export GIT_REPO="${GIT_REPO:-https://github.com/aptos-labs/aptos-core.git}"
# TODO: this is a hack to test the build, we should use main branch.
export GIT_BRANCH="${GIT_BRANCH:-main}"
export GIT_REF="${GIT_REF:-$(git rev-parse HEAD)}"
docker buildx build --file docker/indexer-grpc/cache-worker.Dockerfile --build-arg=GIT_REPO=$GIT_REPO --build-arg=GIT_REF=$GIT_REF --build-arg=GIT_BRANCH=$GIT_BRANCH -t aptos-core:indexer-grpc-cache-worker-$GIT_REF -t aptos-core:indexer-grpc-cache-worker --load .