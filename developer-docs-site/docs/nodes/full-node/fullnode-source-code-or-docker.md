---
title: "Run a Fullnode with Source or Docker"
slug: "fullnode-source-code-or-docker"
sidebar_position: 10
---

# Run a Public Fullnode with the Aptos Source Code or Docker

You can run your own [public fullnode](../../concepts/fullnodes.md) to synchronize with the state of the Aptos blockchain and stay up-to-date. Public fullnodes replicate the entire state of the blockchain by querying other Aptos fullnodes (public fullnodes or validator fullnodes) or validators.

Alternatively, you can use the public fullnodes provided by Aptos Labs. However, such Aptos Labs-provided public fullnodes have rate limits, which can impede your development. By running your own public fullnode you can directly synchronize with the Aptos blockchain and avoid such rate limits.

Public fullnodes can be run by anyone. This tutorial explains how to configure a public fullnode to connect to an Aptos network.

:::caution Choose a network
This document describes how to start a public fullnode in the Aptos `mainnet` network yet can easily be used to do the same in the `devnet` and `testnet` networks. To do so, instead check out the desired branch and use the `genesis.blob` and `waypoint.txt` node files for the respective branch: [`mainnet`](../node-files-all-networks/node-files.md), [`devnet`](../node-files-all-networks/node-files-devnet.md), and [`testnet`](../node-files-all-networks/node-files-testnet.md).
:::

:::tip Starting a node in testnet?
If this is the first time you're starting a fullnode in `testnet`, it is recommended to bootstrap your node first by restoring from a [backup](../full-node/aptos-db-restore.md) or downloading [a snapshot](../full-node/bootstrap-fullnode.md). This will avoid any potential issues with network connectivity and peer discovery.
:::

## Hardware requirements

We recommend the following hardware resources:

- For running a production grade public fullnode:

  - **CPU**: 8 cores, 16 threads (Intel Xeon Skylake or newer).
  - **Memory**: 32GB RAM.

- For running the public fullnode for development or testing:

  - **CPU**: 2 cores.
  - **Memory**: 4GB RAM.

## Storage requirements

