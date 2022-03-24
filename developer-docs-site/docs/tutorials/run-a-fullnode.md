---
title: "Run a FullNode"
slug: "run-a-fullnode"
sidebar_position: 10
---

# Run a FullNode

You can run [FullNodes](/basics/basics-fullnodes) to synchronize the state of the Aptos Blockchain and stay up-to-date. FullNodes can be run by anyone. FullNodes replicate the entire state of the blockchain by querying other Aptos FullNodes or validators.

This tutorial explains how to configure a public FullNode to connect to the Aptos devnet. The FullNodes provided by Aptos Labs have rate limits that can impede development. This will provide you with the data directly to avoid such rate limiting.

> **Note:** Your public FullNode will be connected to devnet with a REST endpoint accessible on your computer at localhost:8080.
>

## Prerequisites
Before you get started with this tutorial, we recommend you familiarize yourself with the following:
* [Validator node concepts](/basics/basics-validator-nodes) 
* [FullNode concepts](/basics/basics-fullnodes) 
* [REST specifications][rest_spec]

## Hardware requirements
For running a production grade Fullnode we recommend the following:
* CPU: 4 cores (Intel Xeon Skylake or newer)
* Memory: 8GiB RAM

If running the Fullnode for development or testing purpose:
* CPU: 2 cores
* Memory: 4GiB RAM

## Getting started

You can configure a public FullNode in two ways: using the Aptos-core source code or using Docker.

### Using Aptos-core source code
1. Download the Aptos-core repository from GitHub and prepare your developer environment by running the following commands:
     ```
     git clone https://github.com/aptos-labs/aptos-core.git
     cd aptos-core
     ./scripts/dev_setup.sh
     source ~/.cargo/env
     ```
2. Checkout the branch for devnet using `git checkout origin/devnet`.
3. To prepare your configuration file:
     * Copy `config/src/config/test_data/public_full_node.yaml` to your current working directory.
     * Download the [genesis][devnet_genesis] and [waypoint][devnet_waypoint] files for devnet.
     * Update the `public_full_node.yaml` file in your current working directory by:
       * Specifying the directory where you want to store the devnet database. Specify this next to `base:data_dir` (for example, `./data`).
       * Copying and pasting the contents of the waypoint file to the `waypoint` field.
       * Reading through the config and making any other desired changes. You can see what configurations the `public_full_node.yaml` file should have by checking the following file as an example: `docker/compose/public_full_node/public_full_node.yaml`
4. Start the Aptos FullNode using `cargo run -p aptos-node --release -- -f ./public_full_node.yaml`

You have now successfully configured and started running a FullNode connected to Aptos devnet.

Note: This will build a release binary under `target/release/aptos-node`. The release binaries tend to be substantially faster than debug binaries but lack debugging information useful for development. Simply omit the `--release` flag to build a debug binary.

### Using Docker

You can also use Docker to configure and run your FullNode.

1. Install Docker and Docker-Compose.
2. Create a directory for your public FullNode composition.
3. Download the public FullNode [docker compose][pfn_docker_compose] and [aptos-core][pfn_config_file] configuration files into this directory.
4. Download the devenet [genesis][devnet_genesis] and [waypoint][devnet_waypoint] files into that directory.
5. Run docker-compose: `docker-compose up`.

## Understand and verify the correctness of your FullNode

### Initial synchronization
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

* At the same time, the state sync component will output similar information.

* The number of connections should be more than 0, `curl 127.0.0.1:9101/metrics 2> /dev/null | grep aptos_connections`. If it's not, see the next section for potential solutions.

* The blockchain (devnet) ledger’s volume can be monitored by entering the Docker container:

  ```
  # Obtain the container id:
  id=$(docker container ls | grep public_full_node_fullnode_1 | grep -oE "^[0-9a-zA-Z]+")
  # Enter the container:
  docker exec -it $id /bin/bash
  # Observe the volume (ledger) size:
  du -cs -BM /opt/aptos/data
  ```

### Add upstream seed peers
Devnet validator fullnodes will only accept a maximum of 1000 connections. If our network is experiencing high volume, your fullnode might not able to connect. You might see "NoAvailablePeers" in your node's error messages. If this happens, you can set `seeds` in the FullNode configuration file to add new  peers to connect to. We prepared some FullNodes addresses for you to use, here. Also. feel free to use the ones provided by the community (anyone already running a fullnode can provide their address for you to connect). Add these to your configuration file:
```
seeds:
  4d6a710365a2d95ac6ffbd9b9198a86a:
      addresses:
      - "/dns4/pfn0.node.devnet.aptoslabs.com/tcp/6182/ln-noise-ik/bb14af025d226288a3488b4433cf5cb54d6a710365a2d95ac6ffbd9b9198a86a/ln-handshake/0"
      role: "Upstream"
  52173b436ae1809df4a5fcfc67f8fc61:
      addresses:
      - "/dns4/pfn1.node.devnet.aptoslabs.com/tcp/6182/ln-noise-ik/7fe8523388084607cdf78ff40e3e717652173b436ae1809df4a5fcfc67f8fc61/ln-handshake/0"
      role: "Upstream"
  476222516fdc55869d2b649c614d965b:
      addresses:
      - "/dns4/pfn2.node.devnet.aptoslabs.com/tcp/6182/ln-noise-ik/f6b135a59591677afc98168791551a0a476222516fdc55869d2b649c614d965b/ln-handshake/0"
      role: "Upstream"
```

