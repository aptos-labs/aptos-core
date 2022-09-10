---
title: "Node Networks and Synchronization"
slug: "basics-node-networks-sync"
---

# Node network topology

Validator nodes and fullnodes form a hierarchical structure with validator nodes at the root and fullnodes everywhere else. The Aptos blockchain distinguishes two types of fullnodes: validator fullnodes and public fullnodes. Validator fullnodes connect directly to validator nodes and offer scalability alongside DDoS mitigation. Public fullnodes connect to validator fullnodes (or other public fullnodes) to gain low-latency access to the Aptos network.

![v-fn-network.svg](/img/docs/v-fn-network.svg)

## Separate network stacks
The Aptos blockchain supports distinct networking stacks for various network topologies. For example, the validator network is independent of the fullnode network. The advantages of having separate network stacks include:
* Clean separation between the different networks.
* Better support for security preferences (e.g., bidirectional vs server authentication).
* Allowance for isolated discovery protocols (i.e., on-chain discovery for validator node's public endpoints vs. manual configuration for private organizations).

# Node synchronization
Aptos nodes synchronize to the latest state of the Aptos blockchain through two mechanisms: consensus or state synchronization. Validator nodes will use both consensus and state synchronization to stay up-to-date, while fullnodes use only state synchronization.

For example, a validator node will invoke state synchronization when it comes online for the first time or reboots (e.g., after being offline for a while). Once the validator is up-to-date with the latest state of the blockchain it will begin participating in consensus and rely exclusively on consensus to stay up-to-date. Fullnodes, however, continuously rely on state synchronization to get and stay up-to-date as the blockchain grows.

## State synchronizer

Each Aptos node contains a [State Synchronizer](./state-sync) component which is used to synchronize the state of the node with its peers. This component has the same functionality for all types of Aptos nodes: it utilizes the dedicated peer-to-peer network to continuously request and disseminate blockchain data. Validator nodes distribute blockchain data within the validator node network, while fullnodes rely on other fullnodes (i.e., validator nodes or public fullnodes).

