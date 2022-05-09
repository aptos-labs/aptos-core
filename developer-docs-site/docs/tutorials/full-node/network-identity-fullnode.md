---
title: "Network Identity For FullNode"
slug: "network-identity-fullnode"
sidebar_position: 12
---

# Network Identity For FullNode

FullNodes will automatically start up with a randomly generated network identity. This works well for regular FullNodes. However:

- You may want your FullNode to be added to a specific upstream FullNode's allowlist (i.e., another FullNode participant in the Aptos network), because:

  - You might require specific permissions for your FullNode on this specific upstream FullNode, or
  - This upstream FullNode only allows known identities to connect to it, or
  - You may wish to advertise your FullNode for other Aptos FullNodes to connect to (to help support the Aptos network).

In such cases, it helps if you run your FullNode with a static network identity, instead of a randomly generated network identity that keeps changing every time you start up your FullNode.

This guide will show you how to:

- Create a static network identity for your FullNode.
- Start a node with a static network identity.
- Allow other FullNodes to connect to your FullNode.

## Before you proceed

Before you proceed, make sure that you already know how to start your local FullNode. See [Run a FullNode](run-a-fullnode) for detailed documentation.

:::caution Docker support only on Linux

Docker container is currently supported only on Linux x86-64 platform. If you are on macOS or Windows platform, use the Aptos-core source approach.

:::

## Creating a static identity for a FullNode

To create a static identity for your FullNode:

1. You first create a private key, public key pair for your FullNode.
2. Next you derive the `peer_id` from the public key.
3. Finally, you use the `peer_id` in your `public_full_node.yaml` to create a static network identity for your FullNode.

Follow the below detailed steps:

1. Fork and clone the [aptos-labs/aptos-core](https://github.com/aptos-labs/aptos-core) repo. For example:

    ```
    $ git clone https://github.com/<YOUR-GITHUB-USERID>/aptos-core.git
    $ cd aptos-core
    $ ./scripts/dev_setup.sh
    $ source ~/.cargo/env
    ```

    **Using Docker**

    Alternatively, if you are on Linux x86-64 platform, you can use the Aptos Docker image.

    `cd` into the directory for your local public FullNode and start a Docker container with the latest tools, for example:

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

  Example `private-key.txt` and the associated `private-key.txt.pub` files are shown below:

  ```
  $ cat ~/private-key.txt
  C83110913CBE4583F820FABEB7514293624E46862FAE1FD339B923F0CACC647D%           

  $ cat ~/private-key.txt.pub
  B881EA2C174D8211C123E5A91D86227DB116A44BB345A6E66874F83D8993F813%
  ```

  **Using Docker**

  Run this step from inside the `aptoslab/tools` Docker container. Open a new terminal and `cd` into the directory where you started the Docker container for your FullNode. Making sure to provide the full path to where you want the private key TXT file to be stored, run the command as below:

  ```
  aptos-operational-tool generate-key \
      --encoding hex \
      --key-type x25519 \
      --key-file /path/to/private-key.txt
  ```

3. Retrieve the peer identity

  When you use Aptos-core source to generate a private key, use the below Aptos CLI command to generate the `peer_id`:

  ```
  aptos key extract-peer  --private-key-file private-key.txt  \
      --output-file peer-info.yaml
  ```

    **Using Docker**

    From inside the `aptoslab/tools` Docker container:

    ```
    $ aptos-operational-tool extract-peer-from-file \
        --encoding hex \
        --key-file /path/to/private-key.txt \
        --output-file /path/to/peer-info.yaml
    ```

  This will create a YAML file that will have your `peer_id` corresponding to the `private-key.txt` you provided.

  Example output `peer-info.yaml`:

   ```
   ---
   B881EA2C174D8211C123E5A91D86227DB116A44BB345A6E66874F83D8993F813:
     addresses: []
     keys:
       - "0xB881EA2C174D8211C123E5A91D86227DB116A44BB345A6E66874F83D8993F813"
   role: Upstream
    ```

  In this example, `B881EA2C174D8211C123E5A91D86227DB116A44BB345A6E66874F83D8993F813` is the `peer_id`. Use this in the `peer_id` field of your `public_full_node.yaml` to create a static identity for your FullNode.


## Start a node with a static network identity

After you generated the public identity key you can startup the FullNode with a static network identity by using the public key in the `peer_id` field of the configuration file `public_full_node.yaml`:

```
full_node_networks:
- network_id: "public"
  discovery_method: "onchain"
  identity:
    type: "from_config"
    key: "<PRIVATE_KEY>"
    peer_id: "<PEER_ID>"
```

In our example, you would specify the above-generated `peer_id` in place of the `<PEER_ID>`:

```
full_node_networks:
- network_id: "public"
  discovery_method: "onchain"
  identity:
    type: "from_config"
    key: "C83110913CBE4583F820FABEB7514293624E46862FAE1FD339B923F0CACC647D"
    peer_id: "B881EA2C174D8211C123E5A91D86227DB116A44BB345A6E66874F83D8993F813"
```

## Allowing other FullNodes to connect

Once you start your FullNode with a static identity you can allow others to connect to devnet through your node.

:::note

In the below steps, the port numbers used are for illustration only. You can use your choice of port numbers.

:::

- Make sure you open port `6180` (or `6182`, for example, depending on which port your node is listening to) and that you open your firewall.
- If you are using Docker, simply add `- "6180:6180"` or `- "6182:6182"` under ports in your ``docker-compose.yaml`` file.
- Share your FullNode static network identity with others. They can then use it in the `seeds` key of their `public_full_node.yaml` file to connect to your FullNode.
- Make sure the port number you put in the `addresses` matches the one you have in the FullNode configuration file `public_full_node.yaml` (for example, `6180` or `6182`).

Share your FullNode static network identity in the following format in the Discord channel `advertise-full-nodes`:

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

 For example:

  ```
  B881EA2C174D8211C123E5A91D86227DB116A44BB345A6E66874F83D8993F813:
    addresses:
    - "/dns4/pfn0.node.devnet.aptoslabs.com/tcp/6182/ln-noise-ik/B881EA2C174D8211C123E5A91D86227DB116A44BB345A6E66874F83D8993F813/ln-handshake/0"
    role: "Upstream"
  B881EA2C174D8211C123E5A91D86227DB116A44BB345A6E66874F83D8993F813:
    addresses:
    - "/ip4/100.20.221.187/tcp/6182/ln-noise-ik/B881EA2C174D8211C123E5A91D86227DB116A44BB345A6E66874F83D8993F813/ln-handshake/0"
    role: "Upstream"
  ```

:::note

Peer ID is synonymous with `AccountAddress`. See [NetworkAddress](https://github.com/aptos-labs/aptos-core/blob/main/documentation/specifications/network/network-address.md) to see how `addresses` key value is constructed.

:::
