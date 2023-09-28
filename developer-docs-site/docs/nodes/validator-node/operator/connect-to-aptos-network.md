---
title: "Connect to Aptos Network"
slug: "connect-to-aptos-network"
---

# Connect to Aptos Network

This document describes how to connect your running validator node and validator fullnode to an Aptos network. Follow these instructions only if your validator has met the minimal [staking](../../../concepts/staking.md) requirement. 

:::tip Minimum staking requirement
The current required minimum for staking is 1M APT tokens.
:::


## Initializing the stake pool

First, you need to initialize the stake pool. 

To initialize a staking pool, follow the instructions in [staking pool operations.](../../../nodes/validator-node/operator/staking-pool-operations.md#initialize-cli)

To initialize a delegation pool, follow the instructions in [delegation pool operations.](../../../nodes/validator-node/operatordelegation-pool-operations/#initialize-a-delegation-pool)


## Bootstrapping validator node

After initializing the stake pool, make sure the validator node is bootstrapped with the correct [genesis blob and waypoint](../../node-files-all-networks/node-files.md) for the corresponding network. To bootstrap your node, first you need to know the pool address to use:


```bash
aptos node get-stake-pool \
  --owner-address <owner_address> 
```

### Using source code

1. Stop your node and remove the data directory. 
   - **Make sure you remove the `secure-data.json` file also**. View [validator.yaml](https://github.com/aptos-labs/aptos-core/blob/e358a61018bb056812b5c3dbd197b0311a071baf/docker/compose/aptos-node/validator.yaml#L13) to see the location of the `secure-data.json` file. 
2. Download the `genesis.blob` and `waypoint.txt` files published by Aptos. 
   - See [Node Files](../../node-files-all-networks/node-files.md) for your network (mainnet, testnet, or devnet) for the locations and commands to download these files.
3. Update your `account_address` in the `validator-identity.yaml` and `validator-fullnode-identity.yaml` files to your **pool address**. Do not change anything else. Keep the keys as they are. 
4. Pull the latest changes from the associated (ex. `mainnet`) branch. 
5. [Optional] You can use [fast sync](../../../guides/state-sync.md#fast-syncing) to bootstrap your node if the network has been running for a long time (e.g. testnet, mainnet). Add the below configuration to your `validator.yaml` and `fullnode.yaml` files:
    ```yaml
    state_sync:
     state_sync_driver:
         bootstrapping_mode: DownloadLatestStates
         continuous_syncing_mode: ApplyTransactionOutputs
    ```
6. Close the metrics port `9101` and the REST API port `80` on your validator (you can leave it open for public fullnode).
7. Restart the validator node and validator fullnode.

### Using Docker

1. Stop your node and remove the data volumes: `docker compose down --volumes`. 
   - **Make sure you remove the `secure-data.json` file too.** See this [validator.yaml](https://github.com/aptos-labs/aptos-core/blob/e358a61018bb056812b5c3dbd197b0311a071baf/docker/compose/aptos-node/validator.yaml#L13) line for the location of the `secure-data.json` file. 
2. Download the `genesis.blob` and `waypoint.txt` files published by Aptos. 
   - See [Node Files](../../node-files-all-networks/node-files.md) for locations and commands to download these files.
3. Update your `account_address` in the `validator-identity.yaml` and `validator-fullnode-identity.yaml` files to your **pool address**.
4. Update your Docker image to the [latest release](../../../releases/index.md) of the network branch (e.g. mainnet, testnet).
5. [Optional] You can use [fast sync](../../../guides/state-sync.md#fast-syncing) to bootstrap your node if the network has been running for a long time (e.g. testnet). Add this configuration to your `validator.yaml` and `fullnode.yaml` files:
    ```yaml
    state_sync:
     state_sync_driver:
         bootstrapping_mode: DownloadLatestStates
         continuous_syncing_mode: ApplyTransactionOutputs
    ```
6. Close the metrics port `9101` and the REST API port `80` on your validator (remove it from the Docker compose file). You can leave it open for the public fullnode.
7. Restart the node with: `docker compose up`

### Using Terraform

1. Increase the `era` number in your Terraform configuration. When this configuration is applied, it will wipe the data.
2. Update `chain_id` to 1 (for mainnet). The chain IDs for other Aptos networks are in [Aptos Blockchain Deployments](../../deployments.md).
3. Update your Docker image to the [latest release](../../../releases/index.md) of the network branch (e.g. mainnet, testnet).
4. Close the metrics port and the REST API port for validator. 
5. [Optional] You can use fast sync to bootstrap your node if the network has been running for a long time (e.g. testnet). by adding the following Helm values in your `main.tf ` file:

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


6. **Add monitoring components**

  :::tip Supported only using Terraform
  This is currently only supported using Terraform.
  :::

     1. Set the `enable_monitoring` variable in your terraform module. For example:

         ```rust
         module "aptos-node" {
           ...
           enable_monitoring           = true
           utility_instance_num        = 3  # this will add one more utility instance to run monitoring component
         }
         ```

     2. Apply the changes with: `terraform apply`

     3. You will see a new pod getting created. Run `kubectl get pods` to check.

     4. Access the dashboard.

         First, find the IP/DNS for the monitoring load balancer.

         ```bash
         kubectl get svc ${WORKSPACE}-mon-aptos-monitoring --output jsonpath='{.status.loadBalancer.ingress[0]}'
         ```

         You can access the dashboard on `http://<ip/DNS>`.


7. Pull latest of the terraform module `terraform get -update`, and then apply Terraform: `terraform apply`.
8. Download the `genesis.blob` and `waypoint.txt` files published by Aptos. 
   - See [Node Files](../../node-files-all-networks/node-files.md) for locations and commands to download these files.
9. Update your `account_address` in the `validator-identity.yaml` and `validator-fullnode-identity.yaml` files to your  **pool address**. Do not change anything else. Keep the keys as they are.
10. Recreate the secrets. Make sure the secret name matches your `era` number, e.g. if you have `era = 3`, then you should replace the secret name to be:
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

## Verify Node Connections

Now that you have [connected to the Aptos network](./connect-to-aptos-network.md), you should verify your node connections.

:::tip Node Liveness Definition
See [node liveness criteria](../operator/node-liveness-criteria.md) for details. 
:::

After your validator node has joined the validator set, you can validate its correctness by following these steps:

1. Verify that your node is connecting to other peers on the network. **Replace `127.0.0.1` with your validator IP/DNS if deployed on the cloud**.

    ```bash
    curl 127.0.0.1:9101/metrics 2> /dev/null | grep "aptos_connections{.*\"Validator\".*}"
    ```

    The command will output the number of inbound and outbound connections of your validator node. For example:

    ```bash
    aptos_connections{direction="inbound",network_id="Validator",peer_id="f326fd30",role_type="validator"} 5
    aptos_connections{direction="outbound",network_id="Validator",peer_id="f326fd30",role_type="validator"} 2
    ```

    As long as one of the metrics is greater than zero, your node is connected to at least one of the peers on the testnet.

2. You can also check if your node is connected to an Aptos node: replace `<Aptos Peer ID>` with the peer ID shared by Aptos team.

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
    
5. Finally, the most straight forward way to see if your node is functioning properly is to check if it is making staking reward. You can check it on the Aptos Explorer: `https://explorer.aptoslabs.com/account/<owner-account-address>?network=Mainnet`:

    ```json
    0x1::stake::StakePool

    "active": {
      "value": "100009129447462"
    }
    ```

## Joining Validator Set

After your node has synced, follow the below steps to set up the validator node using the operator account and join the validator set.

:::tip Mainnet vs Testnet
The below CLI command examples use mainnet. Change the `--network` value for testnet and devnet. View the values in [Aptos Blockchain Deployments](../../deployments.md) to see how profiles can be configured based on the network.
:::

### 1. Initialize Aptos CLI

  ```bash
  aptos init --profile mainnet-operator \
  --network mainnet \
  --private-key <operator_account_private_key> \
  --skip-faucet
  ```
  
:::tip
The `account_private_key` for the operator can be found in the `private-keys.yaml` file under `~/$WORKSPACE/keys` folder.
:::

### 2. Check your validator account balance 

Make sure you have enough APT to pay for gas. You can check for this either on the Aptos Explorer or using the CLI:

- On the Aptos Explorer `https://explorer.aptoslabs.com/account/<account-address>?network=Mainnet`, or 
- Use the CLI:

  ```bash
  aptos account list --profile mainnet-operator
  ```
    
This will show you the coin balance you have in the validator account. You will see an output like below:
    
```json
"coin": {
    "value": "5000"
  }
```

:::tip Already in validator set? Skip to Step 6
If you know you are already in the validator set, then skip steps 3, 4, and 5 and go directly to step 6 to confirm it.
:::

### 3. Update validator network addresses on-chain

```bash
aptos node update-validator-network-addresses  \
  --pool-address <pool-address> \
  --operator-config-file ~/$WORKSPACE/$USERNAME/operator.yaml \
  --profile mainnet-operator
```

:::tip Important notes
The network address updates and the consensus key rotation will be applied only at the end of the current epoch. Note that the validator need not leave the validator set to make these updates. You can run the commands for address and key changes. For the remaining duration of the current epoch your validator will still use the old key and addresses but when the epoch ends it will switch to the new key and addresses.
:::

### 4. Rotate the validator consensus key on-chain

```bash
aptos node update-consensus-key  \
  --pool-address <pool-address> \
  --operator-config-file ~/$WORKSPACE/$USERNAME/operator.yaml \
  --profile mainnet-operator
```

### 5. Join the validator set

```bash
aptos node join-validator-set \
  --pool-address <pool-address> \
  --profile mainnet-operator
```

The validator set is updated at every epoch change. You will see your validator node joining the validator set only in the next epoch. Both validator and validator fullnode will start syncing once your validator is in the validator set.

:::tip When is next epoch?
You can see it on the [Aptos Explorer](https://explorer.aptoslabs.com/validators/all?network=mainnet) or by running the command `aptos node get-stake-pool` as shown in [Checking your stake pool information](#checking-your-stake-pool-information).
:::

### 6. Check the validator set
   
When you join the validator set, your validator node will be in "Pending Active" state until the next epoch occurs. **During this time you might see errors like "No connected AptosNet peers". This is normal.** Run the below command to look for your validator in the "pending_active" list.

```bash
aptos node show-validator-set --profile mainnet-operator | jq -r '.Result.pending_active' | grep <pool_address>
```

When the next epoch happens, the node will be moved into "active_validators" list.  Run the below command to see your validator in the "active_validators" list:

```bash
aptos node show-validator-set --profile mainnet-operator | jq -r '.Result.active_validators' | grep <pool_address>
```
    
    You should expect the active value for your `StakePool` to keep increasing. It is updated at every epoch.
