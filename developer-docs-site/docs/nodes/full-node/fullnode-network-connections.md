---
title: "Fullnode Network Connections"
slug: "fullnode-network-connections"
---

# Fullnode Network Connections

When running a fullnode on an Aptos network, you can configure your node's
network connections for a few different purposes. For example, you can add
a seed peer to your node's configuration YAML to connect your node to a
specific peer of your choosing. Or you can create a static network identity
for your node to allow other nodes to connect to you, as described in [Network Identity For Fullnode](/nodes/full-node/network-identity-fullnode).

This document describes how to configure the network of your fullnode for
different deployments and requirements, including:

- Allowing fullnodes to connect to your node.
- Connecting your fullnode to an Aptos blockchain deployment.
- Connecting your fullnode to seed peers.
- Configuring priority access for other fullnodes.
- Configuring your fullnode as a private fullnode.

## Allowing fullnodes to connect to your node

:::tip Before you proceed

Before allowing other fullnodes to connect to your fullnode,
be sure to create a fullnode identity. See [Network Identity For Fullnode](/nodes/full-node/network-identity-fullnode).

:::

Once you start your fullnode with a static identity you can allow others to connect to your fullnode:

:::tip

In the below steps, the port numbers used are for illustration only. You can
use your choice of port numbers. See [Ports and port settings](/nodes/validator-node/operator/node-requirements#networking-requirements) for an explanation of port settings and how they are used.

:::

- Make sure you open port `6180` (or `6182`, for example, depending on which port your node is listening to) and that you open your firewall.
- If you are using Docker, simply add `- "6180:6180"` or `- "6182:6182"` under ports in your ``docker-compose.yaml`` file.
- Share your fullnode static network identity with others. They can then use it in the `seeds` key of their `fullnode.yaml` file to connect to your fullnode. See the section below.
- Make sure the port number you put in the `addresses` matches the one you have in the fullnode configuration file `fullnode.yaml` (for example, `6180` or `6182`).

Share your fullnode static network identity in the following format in our Discord to advertise your node.
Note, the Discord channel to share your identity may differ depending on the blockchain deployment you're running in.
See [Aptos Blockchain Deployments](/nodes/aptos-deployments) for more information.

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

Peer ID is synonymous with `AccountAddress`. See [NetworkAddress](https://github.com/aptos-labs/aptos-core/blob/main/documentation/specifications/network/network-address.md) to see how the `addresses` key value is constructed.

:::

## Connecting your fullnode to an Aptos blockchain deployment

When running a fullnode on an Aptos blockchain deployment, your node will be
able to discover other nodes in the network automatically, e.g., using the
genesis blob or the network addresses of the validators and validator fullnodes
registered on the blockchain. Be sure to download the correct genesis blob and
waypoint for your fullnode to ensure your node connects to the correct Aptos
blockchain deployment. See [Aptos Blockchain Deployments](/nodes/aptos-deployments)
for more information.

## Connecting your fullnode to seed peers

All Aptos fullnodes are configured to accept a maximum number of network
connections. As a result, if the network is experiencing high network
connection volume, your fullnode might not able to connect to the default
nodes in the network and you may see several errors in your node's logs, e.g.,
`No connected AptosNet peers!` or `Unable to fetch peers to poll!`.

If this happens continuously, you should manually add seed peers to your node's
configuration file to connect to other nodes.

:::tip

You may see `No connected AptosNet peers!` or `Unable to fetch peers to poll!` in your node's error messages. This is normal when the node is first starting.
Wait for the node to run for a few minutes to see if it connects to peers. If not, follow the below steps:

:::

See below for a few seed peer addresses you can use in your
`public_full_node.yaml` file. The peers you choose will differ based on the
blockchain deployment your node is running in.

:::tip

You can also use the fullnode addresses provided by the Aptos community. Anyone already running a fullnode can provide their address for you to connect. See the Aptos Discord.

:::


### Devnet seed peers

To add seeds to your devnet fullnode, add these to your `public_full_node.yaml` configuration file under your `discovery_method`, as shown in the below example:

```yaml
...
full_node_networks:
    - discovery_method: "onchain"
      listen_address: ...
      seeds: # All seeds are declared below
        bb14af025d226288a3488b4433cf5cb54d6a710365a2d95ac6ffbd9b9198a86a:
            addresses:
            - "/dns4/pfn0.node.devnet.aptoslabs.com/tcp/6182/noise-ik/bb14af025d226288a3488b4433cf5cb54d6a710365a2d95ac6ffbd9b9198a86a/handshake/0"
            role: "Upstream"
        7fe8523388084607cdf78ff40e3e717652173b436ae1809df4a5fcfc67f8fc61:
            addresses:
            - "/dns4/pfn1.node.devnet.aptoslabs.com/tcp/6182/noise-ik/7fe8523388084607cdf78ff40e3e717652173b436ae1809df4a5fcfc67f8fc61/handshake/0"
            role: "Upstream"
        f6b135a59591677afc98168791551a0a476222516fdc55869d2b649c614d965b:
            addresses:
            - "/dns4/pfn2.node.devnet.aptoslabs.com/tcp/6182/noise-ik/f6b135a59591677afc98168791551a0a476222516fdc55869d2b649c614d965b/handshake/0"
            role: "Upstream"
...
```

## Configuring priority access for other fullnodes

To configure your fullnode to allow another fullnode to connect to it even
when your fullnode has hit the maximum number of available network connections,
follow this method:

In the configuration file for your fullnode add the other fullnode as a seed
peer with the `Downstream` role. This will allow the other fullnode to connect
directly to you with priority access. In your fullnode configuration file, add:
```yaml
seeds:
  <other fullnode account>
    addresses:
    - <address of the other fullnode>
    role: Downstream # Allows the node to connect to us
```

Similarly, to make the other fullnode connect to yours, add the following to the
other fullnode's configuration file:
```yaml
seeds:
  <your fullnode account>
    addresses:
    - <address of your fullnode>
    role: PreferredUpstream # Allows the node to connect to the seed peer
```

## Configuring your fullnode as a private fullnode

You can also configure your fullnode as a private fullnode should you wish.
What this means is that your fullnode will not allow unauthenticated
connections, specifically, any node that is not a validator, validator
fullnode, or seed peer will be unable to connect to your fullnode.

To configure your fullnode as a private fullnode, add the following to your
fullnode configuration file. Note, you should add this to the first network
entry in the `full_node_networks` configuration:

```yaml
...
full_node_networks:
  - discovery_method: "onchain"
    listen_address: ...
    max_inbound_connections: 0  # Prevents any unauthenticated inbound connections
    mutual_authentication: true  # Requires authenticated connections
    ...
...
```
