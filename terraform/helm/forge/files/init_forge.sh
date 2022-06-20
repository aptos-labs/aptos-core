#!/bin/sh

# Init script to be run in a Forge pod
# This separates the pod lifecycle from aptos/aptos

# set up internal helm repo, naming it testnet-internal
echo "Setting up repo s3://{{ .Values.forge.helmBucket }}/charts"
helm plugin install https://github.com/C123R/helm-blob.git
helm repo add testnet-internal s3://{{ .Values.forge.helmBucket }}/charts
helm search repo testnet-internal --versions
