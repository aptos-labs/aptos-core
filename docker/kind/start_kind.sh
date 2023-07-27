#!/bin/bash

set -ex

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" &>/dev/null && pwd)

if command -v kind &>/dev/null; then
  echo "kind found"
else
  echo "kind could not be found"
  echo "Install it here: https://kind.sigs.k8s.io/docs/user/quick-start/#installation"
  exit
fi

if uname -a | grep -q Darwin; then
  echo "MacOS detected, building kind node image for arm64..."
  docker build -t tempkind $SCRIPT_DIR
  kind create cluster --image tempkind
else
  kind create cluster --image kindest/node:v1.23.17
fi

kubectl get pods --all-namespaces
