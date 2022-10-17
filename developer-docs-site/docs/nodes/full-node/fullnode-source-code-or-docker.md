---
title: "Fullnode Using Aptos Source or Docker"
slug: "fullnode-source-code-or-docker"
sidebar_position: 10
---

# Fullnode Using Aptos Source or Docker

You can run your own [Fullnode](/concepts/basics-fullnodes) to synchronize with the state of the Aptos blockchain and stay up-to-date. Fullnodes replicate the entire state of the blockchain by querying other Aptos fullnodes or validators.

Alternatively, you can use the fullnodes provided by Aptos Labs. However, such Aptos Labs-provided fullnodes have rate limits, which can impede your development. By running your own fullnode you can directly synchronize with the Aptos blockchain and avoid such rate limits.

Fullnodes can be run by anyone. This tutorial explains how to configure a public fullnode to connect to the Aptos devnet.

:::tip Default connection to devnet
If you follow the default setup in this document, then your public fullnode will be connected to the Aptos devnet with a REST endpoint accessible on your computer at localhost:8080. To connect to a different Aptos network, such as devnet or testnet, make sure to change the settings from `devnet` to other networks. To connect to other networks, you can find genesis and waypoint here ➜ https://github.com/aptos-labs/aptos-networks.
:::

## Before you proceed

Before you get started with this tutorial, read the following sections:

