---
title: "Run a FullNode"
slug: "run-a-fullnode"
sidebar_position: 10
---

# Run a FullNode

You can run [FullNodes](/basics/basics-fullnodes) to verify the state and synchronize to the Aptos Blockchain. FullNodes can be run by anyone. FullNodes replicate the full state of the blockchain by querying each other or the validators directly.

This tutorial details how to configure a public FullNode to connect to the Aptos devnet. The FullNodes provided by Aptos Labs have rate limits that can impede development on Testnet. This will provide you with the data directly to avoid such rate limiting.

> **Note:** Your public FullNode will be connected to devnet with a REST endpoint accessible on your computer at localhost:8080.
>

#### Prerequisites
Before you get started with this tutorial, we recommend you familiarize yourself with the following:
* [Validator node concepts](/basics/basics-validator-nodes) 
* [FullNode concepts](/basics/basics-fullnodes) 
* [REST specifications][rest_spec]


## Getting started
You can configure a public FullNode in two ways: using the Aptos-core source code or Docker.

### Hardware requirement
For running a production grade Fullnode we recommend using hardware with:
* CPU: Intel Xeon Skylake or newer, 4 cores
* Memory: 8GiB RAM

If running the Fullnode for development or testing purpose:
* CPU: 2 cores
* Memory: 4GiB RAM

### Using Aptos-core source code
1. Download and clone the Aptos-core repository from GitHub and prepare your developer environment by running the following commands:
     ```
     git clone https://github.com/aptos-labs/aptos-core.git
     cd aptos
     ./scripts/dev_setup.sh
     source ~/.cargo/env
     ```
2. Checkout the branch for devnet using `git checkout origin/devnet`.
3. To prepare your configuration file:
     * Copy `config/src/config/test_data/public_full_node.yaml` to your current working directory.
     * Download [genesis][devnet_genesis] and [waypoint][devnet_waypoint] files for devnet.
     * Update the public_full_node.yaml file in your current working directory by:
       * Specifying the directory where you want devnet to store its database next to `base:data_dir`; for example, `./data`.
       * Copying and pasting the contents of the waypoint file to the `waypoint` field.
       * Reading through the config and making any other desired changes. You can see what configurations the `public_full_node.yaml` file should have by checking the following file as an example: `docker/compose/public_full_node/public_full_node.yaml`
4. Run the aptos-node using `cargo run -p aptos-node --release -- -f ./public_full_node.yaml`

You have now successfully configured and started running a FullNode connected to Aptos devnet.

Note: This will build a release binary under `target/release/aptos-node`. The release binaries tend to be substantially faster than debug binaries but lack debugging information useful for development. Simply omit the `--release` flag to build a debug binary.

## Using Docker

You can also use Docker to configure and run your FullNode.

1. Install Docker and Docker-Compose.
2. Create a directory for your public FullNode composition.
3. Download the public FullNode [docker compose][pfn_docker_compose] and [aptos-core][pfn_config_file] configuration files into this directory.
4. Download [genesis][devnet_genesis] and [waypoint][devnet_waypoint] files for devnet into that directory.
5. Run docker-compose: `docker-compose up`.

### Understand and verify the correctness of your FullNode

#### Initial synchronization
During the initial synchronization of your FullNode, there may be a lot of data to transfer.

* Progress can be monitored by querying the metrics port `curl 127.0.0.1:9101/metrics 2> /dev/null | grep aptos_state_sync_version | grep type`, which will print out several counters:
  * `aptos_state_sync_version{type="committed"}` -- the latest (blockchain) version that is backed by a signed commitment (ledger info) from the validators
  * `aptos_state_sync_version{type="highest"}` -- the highest or latest known version, typically the same as target
  * `aptos_state_sync_version{type="synced"}` -- the latest blockchain version available in storage, it might not be backed by a ledger info
  * `aptos_state_sync_version{type="target"}` -- the state sync's current target ledger info version
* The Executor component will update the output log by showing that 1000 blocks are committed at a time:

  ```
  fullnode_1  | INFO 2020-09-28T23:16:04.425083Z execution/executor/src/lib.rs:534 sync_request_received {"local_synced_version":633750,"name":"chunk_executor","first_version_in_request":633751,"num_txns_in_request":250}
  fullnode_1  | INFO 2020-09-28T23:16:04.508902Z execution/executor/src/lib.rs:580 sync_finished {"committed_with_ledger_info":false,"name":"chunk_executor","synced_to_version":634000}
  ```

* At the same time, the StateSync component will output similar information but show the destination.

