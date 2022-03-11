---
title: "Configure and run a public FullNode"
slug: "configure-run-public-fullnode"
sidebar_position: 10
---
You can run [FullNodes](/basics/basics-fullnodes) to verify the state and synchronize to the Aptos Blockchain. FullNodes can be run by anyone. FullNodes replicate the full state of the blockchain by querying each other, or by querying the validators directly.

This tutorial details how to configure a public FullNode to connect to *testnet*, the Aptos Payment Network’s public test network..

> **Note:** Your public FullNode will be connected to testnet with a JSON-RPC endpoint accessible on your computer at localhost:8080.
>

#### Prerequisites
Before you get started with this tutorial, we recommend you familiarize yourself with the following:
* [Validator node concepts](/basics/basics-validator-nodes) 
* [FullNode concepts](/basics/basics-fullnodes) 
* [REST specifications][rest_spec]


## Getting started
You can configure a public FullNode in two ways: using the Aptos-core source code or Docker.

### Using Aptos-core source code
1. Download and clone the Aptos-core repository from GitHub and prepare your developer environment by running the following commands:
     ```
     git clone https://github.com/aptos-labs/aptos-core.git
     cd aptos
     ./scripts/dev_setup.sh
     source ~/.cargo/env
     ```

2. Checkout the branch for testnet using `git checkout origin/testnet`.

3. To prepare your configuration file:

     * Copy `config/src/config/test_data/public_full_node.yaml` to your current working directory.

     * Download [genesis][testnet_genesis] and [waypoint][testnet_waypoint] files for testnet.

     * Update the public_full_node.yaml file in your current working directory by:

       * Specifying the directory where you want testnet to store its database next to `base:data_dir`; for example, `./data`.

       * Copying and pasting the contents of the waypoint file to the `waypoint` field.

       * Reading through the config and making any other desired changes. You can see what configurations the `public_full_node.yaml` file should have by checking the following file as an example: `docker/compose/public_full_node/public_full_node.yaml`
4. Run the aptos-node using `cargo run -p aptos-node --release -- -f ./public_full_node.yaml`

You have now successfully configured and started running a public FullNode in testnet..

Note: This will build a release binary under `target/release/aptos-node`. The release binaries tend to be substantially faster than debug binaries but lack debugging information useful for development. Simply omit the `--release` flag to build a debug binary.

## Using Docker

You can also use Docker to configure and run your PublicFullNode.

1. Install Docker and Docker-Compose.
2. Create a directory for your public FullNode composition.
3. Download the public FullNode [docker compose][pfn_docker_compose] and [aptos-core][pfn_config_file] configuration files into this directory.
4. Download [genesis][testnet_genesis] and [waypoint][testnet_waypoint] files for testnet into that directory.
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

* The blockchain (testnet) ledger’s volume can be monitored by entering the container:

  ```
  # Obtain the container id:
  id=$(docker container ls | grep public_full_node_fullnode_1 | grep -oE "^[0-9a-zA-Z]+")
  # Enter the container:
  docker exec -it $id /bin/bash
  # Observe the volume (ledger) size:
  du -cs -BM /opt/aptos/data
  ```

[pfn_config_file]: https://github.com/aptos-labs/aptos-core/tree/main/docker/compose/public_full_node/public_full_node.yaml
[pfn_docker_compose]: https://github.com/aptos-labs/aptos-core/tree/main/docker/compose/public_full_node/docker-compose.yaml
[rest_spec]: https://github.com/aptos-labs/aptos-core/tree/main/api
[testnet_genesis]: https://dev.fullnode.aptoslabs.com/genesis.blob
[testnet_waypoint]: https://dev.fullnode.aptoslabs.com/waypoint.txt
