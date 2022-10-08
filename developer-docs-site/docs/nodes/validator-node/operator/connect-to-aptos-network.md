---
title: "Connecting to Aptos Network"
slug: "connect-to-aptos-network"
---

# Connecting to Aptos Network

This document describes how to connect your running validator node and public fullnode to an Aptos network. Follow these instructions only if your validator has met the minimal staking requirement. 

:::tip Minimum staking requirement
The current required minimum for staking is 1M APT tokens.
:::

## Bootstrapping validator node

Before joining the network, you need to make sure the validator node is bootstrapped with the correct genesis blob and waypoint for corresponding network. To bootstrap your node, first you need to know the pool address to use:

```
aptos node get-stake-pool --owner-address <owner_address> --url <REST API URL>
```

### Using source code

1. Stop your node and remove the data directory. **Make sure you remove the `secure-data.json` file too**. [Click here to see the location of the `secure-data.json` file](https://github.com/aptos-labs/aptos-core/blob/e358a61018bb056812b5c3dbd197b0311a071baf/docker/compose/aptos-node/validator.yaml#L13). 
2. Download the `genesis.blob` and `waypoint.txt` files published by Aptos Labs team.
3. Update your `account_address` in the `validator-identity.yaml` and `validator-fullnode-identity.yaml` files to your **pool address**. Do not change anything else. Keep the keys as they are. 
4. Pull the latest changes from the `mainnet` branch. It should be commit: `843b204dce971d98449b82624f4f684c7a18b991`.
5. [Optional] You can use fast sync to bootstrap your node if the network has been running for a long time (e.g. testnet). Add the below configuration to your `validator.yaml` and `fullnode.yaml` files. Also see [Fast syncing](/concepts/state-sync#fast-syncing).
    ```yaml
    state_sync:
     state_sync_driver:
         bootstrapping_mode: DownloadLatestStates
         continuous_syncing_mode: ApplyTransactionOutputs
    ```
6. Close the metrics port `9101` and the REST API port `80` on your validator (you can leave it open for public fullnode).
7. Restart the validator node and validator fullnode.

### Using Docker

1. Stop your node and remove the data volumes, `docker compose down --volumes`. **Make sure you remove the `secure-data.json` file too.** [Click here to see the location of the `secure-data.json` file](https://github.com/aptos-labs/aptos-core/blob/e358a61018bb056812b5c3dbd197b0311a071baf/docker/compose/aptos-node/validator.yaml#L13). 
2. Download the `genesis.blob` and `waypoint.txt` files published by Aptos Labs team.
3. Update your `account_address` in the `validator-identity.yaml` and `validator-fullnode-identity.yaml` files to your  **pool address**.
4. Update your Docker image to use the tag `testnet_843b204dce971d98449b82624f4f684c7a18b991`.
5. [Optional] You can use fast sync to bootstrap your node if the network has been running for a long time (e.g. testnet). Add this configuration to your `validator.yaml` and `fullnode.yaml` files. Also see [Fast syncing](/concepts/state-sync#fast-syncing).
    ```yaml
    state_sync:
     state_sync_driver:
         bootstrapping_mode: DownloadLatestStates
         continuous_syncing_mode: ApplyTransactionOutputs
    ```
6. Close the metrics port `9101` and the REST API port `80` on your validator (remove it from the Docker compose file). You can leave it open for the public fullnode.
7. Restart the node: `docker compose up`.

### Using Terraform

1. Increase `era` number in your Terraform configuration. When this configuration is applied, it will wipe the data.
2. Update `chain_id` to 2.
3. Update your Docker image to use the tag `testnet_843b204dce971d98449b82624f4f684c7a18b991`.
4. Close the metrics port and the REST API port for validator. [Optional] You can use fast sync to bootstrap your node if the network has been running for a long time (e.g. testnet). by adding the following Helm values in your `main.tf ` file:

    ```json
    module "aptos-node" {
        ...

        helm_values = {
            validator = {
              config = {
                # use fast sync to start the node
                state_sync = {
                  state_sync_driver = {
                    bootstrapping_mode = "DownloadLatestStates"
                  }
                }
              }
            }
            service = {
              validator = {
                enableRestApi = false
                enableMetricsPort = false
              }
            }
        }
    }

    ```
5. Pull latest of the terraform module `terraform get -update`, and then apply Terraform: `terraform apply`.
6. Download the `genesis.blob` and `waypoint.txt` files published by Aptos Labs team.
7. Update your `account_address` in the `validator-identity.yaml` and `validator-fullnode-identity.yaml` files to your  **pool address**. Do not change anything else. Keep the keys as they are.
8. Recreate the secrets. Make sure the secret name matches your `era` number, e.g. if you have `era = 3`, then you should replace the secret name to be:
  ```bash
  ${WORKSPACE}-aptos-node-0-genesis-e3
  ```

  ```bash
  export WORKSPACE=<your workspace name>

  kubectl create secret generic ${WORKSPACE}-aptos-node-0-genesis-e2 \
      --from-file=genesis.blob=genesis.blob \
      --from-file=waypoint.txt=waypoint.txt \
      --from-file=validator-identity.yaml=keys/validator-identity.yaml \
      --from-file=validator-full-node-identity.yaml=keys/validator-full-node-identity.yaml
  ```


## Verify node connections

:::tip Node Liveness Definition
See [node liveness defined here](https://aptos.dev/reference/node-liveness-criteria/#verifying-the-liveness-of-your-node). 
:::

After your validator node joined the validator set, you can verify the correctness following those steps:

1. Verify that your node is connecting to other peers on testnet. **Replace `127.0.0.1` with your validator IP/DNS if deployed on the cloud**.

    ```bash
    curl 127.0.0.1:9101/metrics 2> /dev/null | grep "aptos_connections{.*\"Validator\".*}"
    ```

    The command will output the number of inbound and outbound connections of your validator node. For example:

    ```bash
    aptos_connections{direction="inbound",network_id="Validator",peer_id="f326fd30",role_type="validator"} 5
    aptos_connections{direction="outbound",network_id="Validator",peer_id="f326fd30",role_type="validator"} 2
    ```

    As long as one of the metrics is greater than zero, your node is connected to at least one of the peers on the testnet.

2. You can also check if your node is connected to Aptos Labs's node: replace `<Aptos Peer ID>` with the peer ID shared by Aptos team.

    ```bash
    curl 127.0.0.1:9101/metrics 2> /dev/null | grep "aptos_network_peer_connected{.*remote_peer_id=\"<Aptos Peer ID>\".*}"
    ```

3. Check if your node is state syncing.

    ```bash
    curl 127.0.0.1:9101/metrics 2> /dev/null | grep "aptos_state_sync_version"
    ```
    
    You should expect to see the "committed" version keeps increasing.

4. After your node state syncs to the latest version, you can also check if consensus is making progress, and your node is proposing.

    ```bash
    curl 127.0.0.1:9101/metrics 2> /dev/null | grep "aptos_consensus_current_round"

    curl 127.0.0.1:9101/metrics 2> /dev/null | grep "aptos_consensus_proposals_count"
    ```

    You should expect to see this number keep increasing.
    
5. Finally, the most straight forward way to see if your node is functioning properly is to check if it is making staking reward. You can check it on the Aptos Explorer: `https://explorer.aptoslabs.com/account/<owner-account-address>?network=testnet`:

    ```json
    0x1::stake::StakePool

    "active": {
      "value": "100009129447462"
    }
    ```
    
    You should expect the active value for your `StakePool` to keep increasing. It is updated at every epoch.
