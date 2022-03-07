---
title: "Node networks and synchronization"
slug: "basics-node-networks-sync"
---
In this page, you will learn about:
* Types of node networks
* How and when a Aptos node synchronizes to the latest state of the Aptos Blockchain

# Node network topology

Validator nodes and FullNodes form a hierarchical architecture with validator nodes at the root. FullNodes near the validators offer scalability and provide DDoS resistance. Consumers may run their own FullNodes to gain access to the Aptos Blockchain. These FullNodes will likely connect to FullNodes run by validators and other upstream participants.

![v-fn-network.svg](/img/docs/v-fn-network.svg)

## Separate network stacks
The Aptos Blockchain supports distinct networking stacks for various network topologies. For example, the validator network is independent of the FullNode network. The advantages of having separate network stacks include:
* Clean separation between the different networks.
* Better support for security preferences (bidirectional vs server authentication).
* Allowance for isolated discovery protocols (on-chain discovery for validator node's public endpoint vs manual configuration for private organizations).

# Node synchronization
An Aptos node synchronizes to the latest state of the Aptos Blockchain when:
* It comes online for the first time (bootstrap).
* It restarts.
* It comes online after being offline for some time.
* When there is a network partition.
* FullNodes synchronize with their upstream nodes continuously during a normal workload.

## State synchronizer
Each Aptos node contains a State Synchronizer component which is used to synchronize the state of a node to its upstream peers. This component has the same function for all types of Aptos nodes. It utilizes the dedicated peer-to-peer network stack to perform synchronization and it uses a long-polling API.

The upstream peers that are used for synchronizing to the latest state of the blockchain are different for each type of node:
* Validator nodes use the validator node network.
* Public FullNodes can either use the initial set of peers or the validator nodes that are open for public access.

## Synchronization API
The Aptos node's state synchronizer communicates with the upstream nodes' state synchronizers to get chunks of transactions to synchronize with the current state of the Aptos Blockchain. Learn more about how this works in the specifications [here](https://github.com/aptos-labs/aptos-core/tree/main/documentation/specifications/state_sync).
