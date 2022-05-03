---
title: "Run a FullNode"
slug: "run-a-fullnode"
sidebar_position: 10
---

# Run a FullNode

You can run your own [FullNode](/basics/basics-fullnodes) to synchronize with the state of the Aptos Blockchain and stay up-to-date. FullNodes replicate the entire state of the blockchain by querying other Aptos FullNodes or validators.

Alternatively, you can use the FullNodes provided by Aptos Labs. However, such Aptos Labs-provided FullNodes have rate limits, which can impede your development. By running your own FullNode you can directly synchronize with the Aptos Blockchain and avoid such rate limits.

FullNodes can be run by anyone. This tutorial explains how to configure a public FullNode to connect to the Aptos devnet.

:::tip

Your public FullNode will be connected to the Aptos devnet with a REST endpoint accessible on your computer at localhost:8080.

:::

## Before you proceed

Before you get started with this tutorial, read the following sections:

* [Validator node concepts](/basics/basics-validator-nodes).
* [FullNode concepts](/basics/basics-fullnodes).
* [REST specifications][rest_spec].

:::caution Docker support only on Linux

Docker container is currently supported only on Linux x86-64 platform. If you are on macOS or Windows platform, use the Aptos-core source approach.

:::

## Hardware requirements

We recommend the following hardware resources:

- For running a production grade FullNode:

  - **CPU**: 4 cores (Intel Xeon Skylake or newer).
  - **Memory**: 8GiB RAM.

- For running the FullNode for development or testing:

  - **CPU**: 2 cores.
  - **Memory**: 4GiB RAM.

## Storage requirements

The amount of data stored by Aptos depends on the ledger history (length) of the blockchain and the number
of on-chain states (e.g., accounts). These values depend on several factors, including: the age of the blockchain,
the average transaction rate and the configuration of the ledger pruner.

:::tip

Given that devnet is currently being reset on a weekly basis, we estimate that Aptos will not require more than several GBs of storage. See the `#devnet-release` channel on Aptos Discord.

:::

## Configuring a FullNode

You can configure a public FullNode in two ways:

