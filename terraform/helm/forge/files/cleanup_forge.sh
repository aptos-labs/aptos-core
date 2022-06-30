#!/bin/sh

# Script to be run in a Forge pod after the test runs as cleanup
# This separates the pod lifecycle from aptos-core

kubectl delete deployment,sts -l "app.kubernetes.io/part-of=aptos-node"
echo "Forge cleaned"
