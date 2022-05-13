---
title: "Connecting to Aptos Incentivized Testnet"
slug: "connect-to-testnet"
sidebar_position: 14
---

# Connecting to Aptos Incentivized Testnet

Only do this if you got confirmation email from Aptos team for your eligibility. Nodes not selected will not be included in the genesis, thus not be able to connecting to incentivized testnet as a validator node. You can still run public fullnode in this case if you want.

## Using source code

- Stop your node and remove the data directory.
- Download the `genesis.blob` and `waypoint.txt` file published by Aptos Labs team.
- Restarting the node

## Using Docker

- Stop your node and remove the data volumes, `docker-compose down --volumes`
- Download the `genesis.blob` and `waypoint.txt` file published by Aptos Labs team.
- Restarting the node: `docker-compose up`

## Using Terraform

- Increase `era` number in your Terraform config, this will wipe the data once applied.
- Apply Terraform: `terraform apply`
- Download the `genesis.blob` and `waypoint.txt` file published by Aptos Labs team.
- Recreate the secrets:
    ```
    export WORKSPACE=<your workspace name>

    kubectl create secret generic ${WORKSPACE}-aptos-node-genesis-e2 \
        --from-file=genesis.blob=genesis.blob \
        --from-file=waypoint.txt=waypoint.txt \
        --from-file=validator-identity.yaml=validator-identity.yaml \
        --from-file=validator-full-node-identity.yaml=validator-full-node-identity.yaml
    ```