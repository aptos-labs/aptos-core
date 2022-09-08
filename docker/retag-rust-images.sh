#!/usr/bin/env bash

set -ex

IMAGES=(
  validator
  indexer
  node-checker
  tools
  faucet
  forge
  telemetry-service
)

for IMAGE in "${IMAGES[@]}"
do
    crane copy "$REGISTRY_BASE/$IMAGE:$SOURCE_TAG" "$REGISTRY_BASE/$IMAGE:$TARGET_TAG"
done