1. Using the [aptos-core](https://github.com/aptos-labs/aptos-core) source code.
2. Using Docker.

This document describes how to configure your public FullNode using both the methods.

### Using Aptos-core source code

1. Fork and clone the Aptos repo.

    - Fork the Aptos Core repo by clicking on the **Fork** on the top right of this repo page: https://github.com/aptos-labs/aptos-core.
    - Clone your fork.

      ```
      git clone https://github.com/<YOUR-GITHUB-USERID>/aptos-core

      ```

2. `cd` into `aptos-core` directory.

    ```
    cd aptos-core
    ```

3. Run the `scripts/dev_setup.sh` Bash script as shown below. This will prepare your developer environment.

    ```
    ./scripts/dev_setup.sh
    ```

4. Update your current shell environment.

    ```
    source ~/.cargo/env
    ```

With your development environment ready, now you can start to setup your FullNode.

5. Checkout the `devnet` branch using `git checkout --track origin/devnet`.

6. Make sure your current working directory is `aptos-core`. Copy the YAML configuration file from `config/src/config/test_data/public_full_node.yaml` to your current working directory. You will edit this file to ensure that your FullNode:

    - Contains the correct genesis blob that is published by the Aptos devnet.
    - Synchronizes correctly with the devnet, by using the checkpoint file `waypoint.txt` published by the devnet, and
    - Stores the devnet database at a location of your choice on your local machine.

7. Make sure your current working directory is `aptos-core`. The Aptos devnet publishes the `genesis.blob` and `waypoint.txt` files. Download them:

    - Click here [genesis][devnet_genesis] or run the below command on your terminal:
      ```
      wget https://devnet.aptoslabs.com/genesis.blob
      ```

    - Click here [waypoint][devnet_waypoint] and save the file, or run the below command on your terminal:
      ```
      wget https://devnet.aptoslabs.com/waypoint.txt
      ```

8. Edit the `aptos-core/public_full_node.yaml` file in your current working directory as follows. See the example YAML file in: `docker/compose/public_full_node/public_full_node.yaml`.

    - Copy and paste the contents of the `waypoint.txt` file into the `from_config` field of the `waypoint` list. For example:

      ```
      $ cat waypoint.txt
      0:683990e3bdc1bbf0204fc4a564e07e628229c913c8cc6ad96d8b30f9446233cb
      ```
      Copy paste the above contents as below:
      ```
      waypoint:
        from_config: "0:683990e3bdc1bbf0204fc4a564e07e628229c913c8cc6ad96d8b30f9446233cb"
      ```

    - For the `genesis_file_location` key, provide the full path to the `genesis.blob` file. For example:

      ```
      genesis_file_location: "/path/to/aptos-core/genesis.blob"
      ```

    - For the `data_dir` key in the `base` list, specify the directory where on your local computer you want to store the devnet database. This can be anywhere on your computer. For example, you can create a directory `my-full-node/data` in your home directory and specify it as:

      ```
      data_dir: "/path/to/my/homedir/my-full-node/data"
      ```

9. Start your local FullNode by running the below command:

  ```
  cargo run -p aptos-node --release -- -f ./public_full_node.yaml
  ```

You have now successfully configured and started running a FullNode connected to Aptos devnet.

:::note

This will build a release binary: `aptos-core/target/release/aptos-node`. The release binaries tend to be substantially faster than debug binaries but lack debugging information useful for development. To build a debug binary, omit the `--release` flag.

:::

### Using Docker

This section describes how to configure and run your FullNode using Docker.

1. Install [Docker](https://docs.docker.com/get-docker/) including [Docker-Compose](https://docs.docker.com/compose/install/).
2. Create a directory for your local public FullNode, and `cd` into it.
3. Download the following YAML configuration files:

    - Click on [Public FullNode Docker Compose](https://github.com/aptos-labs/aptos-core/tree/main/docker/compose/public_full_node/docker-compose.yaml) and save the file, or run the below command on your terminal:
    ```
    wget https://raw.githubusercontent.com/aptos-labs/aptos-core/main/docker/compose/public_full_node/docker-compose.yaml
    ```

    and

    - Click on [Public FullNode Aptos-core](https://github.com/aptos-labs/aptos-core/tree/main/docker/compose/public_full_node/public_full_node.yaml) and save the file, or run the below command on your terminal:
    ```
    wget https://raw.githubusercontent.com/aptos-labs/aptos-core/main/docker/compose/public_full_node/public_full_node.yaml
    ```
4. The Aptos devnet publishes the `genesis.blob` and `waypoint.txt` files. Download them:

    - Click on [genesis][devnet_genesis] or run the below command on your terminal:
      ```
      wget https://devnet.aptoslabs.com/genesis.blob
      ```

    - Click on [waypoint][devnet_waypoint] and save the file, or run the below command on your terminal:
      ```
      wget https://devnet.aptoslabs.com/waypoint.txt
      ```

5. Start Docker Compose by running the command:

    ```
    docker-compose up
    ```

## Verify the correctness of your FullNode

### Verify initial synchronization

During the initial synchronization of your FullNode, there may be a lot of data to transfer. You can monitor the progress by querying the metrics port to see what version your node is currently synced to. Run the following command to see the current synced version of your node:

```
curl 127.0.0.1:9101/metrics 2> /dev/null | grep "aptos_state_sync_version{.*\"synced\"}" | awk '{print $2}'
```

The command will output the current synced version of your node. For example:

```
$ 71000
```

Compare the synced version returned by this command (e.g., `71000`) with the `Current Version` (latest) shown on the
[Aptos status page](https://status.devnet.aptos.dev/). If your node is catching up to the current version, it is synchronizing correctly.

:::note

It is fine if the status page differs by a few versions, as the status
page does not automatically refresh.

:::

### (Optional) Verify outbound network connections

Optionally, you can check the output network connections. The number of outbound network connections should be more than `0`. Run the following command:

```
curl 127.0.0.1:9101/metrics 2> /dev/null | grep "aptos_connections{direction=\"outbound\""
```

The above command will output the number of outbound network connections for your node. For example:

```
$ curl 127.0.0.1:9101/metrics 2> /dev/null | grep "aptos_connections{direction=\"outbound\""
aptos_connections{direction="outbound",network_id="Public",peer_id="aabd651f",role_type="full_node"} 3
```

If the number of outbound connections returned is `0`, then it means your node cannot connect to the Aptos blockchain. If this happens to you, follow these steps to resolve the issue:

1. Update your node to the latest release by following the [update instructions](#update-fullnode-with-new-releases).
2. Remove any `seed` peers you may have added to your `public_full_node.yaml` configuration file. The seeds may be preventing you from connecting to the network. Seed peers are discussed in the [Add upstream seed peers](#add-upstream-seed-peers) section.

### (Optional) Examine Docker ledger size

The blockchain ledger's volume for Aptos devnet can be monitored by entering the Docker container ID and checking the size.
This will allow you to see how much storage the blockchain ledger is currently consuming.

- First, run `docker container ls` on your terminal and copy the NAME field output. This will be a string similar to `public_full_node_fullnode_1`.
- Next, run these commands to check the storage size consumed by the ledger, using the NAME field you copied over in place of `public_full_node_fullnode_1`:

```
# Obtain the container ID:
id=$(docker container ls | grep public_full_node_fullnode_1 | grep -oE "^[0-9a-zA-Z]+")
# Enter the container:
docker exec -it $id /bin/bash
# Observe the volume (ledger) size:
du -cs -BM /opt/aptos/data
```

## Add upstream seed peers

:::tip

You may see `NoAvailablePeers` in your node's error messages. This is normal when the node is first starting.
Wait for the node to run for a few minutes to see if it connects to peers. If not, follow the below steps:

:::

Devnet validator FullNodes will only accept a maximum of connections. If Aptos devnet is experiencing high network connection volume, your FullNode might not able to connect and you may see `NoAvailablePeers` continuously in your node's error messages. If this happens, manually add peer addresses in the `seeds` key in `public_full_node.yaml`, the FullNode configuration file. This will then connect your FullNode to the specified seed peer.

See below for a few seed peer addresses you can use in your `public_full_node.yaml` file.

:::tip

You can also use the FullNode addresses provided by the Aptos community. Anyone already running a FullNode can provide their address for you to connect. See the channel `#advertise-full-nodes` in Aptos Discord.

:::

Add these to your `public_full_node.yaml` configuration file under your `discovery_method`, as shown in the below example:

```
...
full_node_networks:
    - discovery_method: "onchain"
      # The network must have a listen address to specify protocols. This runs it locally to
      # prevent remote, incoming connections.
      listen_address: ...
      seeds:
        bb14af025d226288a3488b4433cf5cb54d6a710365a2d95ac6ffbd9b9198a86a:
            addresses:
            - "/dns4/pfn0.node.devnet.aptoslabs.com/tcp/6182/ln-noise-ik/bb14af025d226288a3488b4433cf5cb54d6a710365a2d95ac6ffbd9b9198a86a/ln-handshake/0"
            role: "Upstream"
        7fe8523388084607cdf78ff40e3e717652173b436ae1809df4a5fcfc67f8fc61:
            addresses:
            - "/dns4/pfn1.node.devnet.aptoslabs.com/tcp/6182/ln-noise-ik/7fe8523388084607cdf78ff40e3e717652173b436ae1809df4a5fcfc67f8fc61/ln-handshake/0"
            role: "Upstream"
        f6b135a59591677afc98168791551a0a476222516fdc55869d2b649c614d965b:
            addresses:
            - "/dns4/pfn2.node.devnet.aptoslabs.com/tcp/6182/ln-noise-ik/f6b135a59591677afc98168791551a0a476222516fdc55869d2b649c614d965b/ln-handshake/0"
            role: "Upstream"
...            
```

[pfn_config_file]: https://github.com/aptos-labs/aptos-core/tree/main/docker/compose/public_full_node/public_full_node.yaml
[pfn_docker_compose]: https://github.com/aptos-labs/aptos-core/tree/main/docker/compose/public_full_node/docker-compose.yaml
[rest_spec]: https://github.com/aptos-labs/aptos-core/tree/main/api
[devnet_genesis]: https://devnet.aptoslabs.com/genesis.blob
[devnet_waypoint]: https://devnet.aptoslabs.com/waypoint.txt
[aptos-labs/aptos-core]: https://github.com/aptos-labs/aptos-core.git
[status dashboard]: https://status.devnet.aptos.dev
