#!/bin/bash

set -e

export VECTOR_SELF_POD_NAME=my-vector-agent
export K8S_CLUSTER=forge-0

if [ -n "$1" ]; then
  # If filename provided, only process that file
  jq -c -M < "$1" | vector --quiet --config ./files/vector-transforms.yaml --config ./testing/vector-test-config.yaml | jq
  exit 0
fi

# Otherwise process all test*.json files
for testfile in ./testing/*.json; do
  echo "Processing $testfile..."
  jq -c -M < "$testfile" | vector --quiet --config ./files/vector-transforms.yaml --config ./testing/vector-test-config.yaml | jq
done
