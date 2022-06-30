---
title: "Connecting to Aptos Incentivized Testnet"
slug: "connect-to-testnet"
sidebar_position: 14
---

# Connecting to Aptos Incentivized Testnet

Only do this if you got confirmation email from Aptos team for your eligibility. Nodes not selected will not have enough tokens to join the testnet. You can still run public fullnode in this case if you want.

## Boostrapping validator node

Before joining the testnet, you need to bootstrap your node with the genesis blob and waypoint provided by Aptos Labs team. This will convert your node from test mode to prod mode.

### Using source code

- Stop your node and remove the data directory.
- Download the `genesis.blob` and `waypoint.txt` file published by Aptos Labs team.
- Pull the latest changes on `testnet` branch, make sure you're at commit `3b53225b5a1effcce5dee9597a129216510dc424`
- Close the metrics port `9101` for your validator and fullnode
- Restarting the node

### Using Docker

- Stop your node and remove the data volumes, `docker compose down --volumes`
- Download the `genesis.blob` and `waypoint.txt` file published by Aptos Labs team.
- Update your docker image to use tag `testnet_3b53225b5a1effcce5dee9597a129216510dc424`. Check the image sha256 [here](https://hub.docker.com/layers/validator/aptoslabs/validator/testnet_3b53225b5a1effcce5dee9597a129216510dc424/images/sha256-1625f70457a6060ae2e64e699274e1ddca02cb7856406a40d1891b6bf84ae072?context=explore)
- Close metrics port on 9101 for your validator and fullnode (remove it from the docker compose file)
- Restarting the node: `docker compose up`

### Using Terraform

- Increase `era` number in your Terraform config, this will wipe the data once applied.
- Update your docker image to use tag `testnet_3b53225b5a1effcce5dee9597a129216510dc424` in the Terraform config. Check the image sha256 [here](https://hub.docker.com/layers/validator/aptoslabs/validator/testnet_3b53225b5a1effcce5dee9597a129216510dc424/images/sha256-1625f70457a6060ae2e64e699274e1ddca02cb7856406a40d1891b6bf84ae072?context=explore)
- Close metrics port for validator and fullnode, add the helm values in your `main.tf ` file, for example:
    ```
    module "aptos-node" {
        ...

        helm_values = {
            service = {
            validator = {
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

All the selected validator node will be receiving sufficient amount of test token (101,000,000) airdrop from Aptos Labs team to stake their node.

1. Initialize Aptos CLI

    ```
    aptos init --profile ait2 \
    --private-key <account-private-key> \
    --rest-url http://ait2.aptosdev.com \
    --faucet-url http://ait2.aptosdev.com \
    --assume-yes
    ```

2. Register validator candidate on chain

    ```
    aptos node register-validator-candidate \
    --profile ait2 \
    --validator-config-file aptosbot.yaml
    ```

    Replace `aptosbot.yaml` with your validator node config file.

3. Add stake to your validator node

    ```
    aptos node add-stake --amount 100000000 --profile ait2
    ```

    Please don't add too much stake to make sure you still have sufficient token to pay gas fee.

4. Set lockup time for your stake, minimal of 72 hours is required to join validator set.

    ```
    aptos node increase-lockup \
    --profile ait2 \
    --lockup-duration 75h
    ```

5. Join validator set

    ```
    aptos -- node join-validator-set --profile ait2
    ```

    ValidatorSet will be updated at every epoch change, which is **once every hour**. You will only see your node joining the validator set in next epoch. Both Validator and fullnode will start syncing once your validator is in the validator set.


## Verify node connections

You can check the details about node liveness definition [here](https://aptos.dev/reference/node-liveness-criteria/#verifying-the-liveness-of-your-node).

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

3. Once we have enough nodes coming online to form consensus, you can also check if consensus is making progress

    ```
    curl 127.0.0.1:9101/metrics 2> /dev/null | grep "aptos_consensus_current_round"
    ```

    You should expect to see this number keep increasing.
