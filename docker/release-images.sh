#!/usr/bin/env bash

# This script releases our aptos main images to docker hub.
# It does so by copying the images from the aptos GCP artifact registry to docker hub.
# It also copies the release tags to GCP Artifact Registry and AWS ECR.
# Usually it's run in CI, but you can also run it locally in emergency situations, assuming you have the right credentials
# Prerequisites when running locally:
# 1. `docker login` with authorization to push to the `aptoslabs` org
# 2. gcloud auth login --update-adc
# 3. aws-mfa
# Run with: 
# GIT_SHA=${{ github.sha }} GCP_DOCKER_ARTIFACT_REPO="${{ secrets.GCP_DOCKER_ARTIFACT_REPO }}" AWS_ACCOUNT_ID="${{ secrets.AWS_ECR_ACCOUNT_NUM }}" IMAGE_TAG_PREFIX="${{ inputs.image_tag_prefix }}" ./docker/release_images.sh

REQUIRED_VARS=(
  GIT_SHA
  GCP_DOCKER_ARTIFACT_REPO
  AWS_ACCOUNT_ID
  IMAGE_TAG_PREFIX
)

for VAR in "${REQUIRED_VARS[@]}"; do
  if [ -z "${!VAR}" ]; then
    echo "missing required env var: $VAR"
    exit 1
  fi
done

if [ "$CI" == "true" ]; then
  echo "installing crane automatically in CI"
  curl -sL https://github.com/google/go-containerregistry/releases/download/v0.11.0/go-containerregistry_Linux_x86_64.tar.gz > crane.tar.gz
  tar -xf crane.tar.gz
  sha=$(shasum -a 256 ./crane | awk '{ print $1 }')
  [ "$sha" != "2af448965b5feb6c315f4c8e79b18bd15f8c916ead0396be3962baf2f0c815bf" ] && echo "shasum mismatch - got: $sha" && exit 1
  crane="./crane"
else
  if ! [ -x "$(command -v crane)" ]; then
  echo "could not find crane binary in PATH - follow https://github.com/google/go-containerregistry/tree/main/cmd/crane#installation to install"
  exit 1
  fi
  crane=$(which crane)
fi

set -ex

IMAGES=(
  validator
  forge
  tools
  faucet
  indexer
  node-checker
)

TARGET_REGISTRIES=(
  "$GCP_DOCKER_ARTIFACT_REPO"
  "docker.io/aptoslabs"
  "$AWS_ACCOUNT_NUM.dkr.ecr.us-west-2.amazonaws.com/aptos"
)

for IMAGE in "${IMAGES[@]}"; do
  for TARGET_REGISTRY in "${TARGET_REGISTRIES[@]}"; do
    $crane copy "$GCP_DOCKER_ARTIFACT_REPO/$IMAGE:$GIT_SHA" "$TARGET_REGISTRY/$IMAGE:$IMAGE_TAG_PREFIX"
    $crane copy "$GCP_DOCKER_ARTIFACT_REPO/$IMAGE:$GIT_SHA" "$TARGET_REGISTRY/$IMAGE:${IMAGE_TAG_PREFIX}_${GIT_SHA}"
  done
done
