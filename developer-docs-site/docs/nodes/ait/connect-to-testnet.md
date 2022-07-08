---
title: "Connecting to Aptos Incentivized Testnet"
slug: "connect-to-testnet"
sidebar_position: 14
---

# Connecting to Aptos Incentivized Testnet

Do this only if you received the confirmation email from Aptos team for your eligibility. Nodes not selected will not have enough tokens to join the testnet. You can still run public fullnode in this case if you want.

## Boostrapping validator node

Before joining the testnet, you need to bootstrap your node with the genesis blob and waypoint provided by Aptos Labs team. This will convert your node from test mode to prod mode.

### Using source code

- Stop your node and remove the data directory.
- Download the `genesis.blob` and `waypoint.txt` file published by Aptos Labs team.
- Pull the latest changes on `testnet` branch, make sure you're at commit `898fdc4f4ae7eb2a7dad0b4da9b293d7510b5732`
- Close the metrics port `9101` and REST API port `80` for your validator and fullnode
- Restarting the node

### Using Docker

- Stop your node and remove the data volumes, `docker compose down --volumes`
- Download the `genesis.blob` and `waypoint.txt` file published by Aptos Labs team.
- Update your docker image to use tag `testnet_898fdc4f4ae7eb2a7dad0b4da9b293d7510b5732`. Check the image sha256 [here](https://hub.docker.com/layers/validator/aptoslabs/validator/testnet_898fdc4f4ae7eb2a7dad0b4da9b293d7510b5732/images/sha256-4d41177e917f3f5d5d8ec77dea343160cdcdfdb79ada8e7ac5b7b5151ff9bd53?context=explore)
- Close metrics port on 9101 and REST API port `80` for your validator and fullnode (remove it from the docker compose file)
- Restarting the node: `docker compose up`

### Using Terraform

- Increase `era` number in your Terraform config, this will wipe the data once applied.
- Update your docker image to use tag `testnet_898fdc4f4ae7eb2a7dad0b4da9b293d7510b5732` in the Terraform config. Check the image sha256 [here](https://hub.docker.com/layers/validator/aptoslabs/validator/testnet_898fdc4f4ae7eb2a7dad0b4da9b293d7510b5732/images/sha256-4d41177e917f3f5d5d8ec77dea343160cdcdfdb79ada8e7ac5b7b5151ff9bd53?context=explore)
- Close metrics port and REST API port for validator and fullnode, add the helm values in your `main.tf ` file, for example:
    ```
    module "aptos-node" {
        ...

        helm_values = {
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
- Recreate the secrets, make sure the secret name matches your `era` number, e.g. if you have `era = 3`, you should replace the secret name to be `${WORKSPACE}-aptos-node-0-genesis-e3`
    ```
    export WORKSPACE=<your workspace name>

    kubectl create secret generic ${WORKSPACE}-aptos-node-0-genesis-e2 \
        --from-file=genesis.blob=genesis.blob \
        --from-file=waypoint.txt=waypoint.txt \
        --from-file=validator-identity.yaml=validator-identity.yaml \
        --from-file=validator-full-node-identity.yaml=validator-full-node-identity.yaml
    ```

## Joining Validator Set

All the selected validator node will be receiving sufficient amount of test token (100,100,000) airdrop from Aptos Labs team to stake their node.

1. Initialize Aptos CLI

    ```
    aptos init --profile ait2 \
    --private-key <account_private_key> \
    --rest-url http://ait2.aptosdev.com \
    --skip-faucet
    ```
    
    Note: `account_private_key` can be found in the `private-keys.yaml` file.

2. Check your validator account balance

    ```
    aptos account list --profile ait2
    ```
    
    This will show you the coin balance you have in the validator account. You should be able to see something like:
    
    ```
    "coin": {
        "value": "100100000"
      }
    ```

3. Register validator candidate on chain

    ```
    aptos node register-validator-candidate \
    --profile ait2 \
    --validator-config-file aptosbot.yaml
    ```

    Replace `aptosbot.yaml` with your validator node config file.

4. Add stake to your validator node

    ```
    aptos node add-stake --amount 100000000 --profile ait2
    ```

    Please don't add too much stake to make sure you still have sufficient token to pay gas fee.

5. Set lockup time for your stake, minimal of 72 hours is required to join validator set.

    ```
    aptos node increase-lockup \
    --profile ait2 \
    --lockup-duration 75h
    ```

6. Join validator set

    ```
    aptos node join-validator-set --profile ait2
    ```

    ValidatorSet will be updated at every epoch change, which is **once every hour**. You will only see your node joining the validator set in next epoch. Both Validator and fullnode will start syncing once your validator is in the validator set.

7. Check validator set

    ```
    aptos node show-validator-set --profile ait2 | jq -r '.Result.pending_active' | grep <account_address>
    ```
    
    You should be able to see your validator node in "pending_active" list. And when the next epoch change happens, the node will be moved into "active_validators" list. This should happen within one hour from the completion of previous step. During this time, you might see errors like "No connected AptosNet peers", which is normal.
    
    ```
    aptos node show-validator-set --profile ait2 | jq -r '.Result.active_validators' | grep <account_address>
    ```


## Verify node connections

You can check the details about node liveness definition [here](https://aptos.dev/reference/node-liveness-criteria/#verifying-the-liveness-of-your-node). Once your validator node joined the validator set, you can verify the correctness following those steps:

1. Verify that your node is connecting to other peers on testnet. (Replace `127.0.0.1` with your Validator IP/DNS if deployed on the cloud)

    ```
    curl 127.0.0.1:9101/metrics 2> /dev/null | grep "aptos_connections{.*\"Validator\".*}"
    ```

    The command will output the number of inbound and outbound connections of your Validator node. For example:

    ```
    aptos_connections{direction="inbound",network_id="Validator",peer_id="2a40eeab",role_type="validator"} 5
    aptos_connections{direction="outbound",network_id="Validator",peer_id="2a40eeab",role_type="validator"} 2
    ```

    As long as one of the metrics is greater than zero, your node is connected to at least one of the peers on the testnet.

2. You can also check if your node is connected to AptosLabs's node, replace `<Aptos Peer ID>` with the peer ID shared by Aptos team.

    ```
    curl 127.0.0.1:9101/metrics 2> /dev/null | grep "aptos_network_peer_connected{.*remote_peer_id=\"<Aptos Peer ID>\".*}"
    ```

3. Once your node state sync to the latest version, you can also check if consensus is making progress

    ```
    curl 127.0.0.1:9101/metrics 2> /dev/null | grep "aptos_consensus_current_round"
    ```

    You should expect to see this number keep increasing.
