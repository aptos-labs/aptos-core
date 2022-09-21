---
title: "Connecting to Aptos Testnet"
slug: "connect-to-testnet"
sidebar_position: 14
---

# Connecting to Aptos Testnet

Do this only if you received the confirmation email from Aptos team for your eligibility. Nodes not selected will not have enough tokens to join the testnet. You can still run public fullnode in this case if you want.

## Initializing the staking pool

In testnet we will have UI support to allow owner managing the staking pool, see details [here](https://aptos.dev/nodes/ait/steps-in-ait3#initialize-staking-pool), if you've already done this through the UI, you can igore this step and jump into "Bootstrapping validator node". 

Alternatively, you can also use CLI to intialize staking pool:

- Initialize CLI with your wallet **private key**, you can get in from Settings -> Credentials

  ```bash
  aptos init --profile testnet-owner \
    --rest-url https://testnet.aptoslabs.com
  ```

- Initialize staking pool using CLI

  ```bash
  aptos stake initialize-stake-owner \
    --initial-stake-amount 100000000000000 \
    --operator-address <operator-address> \
    --voter-address <voter-address> \
    --profile testnet-owner
  ```

- Don't forget to transfer some coin to your operator account to pay gas, you can do that with Petra, or CLI

  ```bash
  aptos account create --account <operator-account> --profile testnet-owner
  aptos account transfer \
  --account <operator-account> \
  --amount 5000 \
  --profile testnet-owner
  ```

## Bootstrapping validator node

Before joining the testnet, you need to bootstrap your node with the genesis blob and waypoint provided by Aptos Labs team. This will convert your node from test mode to prod mode.

### Using source code

- Stop your node and remove the data directory. **Make sure you remove the secure-data.json file too**, path is defined [here](https://github.com/aptos-labs/aptos-core/blob/e358a61018bb056812b5c3dbd197b0311a071baf/docker/compose/aptos-node/validator.yaml#L13). 
- Download the `genesis.blob` and `waypoint.txt` file published by Aptos Labs team.
- Update your `account_address` in `validator-identity.yaml` to your **owner** wallet address, don't change anything else, keep the keys as is.
- Pull the latest changes on `testnet` branch. It should be commit `843b204dce971d98449b82624f4f684c7a18b991`
- You should use fast sync to bootstrap your node, to sync faster when starting, add this to your `validator.yaml` and `fullnode.yaml`
    ```
    state_sync:
     state_sync_driver:
         bootstrapping_mode: DownloadLatestStates
         continuous_syncing_mode: ApplyTransactionOutputs
    ```
- Close the metrics port `9101` and REST API port `80` for your validator (you can leave it open for fullnode).
- Restarting the node

### Using Docker

- Stop your node and remove the data volumes, `docker compose down --volumes`. Make sure you remove the secure-data.json file too, path is defined [here](https://github.com/aptos-labs/aptos-core/blob/e358a61018bb056812b5c3dbd197b0311a071baf/docker/compose/aptos-node/validator.yaml#L13). 
- Download the `genesis.blob` and `waypoint.txt` file published by Aptos Labs team.
- Update your `account_address` in `validator-identity.yaml` to your **owner** wallet address.
- Update your docker image to use tag `testnet_843b204dce971d98449b82624f4f684c7a18b991`
- You should use fast sync to bootstrap your node, to sync faster when starting, add this to your `validator.yaml` and `fullnode.yaml`
    ```
    state_sync:
     state_sync_driver:
         bootstrapping_mode: DownloadLatestStates
         continuous_syncing_mode: ApplyTransactionOutputs
    ```
- Close metrics port on 9101 and REST API port `80` for your validator (remove it from the docker compose file), you can leave it open for fullnode.
- Restarting the node: `docker compose up`

### Using Terraform

- Increase `era` number in your Terraform config, this will wipe the data once applied.
- Update `chain_id` to 2.
- Update your docker image to use tag `testnet_843b204dce971d98449b82624f4f684c7a18b991`
- Close metrics port and REST API port for validator, also use fast sync to bootstrap the node. add the helm values in your `main.tf ` file, for example:

    ```
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
- Apply Terraform: `terraform apply`
- Download the `genesis.blob` and `waypoint.txt` file published by Aptos Labs team.
- Update your `account_address` in `validator-identity.yaml` to your **owner** wallet address, don't change anything else, keep the keys as is.
- Recreate the secrets, make sure the secret name matches your `era` number, e.g. if you have `era = 3`, you should replace the secret name to be `${WORKSPACE}-aptos-node-0-genesis-e3`

    ```bash
    export WORKSPACE=<your workspace name>

    kubectl create secret generic ${WORKSPACE}-aptos-node-0-genesis-e2 \
        --from-file=genesis.blob=genesis.blob \
        --from-file=waypoint.txt=waypoint.txt \
        --from-file=validator-identity.yaml=keys/validator-identity.yaml \
        --from-file=validator-full-node-identity.yaml=keys/validator-full-node-identity.yaml
    ```

## Joining Validator Set

At this point you already used your owner account to initialized a validator staking pool, and assigned the operator to your operator account. The step below is to setup the validator node using operator account, and join the validator set.

1. Initialize Aptos CLI

    ```bash
    aptos init --profile testnet-operator \
    --private-key <operator_account_private_key> \
    --rest-url https://testnet.aptoslabs.com \
    --skip-faucet
    ```
    
    :::tip
    The `account_private_key` for the operator can be found in the `private-keys.yaml` file under `~/$WORKSPACE/keys` folder.
    :::

2. Check your validator account balance, make sure you have some coins to pay gas. (If not, transfer some coin to this account from your owner account) 

    You can check on the explorer `https://explorer.aptoslabs.com/account/<account-address>?network=testnet` or use the CLI

    ```bash
    aptos account list --profile testnet-operator
    ```
    
    This will show you the coin balance you have in the validator account. You should be able to see something like:
    
    ```json
    "coin": {
        "value": "5000"
      }
    ```

3. Update validator network addresses on chain

    ```bash
    aptos node update-validator-network-addresses  \
      --pool-address <owner-address> \
      --operator-config-file ~/$WORKSPACE/$USERNAME/operator.yaml \
      --profile testnet-operator
    ```

4. Update the validator consensus key on chain

    ```bash
    aptos node update-consensus-key  \
      --pool-address <owner-address> \
      --operator-config-file ~/$WORKSPACE/$USERNAME/operator.yaml \
      --profile testnet-operator
    ```

5. Join the validator set

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

6. Check the validator set

    ```bash
    aptos node show-validator-set --profile testnet-operator | jq -r '.Result.pending_active' | grep <account_address>
    ```
    
    You will see your validator node in "pending_active" list. When the next epoch change happens, the node will be moved into "active_validators" list. This will happen within one hour from the completion of previous step. During this time, you might see errors like "No connected AptosNet peers", which is normal.
    
    ```bash
    aptos node show-validator-set --profile testnet-operator | jq -r '.Result.active_validators' | grep <account_address>
    ```


## Verify node connections

:::tip Node Liveness Definition
See the details of node liveness definition [here](https://aptos.dev/reference/node-liveness-criteria/#verifying-the-liveness-of-your-node). 
:::

Once your validator node joined the validator set, you can verify the correctness following those steps:

1. Verify that your node is connecting to other peers on testnet. **Replace `127.0.0.1` with your Validator IP/DNS if deployed on the cloud**.

    ```bash
    curl 127.0.0.1:9101/metrics 2> /dev/null | grep "aptos_connections{.*\"Validator\".*}"
    ```

    The command will output the number of inbound and outbound connections of your validator node. For example:

    ```bash
    aptos_connections{direction="inbound",network_id="Validator",peer_id="f326fd30",role_type="validator"} 5
    aptos_connections{direction="outbound",network_id="Validator",peer_id="f326fd30",role_type="validator"} 2
    ```

    As long as one of the metrics is greater than zero, your node is connected to at least one of the peers on the testnet.

2. You can also check if your node is connected to AptosLabs's node, replace `<Aptos Peer ID>` with the peer ID shared by Aptos team.

    ```bash
    curl 127.0.0.1:9101/metrics 2> /dev/null | grep "aptos_network_peer_connected{.*remote_peer_id=\"<Aptos Peer ID>\".*}"
    ```

3. Check if your node is state syncing

    ```bash
    curl 127.0.0.1:9101/metrics 2> /dev/null | grep "aptos_state_sync_version"
    ```
    
    You should expect to see the "committed" version keeps increasing.

4. Once your node state sync to the latest version, you can also check if consensus is making progress, and your node is proposing

    ```bash
    curl 127.0.0.1:9101/metrics 2> /dev/null | grep "aptos_consensus_current_round"

    curl 127.0.0.1:9101/metrics 2> /dev/null | grep "aptos_consensus_proposals_count"
    ```

    You should expect to see this number keep increasing.
    
5. Finally, the most straight forward way to see if your node is functioning properly is to check if it's making staking reward. You can check it on the explorer, `https://explorer.aptoslabs.com/account/<owner-account-address>?network=testnet`

    ```
    0x1::stake::StakePool

    "active": {
      "value": "100009129447462"
    }
    ```
    
    You should expect the active value for your StakePool to keep increasing. It's updated at every epoch, so it will be every two hours.


## Leaving Validator Set

A node can choose to leave validator set at anytime, or it would happen automatically when there's not sufficient stake on the validator account. To leave validator set, you can perform the following steps:

1. Leave validator set (will take effect in next epoch)

    ```bash
    aptos node leave-validator-set --profile testnet-operator --pool-address <owner-address>
    ```
