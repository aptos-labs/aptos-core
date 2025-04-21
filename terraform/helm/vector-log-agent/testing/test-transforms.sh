#!/bin/bash

set -e

export VECTOR_SELF_POD_NAME=my-vector-agent
export K8S_CLUSTER=forge-0

jq -c -M < ./testing/test1.json | vector --quiet --config ./files/vector-transforms.yaml --config ./testing/vector-test-config.yaml | jq
jq -c -M < ./testing/test2.json | vector --quiet --config ./files/vector-transforms.yaml --config ./testing/vector-test-config.yaml | jq
jq -c -M < ./testing/test3.json | vector --quiet --config ./files/vector-transforms.yaml --config ./testing/vector-test-config.yaml | jq

jq -c -M < ./testing/log-level-filter-retained.json | vector --quiet --config ./files/vector-transforms.yaml --config ./testing/vector-test-config.yaml | jq
jq -c -M < ./testing/log-level-filter-filtered.json | vector --quiet --config ./files/vector-transforms.yaml --config ./testing/vector-test-config.yaml | jq
