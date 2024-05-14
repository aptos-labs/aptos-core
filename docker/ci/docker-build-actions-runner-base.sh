#!/bin/bash
# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0

# This script is meant to build the actions-runner-base image used by aptos-labs CI infrastructure.
# Run it via `docker/ci/docker-build-actions-runner-base.sh`
set -ex

export GIT_REF="${GIT_REF:-$(git rev-parse HEAD)}"
docker buildx build --file docker/ci/actions-runner-base.Dockerfile -t us-docker.pkg.dev/aptos-registry/docker/actions-runner-base:$GIT_REF -t us-docker.pkg.dev/aptos-registry/docker/actions-runner-base:latest --push .
