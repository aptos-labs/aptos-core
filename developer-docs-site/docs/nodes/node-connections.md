---
title: "Node Connections"
slug: "node-connections"
---

# Node Connections

When running a node on an Aptos network, you can configure your node's network connections for a few different purposes. For example, you can add an upstream seed peer to your node's configuration YAML to connect your node to this peer, as described in [Add upstream seed peers](/nodes/full-node/fullnode-source-code-or-docker#add-upstream-seed-peers). Or you can create a static network identity for your node to allow other nodes to connect to your node, as shown in [Network Identity For Fullnode](/nodes/full-node/network-identity-fullnode).

This document describes how to configure your node for more sophisticated network connection requirements, such as configuring preferred access to a node, configuring a node as a private node, or setting the number of outbound connections. 

## Configuring preferred access to an inbound node

To configure an upstream fullnode (server side) to allow a downstream node to connect to it even when the network is saturated, follow this method:

In the `fullnode.yaml` configuration of the upstream full node (server side), add the downstream fullnode as a seed peer with the `Downstream​​` role. This will mean the upstream fullnode will not make outbound connections, but it will allow inbound connections from the downstream fullnode. See also https://aptos.dev/nodes/full-node/network-identity-fullnode#allowing-other-fullnodes-to-connect. 

In the upstream server's `fullnode.yaml`:
```yaml
 seeds:
  - addresses:
    - address of downstream client with key
  - role: Downstream
```

In the downstream client node's `fullnode.yaml`:
```yaml
seeds: 
- addresses:
  - Address of upstream server with key
  role: PreferredUpstream
```

## Configuring a node as private

To configure a node as a private node, set the following in the node configuraton YAML file:

- `full_node_network[0].max_inbound_connections = 0` 
- `full_node_network[0].mutual_authentication = true`

This will not allow unauthenticated connections.

## Setting the number of outbound connections

To set the number of outbound connections from a node, edit the following field in that node's configuration YAML. For example, to set the number to 4:

- `max_outbound_connections: 4`