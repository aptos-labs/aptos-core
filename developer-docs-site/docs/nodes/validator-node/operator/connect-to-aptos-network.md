---
title: "Connecting to Aptos Network"
slug: "connect-to-aptos-network"
---

# Connecting to Aptos Network

This document describes how to connect your running validator node and public fullnode to an Aptos network. Follow these instructions only if you received a notification from the Aptos team confirming your eligibility. 

:::tip Only selected validator nodes
Validator nodes that are not selected will not have enough tokens to join the network. However, you can still run a public fullnode.
:::

## Initializing the staking pool

:::tip Examples using testnet
The CLI command examples used in this section use testnet. You can use the same command for mainnet by passing the mainnet URL for the `--rest-url` parameter.
::: 

1. Initialize CLI with your wallet **private key**. For Petra wallet, you can get your wallet private key from **Settings** > **Credentials**.

  ```bash
  aptos init --profile testnet-owner \
    --rest-url https://testnet.aptoslabs.com
  ```

2. Initialize staking pool using CLI.

  ```bash
  aptos stake initialize-stake-owner \
    --initial-stake-amount 100000000000000 \
    --operator-address <operator-address> \
    --voter-address <voter-address> \
    --profile testnet-owner
  ```

3. Do not forget to transfer some coins to your operator account to pay for gas. You can use Petra wallet directly or with the below command using CLI:

  ```bash
  aptos account create --account <operator-account> --profile testnet-owner
  aptos account transfer \
  --account <operator-account> \
  --amount 5000 \
  --profile testnet-owner
  ```

## Bootstrapping validator node

Before joining the network, you need to bootstrap your node with the genesis blob and waypoint provided by Aptos Labs team. This will convert your node from test mode to production mode. Use one of the below methods to bootstrap your node.

### Using source code

1. Stop your node and remove the data directory. **Make sure you remove the `secure-data.json` file too**. [Click here to see the location of the `secure-data.json` file](https://github.com/aptos-labs/aptos-core/blob/e358a61018bb056812b5c3dbd197b0311a071baf/docker/compose/aptos-node/validator.yaml#L13). 
2. Download the `genesis.blob` and `waypoint.txt` files published by Aptos Labs team.
3. Update your `account_address` in the `validator-identity.yaml` file to your **operator** wallet address. Do not change anything else. Keep the keys as they are. 
4. Pull the latest changes from the `mainnet` branch. It should be commit: `843b204dce971d98449b82624f4f684c7a18b991`.
5. You should use fast sync to bootstrap your node. To sync faster when starting, add the below configuration to your `validator.yaml` and `fullnode.yaml` files. Also see [Fast syncing](/concepts/state-sync#fast-syncing).
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
3. Update your `account_address` in the `validator-identity.yaml` file to your **operator** wallet address.
4. Update your Docker image to use the tag `testnet_843b204dce971d98449b82624f4f684c7a18b991`.
5. You should use fast sync to bootstrap your node. To sync faster when starting, add this configuration to your `validator.yaml` and `fullnode.yaml` files. Also see [Fast syncing](/concepts/state-sync#fast-syncing).
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
4. Close the metrics port and the REST API port for validator. Also use fast sync to bootstrap the node by adding the following Helm values in your `main.tf ` file:

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
5. Apply Terraform: `terraform apply`.
6. Download the `genesis.blob` and `waypoint.txt` files published by Aptos Labs team.
7. Update your `account_address` in the `validator-identity.yaml` file to your **operator** wallet address. Do not change anything else. Keep the keys as they are.
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

## Joining validator set

Follow these steps to setup the validator node using the operator account and join the validator set.

1. Initialize Aptos CLI.

    ```bash
    aptos init --profile testnet-operator \
    --private-key <operator_account_private_key> \
    --rest-url https://testnet.aptoslabs.com \
    --skip-faucet
    ```
    
    :::tip
    The `account_private_key` for the operator can be found in the `private-keys.yaml` file under `~/$WORKSPACE/keys` folder.
    :::

2. Check your validator account balance. Make sure you have some coins to pay gas. You can do this step either by checking on the Aptos Explorer or using the CLI:

    On the Aptos Explorer `https://explorer.aptoslabs.com/account/<account-address>?network=testnet` or use the CLI:

    ```bash
    aptos account list --profile testnet-operator
    ```
    
    This will show you the coin balance you have in the validator account. You will see something like:
    
    ```json
    "coin": {
        "value": "5000"
      }
    ```

3. Update validator network addresses on chain.

    ```bash
    aptos node update-validator-network-addresses  \
      --pool-address <owner-address> \
      --operator-config-file ~/$WORKSPACE/$USERNAME/operator.yaml \
      --profile testnet-operator
    ```

4. Update the validator consensus key on chain.

    ```bash
    aptos node update-consensus-key  \
      --pool-address <owner-address> \
      --operator-config-file ~/$WORKSPACE/$USERNAME/operator.yaml \
      --profile testnet-operator
    ```

5. Join the validator set.

    ```bash
    aptos node join-validator-set \
      --pool-address <owner-address> \
      --profile testnet-operator \
      --max-gas 10000 
    ```

    :::tip Max gas
    You can adjust the above `max-gas` number. Ensure that you sent your operator enough tokens to pay for the gas fee.
    :::

    The `ValidatorSet` will be updated at every epoch change, which is **once every 2 hours**. You will only see your node joining the validator set in the next epoch. Both validator and fullnode will start syncing once your validator is in the validator set.

6. Check the validator set.

    ```bash
    aptos node show-validator-set --profile testnet-operator | jq -r '.Result.pending_active' | grep <account_address>
    ```
    
    You will see your validator node in "pending_active" list. When the next epoch change happens, the node will be moved into "active_validators" list. This will happen within one hour from the completion of previous step. **During this time you might see errors like "No connected AptosNet peers". This is normal.**
    
    ```bash
    aptos node show-validator-set --profile testnet-operator | jq -r '.Result.active_validators' | grep <account_address>
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


## Leaving validator set

A node can choose to leave validator set at anytime, or it would happen automatically when there is insufficient stake in the validator account. To leave the validator set, you can perform the following steps:

Leave validator set (will take effect in next epoch):

```bash
aptos node leave-validator-set --profile testnet-operator --pool-address <owner-address>
```
