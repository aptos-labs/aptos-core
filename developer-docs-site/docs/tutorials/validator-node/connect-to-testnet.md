---
title: "Connecting to Aptos Incentivized Testnet"
slug: "connect-to-testnet"
sidebar_position: 14
---

# Connecting to Aptos Incentivized Testnet

Only do this if you got confirmation email from Aptos team for your eligibility. Nodes not selected will not be included in the genesis, thus not be able to connect to incentivized testnet as a validator node. You can still run public fullnode in this case if you want.

## Using source code

- Stop your node and remove the data directory.
- Download the `genesis.blob` and `waypoint.txt` file published by Aptos Labs team.
- Pull the latest changes on `testnet` branch, make sure you're at commit `317f80bb`
- Restarting the node

## Using Docker

- Stop your node and remove the data volumes, `docker-compose down --volumes`
- Download the `genesis.blob` and `waypoint.txt` file published by Aptos Labs team.
- Update your docker image to use tag `testnet_317f80bb`. Check the image sha256 [here](https://hub.docker.com/layers/validator/aptoslabs/validator/testnet_317f80bb/images/sha256-5184f637f15a9c071475c5bfb3050777c04aa410e9d43c7ff5e7c4a99a55a252?context=explore)
- Restarting the node: `docker-compose up`

## Using Terraform

- Increase `era` number in your Terraform config, this will wipe the data once applied.
- Update your docker image to use tag `testnet_317f80bb` in the Terraform config. Check the image sha256 [here](https://hub.docker.com/layers/validator/aptoslabs/validator/testnet_317f80bb/images/sha256-5184f637f15a9c071475c5bfb3050777c04aa410e9d43c7ff5e7c4a99a55a252?context=explore)
- Apply Terraform: `terraform apply`
- Download the `genesis.blob` and `waypoint.txt` file published by Aptos Labs team.
- Recreate the secrets, make sure the secret name matches your `era` number, e.g. if you have `era = 3`, you should replace the secret name to be `${WORKSPACE}-aptos-node-genesis-e3`
    ```
    export WORKSPACE=<your workspace name>

    kubectl create secret generic ${WORKSPACE}-aptos-node-genesis-e2 \
        --from-file=genesis.blob=genesis.blob \
        --from-file=waypoint.txt=waypoint.txt \
        --from-file=validator-identity.yaml=validator-identity.yaml \
        --from-file=validator-full-node-identity.yaml=validator-full-node-identity.yaml
    ```

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
