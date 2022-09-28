---
title: "Network Identity For Fullnode"
slug: "network-identity-fullnode"
---

# Network Identity For Fullnode

Fullnodes will automatically start up with a randomly generated network identity. This works well for regular fullnodes. However:

- You may want your fullnode to be added to a specific upstream fullnode's allowlist (i.e., another fullnode participant in the Aptos network), because:

  - You might require specific permissions for your fullnode on this specific upstream fullnode, or
  - This upstream fullnode only allows known identities to connect to it, or
  - You may wish to advertise your fullnode for other Aptos fullnodes to connect to (to help support the Aptos network).

In such cases, it helps if you run your fullnode with a static network identity, instead of a randomly generated network identity that keeps changing every time you start up your fullnode.

This guide will show you how to:

- Create a static network identity for your fullnode.
- Start a node with a static network identity.
- Allow other fullnodes to connect to your fullnode.

## Before you proceed

Before you proceed, make sure that you already know how to start your local fullnode. See [Run a Fullnode](/nodes/full-node/index.md) for detailed documentation.

:::caution Docker support only on Linux

Docker container is currently supported only on Linux x86-64 platform. If you are on macOS or Windows platform, use the Aptos-core source approach.

:::

## Creating a static identity for a fullnode

To create a static identity for your fullnode:

1. You first create a private key, public key pair for your fullnode.
2. Next you derive the `peer_id` from the public key.
3. Finally, you use the `peer_id` in your `fullnode.yaml` to create a static network identity for your fullnode.

Follow the below detailed steps:

