---
title: "Node networks and synchronization"
slug: "basics-node-networks-sync"
---

# Node network topology

Validator nodes and FullNodes form a hierarchical structure with validator nodes at the root and FullNodes everywhere else. The Aptos Blockchain distinguishes two types of FullNodes: Validator FullNodes and Public FullNodes. Validator FullNodes connect directly to validator nodes and offer scalability alongside DDoS mitigation. Public FullNodes connect to Validator FullNodes (or other Public FullNodes) to gain low-latency access to the Aptos network.

![v-fn-network.svg](/img/docs/v-fn-network.svg)

## Separate network stacks
The Aptos Blockchain supports distinct networking stacks for various network topologies. For example, the validator network is independent of the FullNode network. The advantages of having separate network stacks include:
* Clean separation between the different networks.
* Better support for security preferences (e.g., bidirectional vs server authentication).
* Allowance for isolated discovery protocols (i.e., on-chain discovery for validator node's public endpoints vs. manual configuration for private organizations).

# Node synchronization
Aptos nodes synchronize to the latest state of the Aptos Blockchain through two mechanisms: consensus or state synchronization. Validator nodes will use both consensus and state synchronization to stay up-to-date, while FullNodes use only state synchronization.

For example, a validator node will invoke state synchronization when it comes online for the first time or reboots (e.g., after being offline for a while). Once the validator is up-to-date with the latest state of the blockchain it will begin participating in consensus and rely exclusively on consensus to stay up-to-date. FullNodes, however, continuously rely on state synchronization to get and stay up-to-date as the blockchain grows.

## State synchronizer
Each Aptos node contains a [State Synchronizer](https://github.com/aptos-labs/aptos-core/tree/main/state-sync) component which is used to synchronize the state of the node with its peers. This component has the same functionality for all types of Aptos nodes: it utilizes the dedicated peer-to-peer network to continuously request and disseminate blockchain data. Validator nodes distribute blockchain data within the validator node network, while FullNodes rely on other FullNodes (i.e., Validator or Public FullNodes).

## Synchronization API
The Aptos node's state synchronizer communicates with other nodes' state synchronizers to get and send chunks of transactions. Learn more about how this works in the specifications [here](https://github.com/aptos-labs/aptos-core/tree/main/documentation/specifications/state_sync).
