#!/bin/sh
# Copyright © Velor Foundation
# SPDX-License-Identifier: Apache-2.0

# This script is a utility script used by some rust tests such as velor-core/secure/storage/src/tests/vault.rs .
# If this is not directly referenced anywhere in the codebase anymore you can delete it.

set -ex

IP="172.18.0.3"
PORT="8200"
TOKEN="root_token"

docker network create --subnet 172.18.0.0/24 testnet || true

docker run \
    --cap-add=IPC_LOCK \
    -e "VAULT_DEV_LISTEN_ADDRESS=0.0.0.0:$PORT" \
    -e "VAULT_DEV_ROOT_TOKEN_ID=$TOKEN" \
    --ip "$IP" \
    --network testnet \
    --publish "$PORT:$PORT" \
    --detach \
    vault

docker run \
    -e "VAULT_ADDR=http://$IP:$PORT" \
    -e "VAULT_TOKEN=$TOKEN" \
    --network testnet \
    --entrypoint vault \
    vault secrets enable transit