1. Preparation
    
    **Using Aptos-core source code**
    
    Clone the [aptos-labs/aptos-core](https://github.com/aptos-labs/aptos-core) repo. For example:

    ```bash
    git clone https://github.com/aptos-labs/aptos-core.git
    cd aptos-core
    ./scripts/dev_setup.sh
    source ~/.cargo/env
    ```

    **Using Docker**

    Alternatively, if you are on Linux x86-64 platform, you can use the Aptos Docker image.

    `cd` into the directory for your local public fullnode and start a Docker container with the latest tools, for example:

    ```bash
    cd ~/my-full-node
    docker run -it aptoslabs/tools:devnet /bin/bash
    ```

2. Generate the private key

  **Using Aptos-core source code**
  
  Run the [Aptos CLI](https://github.com/aptos-labs/aptos-core/blob/main/crates/aptos/README.md) `aptos` to produce a hex encoded static x25519 private key. This will be the private key for your network identity.

  :::tip

  The below command will also create a corresponding `private-key.txt.pub` file with the public identity key in it.

  :::

  ```bash
  aptos key generate --key-type x25519 --output-file /path/to/private-key.txt

  ```

  Example `private-key.txt` and the associated `private-key.txt.pub` files are shown below:

  ```bash
  cat ~/private-key.txt
  C83110913CBE4583F820FABEB7514293624E46862FAE1FD339B923F0CACC647D%           

  cat ~/private-key.txt.pub
  B881EA2C174D8211C123E5A91D86227DB116A44BB345A6E66874F83D8993F813%
  ```

  **Using Docker**

  Run this step from inside the `aptoslabs/tools` Docker container. Open a new terminal and `cd` into the directory where you started the Docker container for your fullnode. Making sure to provide the full path to where you want the private key TXT file to be stored, run the command as below:

  ```bash
  aptos key generate \
      --key-type x25519 \
      --output-file /path/to/private-key.txt
  ```

3. Retrieve the peer identity
  
  **Using Aptos-core source code**

  :::tip Required: host information
  Use the `--host` flag to provide the host information to output a network address for the fullnode. 
  :::

  ```bash
  aptos key extract-peer --host example.com:6180 \
      --public-network-key-file private-key.txt.pub \
      --output-file peer-info.yaml
  ```
  which will produce the following output:
  ```json
  {
    "Result": {
      "B881EA2C174D8211C123E5A91D86227DB116A44BB345A6E66874F83D8993F813": {
        "addresses": [
          "/dns/example.com/tcp/6180/noise-ik/0xB881EA2C174D8211C123E5A91D86227DB116A44BB345A6E66874F83D8993F813/handshake/0"
        ],
        "keys": [
          "0xB881EA2C174D8211C123E5A91D86227DB116A44BB345A6E66874F83D8993F813"
        ],
        "role": "Upstream"
      }
    }
  }
  ```
  or
  ```bash
  aptos key extract-peer --host 1.1.1.1:6180 \
      --public-network-key-file private-key.txt.pub \
      --output-file peer-info.yaml
  ```
  which will produce the following output:
  ```json
  {
    "Result": {
      "B881EA2C174D8211C123E5A91D86227DB116A44BB345A6E66874F83D8993F813": {
        "addresses": [
          "/ip4/1.1.1.1/tcp/6180/noise-ik/0xB881EA2C174D8211C123E5A91D86227DB116A44BB345A6E66874F83D8993F813/handshake/0"
        ],
        "keys": [
          "0xB881EA2C174D8211C123E5A91D86227DB116A44BB345A6E66874F83D8993F813"
        ],
        "role": "Upstream"
      }
    }
  }
  ```

  **Using Docker**

  Run the same above commands to extract the peer from inside the `aptoslabs/tools` Docker container. For example:

  ```bash
  aptos key extract-peer --host 1.1.1.1:6180 \
      --public-network-key-file /path/to/private-key.txt.pub \
      --output-file /path/to/peer-info.yaml
  ```

  This will create a YAML file that will have your `peer_id` corresponding to the `private-key.txt` you provided.

  Example output `peer-info.yaml` for the `--host example.com:6180` option:

   ```yaml
   ---
   B881EA2C174D8211C123E5A91D86227DB116A44BB345A6E66874F83D8993F813:
     addresses: ["/dns/example.com/tcp/6180/noise-ik/0xB881EA2C174D8211C123E5A91D86227DB116A44BB345A6E66874F83D8993F813/handshake/0"]
     keys:
       - "0xB881EA2C174D8211C123E5A91D86227DB116A44BB345A6E66874F83D8993F813"
   role: Upstream
    ```

  In this example, `B881EA2C174D8211C123E5A91D86227DB116A44BB345A6E66874F83D8993F813` is the `peer_id`. Use this in the `peer_id` field of your `fullnode.yaml` to create a static identity for your fullnode.


## Start a node with a static network identity

After you generated the public identity key you can startup the fullnode with a static network identity by using the public key in the `peer_id` field of the configuration file `fullnode.yaml`:

```yaml
full_node_networks:
- network_id: "public"
  discovery_method: "onchain"
  identity:
    type: "from_config"
    key: "<PRIVATE_KEY>"
    peer_id: "<PEER_ID>"
```

In our example, you would specify the above-generated `peer_id` in place of the `<PEER_ID>`:

```yaml
full_node_networks:
- network_id: "public"
  discovery_method: "onchain"
  identity:
    type: "from_config"
    key: "C83110913CBE4583F820FABEB7514293624E46862FAE1FD339B923F0CACC647D"
    peer_id: "B881EA2C174D8211C123E5A91D86227DB116A44BB345A6E66874F83D8993F813"
```

## Allowing other fullnodes to connect

:::tip Ports and port settings

See [Ports and port settings](/nodes/validator-node/operator/node-requirements#networking-requirements) for an explanation of port settings and how they are used.
:::

Once you start your fullnode with a static identity you can allow others to connect to devnet through your node.

:::tip

In the below steps, the port numbers used are for illustration only. You can use your choice of port numbers.

:::

- Make sure you open port `6180` (or `6182`, for example, depending on which port your node is listening to) and that you open your firewall.
- If you are using Docker, simply add `- "6180:6180"` or `- "6182:6182"` under ports in your ``docker-compose.yaml`` file.
- Share your fullnode static network identity with others. They can then use it in the `seeds` key of their `fullnode.yaml` file to connect to your fullnode.
- Make sure the port number you put in the `addresses` matches the one you have in the fullnode configuration file `fullnode.yaml` (for example, `6180` or `6182`).

Share your fullnode static network identity in the following format in the Discord channel `advertise-full-nodes`:

  ```yaml
  <Peer_ID>:
    addresses:
    # with DNS
    - "/dns4/<DNS_Name>/tcp/<Port_Number>/noise-ik/<Public_Key>/handshake/0"
    role: Upstream
  <Peer_ID>:
    addresses:
    # with IP
    - "/ip4/<IP_Address>/tcp/<Port_Number>/noise-ik/<Public_Key>/handshake/0"
    role: Upstream
  ```

 For example:

  ```yaml
  B881EA2C174D8211C123E5A91D86227DB116A44BB345A6E66874F83D8993F813:
    addresses:
    - "/dns4/pfn0.node.devnet.aptoslabs.com/tcp/6182/noise-ik/B881EA2C174D8211C123E5A91D86227DB116A44BB345A6E66874F83D8993F813/handshake/0"
    role: "Upstream"
  B881EA2C174D8211C123E5A91D86227DB116A44BB345A6E66874F83D8993F813:
    addresses:
    - "/ip4/100.20.221.187/tcp/6182/noise-ik/B881EA2C174D8211C123E5A91D86227DB116A44BB345A6E66874F83D8993F813/handshake/0"
    role: "Upstream"
  ```

:::tip

Peer ID is synonymous with `AccountAddress`. See [NetworkAddress](https://github.com/aptos-labs/aptos-core/blob/main/documentation/specifications/network/network-address.md) to see how `addresses` key value is constructed.

:::