* [Validator node concepts](/concepts/basics-validator-nodes).
* [Fullnode concepts](/concepts/basics-fullnodes).
* [REST specifications](https://fullnode.devnet.aptoslabs.com/v1/spec#/).


## Hardware requirements

We recommend the following hardware resources:

- For running a production grade fullnode:

  - **CPU**: 8 cores, 16 threads (Intel Xeon Skylake or newer).
  - **Memory**: 32GB RAM.

- For running the fullnode for development or testing:

  - **CPU**: 2 cores.
  - **Memory**: 4GB RAM.

## Storage requirements

The amount of data stored by Aptos depends on the ledger history (length) of the blockchain and the number of on-chain states (e.g., accounts). These values depend on several factors, including: the age of the blockchain, the average transaction rate and the configuration of the ledger pruner. Also see [Validator Hardware Requirements](/nodes/validator-node/operator/node-requirements#hardware-requirements).

:::tip
Given that devnet is currently being reset on a weekly basis, we estimate that Aptos will not require more than several GBs of storage. See the `#devnet-release` channel on Aptos Discord.
:::

## Configuring a fullnode

You can configure a public fullnode in one of two ways:

1. Building and running [aptos-core](https://github.com/aptos-labs/aptos-core) from source code.
2. Using Docker.

This document describes how to configure your public fullnode using both methods.

### Approach #1: Building and running from Aptos-core source code

1. Clone the Aptos repo.

    ```bash
    git clone https://github.com/aptos-labs/aptos-core.git
    ```

2. `cd` into `aptos-core` directory.

    ```bash
    cd aptos-core
    ```

3. Run the `scripts/dev_setup.sh` Bash script as shown below. This will prepare your developer environment.

    ```bash
    ./scripts/dev_setup.sh
    ```

4. Update your current shell environment.

    ```bash
    source ~/.cargo/env
    ```

With your development environment ready, now you can start to setup your fullnode.

5. Checkout the `devnet` branch using `git checkout --track origin/devnet`.

6. Make sure your current working directory is `aptos-core`.

   Run:
   ```bash
   cp config/src/config/test_data/public_full_node.yaml fullnode.yaml
   ```
   to create a copy of the fullnode config template. You will edit this file to ensure that your fullnode:

    - Contains the correct genesis blob that is published by the Aptos devnet.
    - Synchronizes correctly with the devnet, by using the checkpoint file `waypoint.txt` published by the devnet, and
    - Stores the devnet database at a location of your choice on your local machine.

7. Make sure your current working directory is `aptos-core`. The Aptos devnet publishes the `genesis.blob` and `waypoint.txt` files. Download them:

    - Click [genesis][devnet_genesis] for genesis blob or run the below command on your terminal:
      ```bash
      curl -O https://devnet.aptoslabs.com/genesis.blob
      ```

    - Click [waypoint][devnet_waypoint] for waypoint.txt and save the file, or run the below command on your terminal:
      ```bash
      curl -O https://devnet.aptoslabs.com/waypoint.txt
      ```
  
    :::tip
    To connect to other networks, you can find genesis and waypoint here ➜ https://github.com/aptos-labs/aptos-networks
    :::

8. Edit the `fullnode.yaml` file in your current working directory as follows.

    - Specify the correct path to the `waypoint.txt` you just downloaded by editing the `base.waypoint.from_file` in the `fullnode.yaml`. By default it points to `waypoint.txt` in the current working directory.

    For example:
      ```yaml
      base:
        waypoint:
          from_file: "./waypoint.txt"
      ```

    - For the `genesis_file_location` key, provide the full path to the `genesis.blob` file. For example:

      ```yaml
      genesis_file_location: "./genesis.blob"
      ```

    - For the `data_dir` key in the `base` list, specify the directory where on your local computer you want to store the devnet database. This can be anywhere on your computer. For example, you can create a directory `my-full-node/data` in your home directory and specify it as:

      ```yaml
      data_dir: "</path/to/my/homedir/my-full-node/data>"
      ```

9. Start your local public fullnode by running the below command:

  ```bash
  cargo run -p aptos-node --release -- -f ./fullnode.yaml
  ```

You have now successfully configured and started running a fullnode connected to Aptos devnet.

:::tip Debugging?
This will build a release binary: `aptos-core/target/release/aptos-node`. The release binaries tend to be substantially faster than debug binaries but lack debugging information useful for development. To build a debug binary, omit the `--release` flag.
:::

### Approach #2: Using Docker

This section describes how to configure and run your fullnode using Docker.

:::caution Supported only on x86-64 CPUs
Running Aptos-core via Docker is currently only supported on x86-64 CPUs and not on ARM64 CPUs (which includes M1/M2 Macs).

We currently only publish Docker images compatible with x86-64 CPUs.
If you have an M1/M2 (ARM64) Mac, use the Aptos-core source approach.
If M1/M2 support is important to you, please comment on and follow this issue: https://github.com/aptos-labs/aptos-core/issues/1412
:::

1. Install [Docker](https://docs.docker.com/get-docker/).
2. Create a directory for your local public fullnode, and `cd` into it.
   For example:
   ```bash
   mkdir aptos-fullnode && cd aptos-fullnode
   ```
3. Run the following script to prepare your local config and data dir for Devnet:
    ```bash
    mkdir data && \
    curl -O https://raw.githubusercontent.com/aptos-labs/aptos-core/devnet/config/src/config/test_data/public_full_node.yaml && \
    curl -O https://devnet.aptoslabs.com/waypoint.txt && \
    curl -O https://devnet.aptoslabs.com/genesis.blob
    ```

    :::tip
    To connect to other networks, you can find genesis and waypoint here ➜ https://github.com/aptos-labs/aptos-networks
    :::

4. Finally, start the fullnode via Docker:
   ```bash
    docker run --pull=always --rm -p 8080:8080 -p 9101:9101 -p 6180:6180 -v $(pwd):/opt/aptos/etc -v $(pwd)/data:/opt/aptos/data --workdir /opt/aptos/etc --name=aptos-fullnode aptoslabs/validator:devnet aptos-node -f /opt/aptos/etc/public_full_node.yaml
   ```
Ensure you have opened the relevant ports: 8080, 9101 and 6180. You may also need to update the 127.0.0.1 with 0.0.0.0 in the `public_full_node.yaml` for the fields `listen_address` and `address` field in the `api` list.

## Verify the correctness of your fullnode

### Verify initial synchronization

During the initial synchronization of your fullnode, there may be a lot of data to transfer. You can monitor the progress by querying the metrics port to see what version your node is currently synced to. Run the following command to see the current synced version of your node:

```bash
curl 127.0.0.1:9101/metrics 2> /dev/null | grep "aptos_state_sync_version{.*\"synced\"}" | awk '{print $2}'
```

The command will output the current synced version of your node. For example:

```bash
71000
```

Compare the synced version returned by this command (e.g., `71000`) with the `Current Version` (latest) shown on the
[Aptos status page](https://status.devnet.aptos.dev/). If your node is catching up to the current version, it is synchronizing correctly.

:::tip
It is fine if the status page differs by a few versions, as the status
page does not automatically refresh.
:::

### (Optional) Verify outbound network connections

Optionally, you can check the output network connections. The number of outbound network connections should be more than `0`. Run the following command:

```bash
curl 127.0.0.1:9101/metrics 2> /dev/null | grep "aptos_connections{direction=\"outbound\""
```

The above command will output the number of outbound network connections for your node. For example:

```bash
curl 127.0.0.1:9101/metrics 2> /dev/null | grep "aptos_connections{direction=\"outbound\""
aptos_connections{direction="outbound",network_id="Public",peer_id="aabd651f",role_type="full_node"} 3
```

If the number of outbound connections returned is `0`, then it means your node cannot connect to the Aptos blockchain. If this happens to you, follow these steps to resolve the issue:

1. Update your node to the latest release by following the [Update Fullnode With New Devnet Releases](/nodes/full-node/update-fullnode-with-new-devnet-releases).
2. Remove any `seed` peers you may have added to your `public_full_node.yaml` configuration file. The seeds may be preventing you from connecting to the network. Seed peers are discussed in the [Connecting your fullnode to seed peers](/nodes/full-node/fullnode-network-connections#connecting-your-fullnode-to-seed-peers) section.

### (Optional) Examine Docker ledger size

The blockchain ledger's volume for Aptos devnet can be monitored by entering the Docker container ID and checking the size.
This will allow you to see how much storage the blockchain ledger is currently consuming.

- First, run `docker container ls` on your terminal and copy the NAME field output. This will be a string similar to `public_full_node_fullnode_1`.
- Next, run these commands to check the storage size consumed by the ledger, using the NAME field you copied over in place of `public_full_node_fullnode_1`:

```bash
# Obtain the container ID:
id=$(docker container ls | grep public_full_node_fullnode_1 | grep -oE "^[0-9a-zA-Z]+")
# Enter the container:
docker exec -it $id /bin/bash
# Observe the volume (ledger) size:
du -cs -BM /opt/aptos/data
```

[rest_spec]: https://github.com/aptos-labs/aptos-core/tree/main/api
[devnet_genesis]: https://devnet.aptoslabs.com/genesis.blob
[devnet_waypoint]: https://devnet.aptoslabs.com/waypoint.txt
[aptos-labs/aptos-core]: https://github.com/aptos-labs/aptos-core.git
[status dashboard]: https://status.devnet.aptos.dev
