#!/bin/bash

set -e

export VECTOR_SELF_POD_NAME=my-vector-agent
export K8S_CLUSTER=mycluster

cat ./testing/test1.json | jq -c -M | vector --quiet --config ./files/vector-transforms.yaml --config ./testing/vector-test-config.yaml | jq
cat ./testing/test2.json | jq -c -M | vector --quiet --config ./files/vector-transforms.yaml --config ./testing/vector-test-config.yaml | jq