* The blockchain (devnet) ledger’s volume can be monitored by entering the container:

  ```
  # Obtain the container id:
  id=$(docker container ls | grep public_full_node_fullnode_1 | grep -oE "^[0-9a-zA-Z]+")
  # Enter the container:
  docker exec -it $id /bin/bash
  # Observe the volume (ledger) size:
  du -cs -BM /opt/aptos/data
  ```

## Advanced Guide

If you want to dive into more customization of your node confg. This advanced guide will show you how to:
* Create a static network identity for your new Fullnode
* Retrieve the public network identity for other nodes allowlist
* Start a node with or without a static network identity

### Create a static identity for a fullnode

Fullnodes will automatically start up with a randomly generated network identity (a PeerId and a Public Key pair).  This works great for regular fullnodes, but if you need another node to allowlist you or provide specific permissions, or if you want to run your fullnode always with a same identity, creating a static network identity can help.

1. Build the `aptos-operational-tool` using the [aptos-labs/aptos-core][] repo, we can build using cargo to run these tools. e.g.
    ```
    $ git clone https://github.com/aptos-labs/aptos-core.git
    $ cd aptos-core
    $ ./scripts/dev_setup.sh
    $ source ~/.cargo/env
    $ cargo run -p aptos-operational-tool -- <command> <args>
    ```

    Alternatively, you can use our docker image. Start a docker container with the latest tools version e.g.

    ```
    $ docker run -i aptoslab/tools:devnet sh -x
    $ aptos-operational-tool <command> <arg>
    ```

2. Run the key generator, to output a hex encoded static x25519 PrivateKey.  This will be your private key for your network identity.
   ```
    $ cargo run -p aptos-operational-tool -- generate-key --encoding hex --key-type x25519 --key-file /path/to/private-key.txt
   ```

   Or inside aptoslab/tools docker container:

    ```
    $ aptos-operational-tool generate-key --encoding hex --key-type x25519 --key-file /path/to/private-key.txt
    ```

    Example key file:

    ```
    $ cat /path/to/private-key.txt
    B8BD811A91D8E6E0C6DAC991009F189337378760B55F3AD05580235325615C74
    ```

### Retrieve public network identity

1. Run the peer generator on the previous key file
   ```
    $ cargo run -p aptos-operational-tool -- extract-peer-from-file --encoding hex --key-file /path/to/private-key.txt --output-file /path/to/peer-info.yaml
   ```

   Or inside aptoslab/tools docker container:

    ```
    $ aptos-operational-tool extract-peer-from-file --encoding hex --key-file /path/to/private-key.txt --output-file /path/to/peer-info.yaml
    ```

   Example output yaml:

   ```
    ---
    14fd60f81a2f8eedb0244ec07a26e575:
      addresses: []
      keys:
        - ca3579457555c80fc7bb39964eb298c414fd60f81a2f8eedb0244ec07a26e575
      role: Downstream
    ```

    In this example, `14fd60f81a2f8eedb0244ec07a26e575` is the peer id, and `ca3579457555c80fc7bb39964eb298c414fd60f81a2f8eedb0244ec07a26e575` is the public key derived from the private key you generated from previous step.

2. This will create a yaml file, that will have your public identity in it for providing to an upstream full node. This is useful if you want to connect your fullnode through a specific upstream full node, and that full node only allows known identity to connect to them. 

### Start a node with the static network identity

Once we have the static identity, we can startup a node with it.
```
full_node_networks:
- network_id: "public"
  discovery_method: "onchain"
  identity:
    type: "from_config"
    key: "<PRIVATE_KEY>"
    peer_id: "<PEER_ID>"
```

Example:

```
full_node_networks:
- network_id: "public"
  discovery_method: "onchain"
  identity:
    type: "from_config"
    key: "B8BD811A91D8E6E0C6DAC991009F189337378760B55F3AD05580235325615C74"
    peer_id: "14fd60f81a2f8eedb0244ec07a26e575"
```

[pfn_config_file]: https://github.com/aptos-labs/aptos-core/tree/main/docker/compose/public_full_node/public_full_node.yaml
[pfn_docker_compose]: https://github.com/aptos-labs/aptos-core/tree/main/docker/compose/public_full_node/docker-compose.yaml
[rest_spec]: https://github.com/aptos-labs/aptos-core/tree/main/api
[devnet_genesis]: https://devnet.aptoslabs.com/genesis.blob
[devnet_waypoint]: https://devnet.aptoslabs.com/waypoint.txt
[aptos-labs/aptos-core]: https://github.com/aptos-labs/aptos-core.git
