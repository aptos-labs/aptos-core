#!/bin/sh

# Init script to be run in a Forge pod
# This separates the pod lifecycle from aptos/aptos

# set up internal helm repo, naming it testnet-internal
helm plugin install https://github.com/hypnoglow/helm-s3.git
helm repo add testnet-internal s3://{{ .Values.forge.helmBucket }}/charts
helm search repo testnet-internal --versions

# init vault
export VAULT_ADDR="{{ .Values.vault.server.address }}"
export VAULT_CACERT="{{ .Values.vault.server.ca_cert }}"
export VAULT_TOKEN=$(cat /opt/vault/token)