The amount of data stored by Aptos depends on the ledger history (length) of the blockchain and the number of on-chain states (e.g., accounts). These values depend on several factors, including: the age of the blockchain, the average transaction rate and the configuration of the ledger pruner. Follow the storage requirements described in [Validator Hardware Requirements](../validator-node/operator/node-requirements.md#hardware-requirements). 

:::tip Devnet blockchain storage
The Aptos devnet is currently reset on a weekly basis. Hence we estimate that if you are connecting to the devnet, then the Aptos blockchain will not require more than several GBs of storage. See the `#devnet-release` channel on Aptos Discord.
:::

## Configuring a public fullnode

You can configure a public fullnode in one of two ways:

1. Building and running [aptos-core](https://github.com/aptos-labs/aptos-core) from source code.
2. Using Docker.

This document describes how to configure your public fullnode using both methods.

### Method 1: Building and running from source

See [Building Aptos From Source](../../guides/building-from-source.md)

1. Check out the `mainnet` branch using `git checkout --track origin/mainnet`; remember, you may instead use `devnet` or `testnet`.

1. Make sure your current working directory is `aptos-core`.

   Run:
   ```bash
   cp config/src/config/test_data/public_full_node.yaml fullnode.yaml
   ```
   to create a copy of the public fullnode configuration YAML template. You will edit this file to ensure that your public fullnode:

    - Contains the correct genesis blob that is published by the Aptos mainnet.
    - Synchronizes correctly with the mainnet, by using the checkpoint file `waypoint.txt` published by the mainnet. 
    - Stores the mainnet database at a location of your choice on your local machine.

1. Make sure your current working directory is `aptos-core`. The Aptos mainnet publishes the `genesis.blob` and `waypoint.txt` files. Download them:

    - Run the below command on your terminal to download the file:
      ```bash
      curl -O https://raw.githubusercontent.com/aptos-labs/aptos-networks/main/mainnet/genesis.blob
      ```

    - Run the below command on your terminal to download the file:
      ```bash
      curl -O https://raw.githubusercontent.com/aptos-labs/aptos-networks/main/mainnet/waypoint.txt
      ```
  
    :::caution Don't want to connect to mainnet?
    To connect to other networks (e.g., `devnet` and `testnet`), you can find genesis and waypoint here ➜ https://github.com/aptos-labs/aptos-networks.
    Be sure to download the `genesis.blob` and `waypoint.txt` for those networks, instead of using the genesis
    and waypoint pointed to by the `curl` commands above.
    :::

1. Edit the `fullnode.yaml` file in your current working directory as follows.

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

1. Start your local public fullnode by running the below command:

  ```bash
  cargo run -p aptos-node --release -- -f ./fullnode.yaml
  ```

You have now successfully configured and started running a fullnode connected to Aptos devnet.

:::tip Debugging?
This will build a release binary: `aptos-core/target/release/aptos-node`. The release binaries tend to be substantially faster than debug binaries but lack debugging information useful for development. To build a debug binary, omit the `--release` flag.

You can also run this directly as `./aptos-core/target/release/aptos-node -f ./fullnode.yaml` after running `cargo build -p aptos-node --release`
:::

---

### Method 2: Using Docker

This section describes how to configure and run your public fullnode using Docker.

:::danger Supported only on x86-64 CPUs
Running Aptos-core via Docker is currently only supported on x86-64 CPUs. If you have an Apple M1/M2 (ARM64) Mac, use the Aptos-core source approach. If M1/M2 support is important to you, comment on this issue: https://github.com/aptos-labs/aptos-core/issues/1412
:::

1. Install [Docker](https://docs.docker.com/get-docker/).
2. Run the following script to prepare your local configuration and data directory for mainnet:
```bash
mkdir mainnet && cd mainnet
mkdir data && \
curl -O https://raw.githubusercontent.com/aptos-labs/aptos-core/mainnet/docker/compose/aptos-node/fullnode.yaml && \
curl -O https://raw.githubusercontent.com/aptos-labs/aptos-networks/main/mainnet/waypoint.txt && \
curl -O https://raw.githubusercontent.com/aptos-labs/aptos-networks/main/mainnet/genesis.blob
```

3. Make sure that the `fullnode.yaml` configuration file that you downloaded contains only the following configuration content. This will ensure that this configuration is for public fullnode and not for either a validator node or a validator fullnode:

```yaml
base:
  role: "full_node"
  data_dir: "/opt/aptos/data"
  waypoint:
    from_file: "/opt/aptos/etc/waypoint.txt"

execution:
  genesis_file_location: "/opt/aptos/etc/genesis.blob"

full_node_networks:
- network_id: "public"
  discovery_method: "onchain"
  listen_address: "/ip4/0.0.0.0/tcp/6182"

api:
  enabled: true
  address: "0.0.0.0:8080"
```

**NOTE**: Set `listen_address: "/ip4/127.0.0.1/tcp/6182"` if you do not want other full nodes connecting to yours. Also see the below note.

4. Run the below `docker` command. **NOTE** the `mainnet` tag always refers to the latest official Docker image tag. You can find the latest hash for comparison at:
https://github.com/aptos-labs/aptos-networks/tree/main/mainnet

```bash
docker run --pull=always \
    --rm -p 8080:8080 \
    -p 9101:9101 -p 6180:6180 \
    -v $(pwd):/opt/aptos/etc -v $(pwd)/data:/opt/aptos/data \
    --workdir /opt/aptos/etc \
    --name=aptos-fullnode aptoslabs/validator:mainnet aptos-node \
    -f /opt/aptos/etc/fullnode.yaml
```

**NOTE**: You may need to prefix the command with `sudo` depending on your configuration

**NOTE**: Ensure you have opened the relevant ports: 8080, 9101 and 6180. You may also need to update the 127.0.0.1 with 0.0.0.0 in the `fullnode.yaml` for the fields `listen_address` and `address` field in the `api` list.

:::caution Don't want to connect to mainnet?
To connect to other networks (e.g., `devnet` and `testnet`), you can find genesis and waypoint here ➜ https://github.com/aptos-labs/aptos-networks.
Be sure to download the `genesis.blob` and `waypoint.txt` for those networks, instead of using the genesis
and waypoint pointed to by the `curl` commands above.
:::

Ensure you have opened the relevant ports: 8080, 9101 and 6180. You may also need to update the 127.0.0.1 with 0.0.0.0 in the `fullnode.yaml` for the fields `listen_address` and `address` field in the `api` list.

## Verify the correctness of your public fullnode

### Verify initial synchronization

During the initial synchronization of your public fullnode, there may be a lot of data to transfer. You can monitor the progress by querying the metrics port to see what version your node is currently synced to. Run the following command to see the current synced version of your node:

```bash
curl 127.0.0.1:9101/metrics 2> /dev/null | grep "aptos_state_sync_version{.*\"synced\"}" | awk '{print $2}'
```

The command will output the current synced version of your node. For example:

```bash
71000
```

Compare the synced version returned by this command (e.g., `71000`) with the highest version shown on the
[Aptos explorer page](https://explorer.aptoslabs.com/?network=mainnet). If your node is catching up to the highest version, it is synchronizing correctly.

:::tip
It is fine if the explorer page differs by a few versions, as the explorer nodes may sync with some variance.
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

1. Update your node to the latest release by following the [Update Fullnode With New Devnet Releases](./update-fullnode-with-new-releases.md).
2. Remove any `seed` peers you may have added to your `public_full_node.yaml` configuration file. The seeds may be preventing you from connecting to the network. Seed peers are discussed in the [Connecting your fullnode to seed peers](./fullnode-network-connections.md#connecting-your-fullnode-to-seed-peers section.

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

## Upgrade your public fullnode

When receiving an update from Aptos for your fullnode, take these measures to minimize downtime. In all cases, you are essentially undoing setup and restarting anew. So first make sure your development environment is up to date.

### Upgrading from source

If you created your Aptos fullnode from source, you should similarly upgrade from source:
1. Stop your local public fullnode by running the below command:
  ```bash
  cargo stop aptos-node
  ```
1. Delete the `waypoint.txt`, `genesis.blob` and `fullnode.yaml` files previously downloaded, installed and configured.
1. Re-install and configure those files as during setup.
1. Restart your local public fullnode by running the same start (`run`) command as before:
  ```bash
  cargo run -p aptos-node --release -- -f ./fullnode.yaml
  ```

  ### Upgrading with Docker

  If you created your Aptos fullnode with Docker, you should similarly upgrade with Docker:
  1. Stop your local public fullnode by running the below command:
    ```bash
    docker-compose down --volumes
    ```
  1. Delete the `waypoint.txt`, `genesis.blob` and `fullnode.yaml` files previously downloaded, installed and configured.
  1. Re-install and configure those files as during setup.
  1. Restart your local public fullnode by running the same start (`run`) command as before:
  ```bash
  docker run --pull=always \
      --rm -p 8080:8080 \
      -p 9101:9101 -p 6180:6180 \
      -v $(pwd):/opt/aptos/etc -v $(pwd)/data:/opt/aptos/data \
      --workdir /opt/aptos/etc \
      --name=aptos-fullnode aptoslabs/validator:mainnet aptos-node \
      -f /opt/aptos/etc/fullnode.yaml
  ```
