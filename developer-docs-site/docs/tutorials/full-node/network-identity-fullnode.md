---
title: "Network Identity For FullNode"
slug: "network-identity-fullnode"
sidebar_position: 12
---

# Network Identity For FullNode

This guide will show you how to:
- Create a static network identity for your FullNode.
- Retrieve the public network identity.
- Start a node with (or without) a static network identity.

## Before you proceed

Before you proceed, make sure that you already know how to start your local FullNode. See [Run a FullNode](run-a-fullnode) for detailed documentation.

:::caution Docker support only on Linux

Docker container is currently supported only on Linux x86-64 platform. If you are on macOS or Windows platform, use the Aptos-core source approach.

:::

## Creating a static identity for a FullNode

FullNodes will automatically start up with a randomly generated network identity (a `PeerId` and a public key pair). This works well for regular FullNodes, but you may wish to be added to another node's allowlist, provide specific permissions or run your FullNode with the same identity. In this case, creating a static network identity can help.

1. Fork and clone the [aptos-labs/aptos-core](https://github.com/aptos-labs/aptos-core) repo. For example:

    ```
    $ git clone https://github.com/<YOUR-GITHUB-USERID>/aptos-core.git
    $ cd aptos-core
    $ ./scripts/dev_setup.sh
    $ source ~/.cargo/env
    ```

    **Using Docker**

    Alternatively, if you are on Linux x86-64 platform, you can use Aptos Docker image. `cd` into the directory for your local public FullNode and start a Docker container with the latest tools, for example:

    ```
    $ cd ~/my-full-node
    $ docker run -it aptoslab/tools:devnet /bin/bash
    ```

2. Run the [Aptos CLI](https://github.com/aptos-labs/aptos-core/blob/main/crates/aptos/README.md) `aptos` to produce a hex encoded static x25519 private key. This will be the private key for your network identity.

  :::note

  The below command will also create a corresponding `private-key.txt.pub` file with the public identity key in it.

  :::

    ```
    aptos key generate --key-type x25519 --output-file /path/to/private-key.txt

    ```

    **Using Docker**

    From inside the `aptoslab/tools` Docker container. Open a new terminal and `cd` into the directory where you started the Docker container for your FullNode. Making sure to provide the full path to where you want the private key TXT file to be stored, run the command as below:

    ```
    aptos-operational-tool generate-key \
        --encoding hex \
        --key-type x25519 \
        --key-file /path/to/private-key.txt
    ```

Example `private-key.txt` and the associated `private-key.txt.pub` files are shown below:

  ```
  $ cat ~/private-key.txt
  C83110913CBE4583F820FABEB7514293624E46862FAE1FD339B923F0CACC647D%           

  $ cat ~/private-key.txt.pub
  B881EA2C174D8211C123E5A91D86227DB116A44BB345A6E66874F83D8993F813%
  ```
Note, that the private-key.txt.pub is not the public key which is referred to later in this tutorial and is not used for running a full node. We mention this key for the sake of completeness as it is used, alongside the private key, to generate the signing key utilized for signing. 

## Retrieve the public network identity

As shown above, when you use Aptos-core source to generate a private key, the associated public identity key will also be generated.

  **Using Docker**

  From inside the `aptoslab/tools` Docker container:

  ```
  $ aptos-operational-tool extract-peer-from-file \
      --encoding hex \
      --key-file /path/to/private-key.txt \
      --output-file /path/to/peer-info.yaml
  ```

This will create a YAML file that will have your public identity in it. This is useful if you want to connect your FullNode to a specific upstream FullNode, and that FullNode only allows known identities to connect to them.

Example output `peer-info.yaml`:

 ```
 ---
 2a873fd3fb4e334b729966dc5aa68118fb5ba7c2c0c39d9860e709fd6589214b:
   addresses: []
   keys:
     - "0x2a873fd3fb4e334b729966dc5aa68118fb5ba7c2c0c39d9860e709fd6589214b"
 role: Upstream
  ```

  In this example, `2a873fd3fb4e334b729966dc5aa68118fb5ba7c2c0c39d9860e709fd6589214b` is the peer ID as well as the public key, which is derived from the private key you generated from the previous step.



## Start a node with a static network identity

Once you have the static identity you can startup the FullNode by modifying the configuration file `public_full_node.yaml`:

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
    key: "C83110913CBE4583F820FABEB7514293624E46862FAE1FD339B923F0CACC647D"
    peer_id: "2a873fd3fb4e334b729966dc5aa68118fb5ba7c2c0c39d9860e709fd6589214b"
```

## Allowing other FullNodes to connect

Once you start your FullNode with a static identity you can allow others to connect to devnet through your node. Follow these recommendations:

- Make sure you open port `6180` (or `6182`, depending on which port your node is listening to) and that you open your firewall.
- If you are using Docker, simply add `- "6180:6180"` or `- "6182:6182"` under ports in your ``docker-compose.yaml`` file.
- You'll need to share your FullNode info for others to use as `seeds` in their configurations (e.g., `peer-info.yaml`):

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

- Make sure the port number you put in the `addressses` matches the one you have in the FullNode config (`6180` or `6182`). For example:

  ```
  2a873fd3fb4e334b729966dc5aa68118fb5ba7c2c0c39d9860e709fd6589214b:
    addresses:
    - "/dns4/pfn0.node.devnet.aptoslabs.com/tcp/6182/ln-noise-ik/2a873fd3fb4e334b729966dc5aa68118fb5ba7c2c0c39d9860e709fd6589214b/ln-handshake/0"
    role: "Upstream"
  2a873fd3fb4e334b729966dc5aa68118fb5ba7c2c0c39d9860e709fd6589214b:
    addresses:
    - "/ip4/100.20.221.187/tcp/6182/ln-noise-ik/2a873fd3fb4e334b729966dc5aa68118fb5ba7c2c0c39d9860e709fd6589214b/ln-handshake/0"
    role: "Upstream"
  ```

:::note

Peer ID is synonymous with `AccountAddress`. See [NetworkAddress](https://github.com/aptos-labs/aptos-core/blob/main/documentation/specifications/network/network-address.md) to see how `addresses` key value is constructed.

:::