## Advanced Guide

If you want to explore additional customizations for your FullNode configurations, this guide will show you how to:
* Create a static network identity for your FullNode
* Retrieve the public network identity
* Start a node with (or without) a static network identity

### Create a static identity for a FullNode

FullNodes will automatically start up with a randomly generated network identity (a `PeerId` and a public key pair). This works well for regular FullNodes, but you may wish to be added to another node's allowlist, provide specific permissions or run your FullNode with the same identity. In this case, creating a static network identity can help.

1. Build the `aptos-operational-tool` using the [aptos-labs/aptos-core][] repo. We can use cargo to build and run these tools, e.g.,
    ```
    $ git clone https://github.com/aptos-labs/aptos-core.git
    $ cd aptos-core
    $ ./scripts/dev_setup.sh
    $ source ~/.cargo/env
    $ cargo run -p aptos-operational-tool -- <command> <args>
    ```

    Alternatively, you can use our docker image. Start a docker container with the latest tools, e.g.,

    ```
    $ docker run -i aptoslab/tools:devnet sh -x
    $ aptos-operational-tool <command> <arg>
    ```

2. Run the key generator, to produce a hex encoded static x25519 private key. This will be the private key for your network identity.
   ```
    $ cargo run -p aptos-operational-tool -- generate-key --encoding hex --key-type x25519 --key-file /path/to/private-key.txt
   ```

   Or inside the `aptoslab/tools` docker container:

    ```
    $ aptos-operational-tool generate-key --encoding hex --key-type x25519 --key-file /path/to/private-key.txt
    ```

    Example key file:

    ```
    $ cat /path/to/private-key.txt
    B8BD811A91D8E6E0C6DAC991009F189337378760B55F3AD05580235325615C74
    ```

### Retrieve the public network identity

1. Run the peer generator on the previous key file
   ```
    $ cargo run -p aptos-operational-tool -- extract-peer-from-file --encoding hex --key-file /path/to/private-key.txt --output-file /path/to/peer-info.yaml
   ```

   Or inside the `aptoslab/tools` docker container:

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

    In this example, `14fd60f81a2f8eedb0244ec07a26e575` is the peer id, and `ca3579457555c80fc7bb39964eb298c414fd60f81a2f8eedb0244ec07a26e575` is the public key derived from the private key you generated from the previous step.

2. This will create a yaml file that will have your public identity in it. This is useful if you want to connect your FullNode to a specific upstream FullNode, and that FullNode only allows known identities to connect to them. 

### Start a node with a static network identity

Once we have the static identity we can startup the node by modifying the configuration file (public_full_node.yaml):
```
full_node_networks:
- network_id: "public"
  discovery_method: "onchain"
  identity:
    type: "from_config"
    key: "<PRIVATE_KEY>"
    peer_id: "<PEER_ID>"
```

In our example, we'd specify:

```
full_node_networks:
- network_id: "public"
  discovery_method: "onchain"
  identity:
    type: "from_config"
    key: "B8BD811A91D8E6E0C6DAC991009F189337378760B55F3AD05580235325615C74"
    peer_id: "14fd60f81a2f8eedb0244ec07a26e575"
```

### Allowing other FullNodes to connect

Once you start your FullNode with a static identity you can allow others to connect to devnet through your node. Make sure you open port `6180` (or `6182`, depending on which port your node is listening to) and that you open your firewall. You'll need to share your FullNode info for others to use as `seeds` in their configurations (peer-info.yaml):

```
<Peer_ID>:
  addresses:
  # with DNS
  - "/dns4/<DNS_Name>/tcp/<Port_Number>/ln-noise-ik/<Public_Key>/ln-handshake/0"
  role: Upstream
<Peer_ID>:
  addresses:
  # with IP
  - "/ip4/<IP_Address>/tcp/<Port_Number>/ln-noise-ik/<Public_Key>/ln-handshake/0"
  role: Upstream
```

Make sure the port number you put in the address matches the one you have in the fullnode config (`6180` or `6182`). For example:

```
4d6a710365a2d95ac6ffbd9b9198a86a:
  addresses:
  - "/dns4/pfn0.node.devnet.aptoslabs.com/tcp/6182/ln-noise-ik/bb14af025d226288a3488b4433cf5cb54d6a710365a2d95ac6ffbd9b9198a86a/ln-handshake/0"
  role: "Upstream"
4d6a710365a2d95ac6ffbd9b9198a86a:
  addresses:
  - "/ip4/100.20.221.187/tcp/6182/ln-noise-ik/bb14af025d226288a3488b4433cf5cb54d6a710365a2d95ac6ffbd9b9198a86a/ln-handshake/0"
  role: "Upstream"
```

[pfn_config_file]: https://github.com/aptos-labs/aptos-core/tree/main/docker/compose/public_full_node/public_full_node.yaml
[pfn_docker_compose]: https://github.com/aptos-labs/aptos-core/tree/main/docker/compose/public_full_node/docker-compose.yaml
[rest_spec]: https://github.com/aptos-labs/aptos-core/tree/main/api
[devnet_genesis]: https://devnet.aptoslabs.com/genesis.blob
[devnet_waypoint]: https://devnet.aptoslabs.com/waypoint.txt
[aptos-labs/aptos-core]: https://github.com/aptos-labs/aptos-core.git
