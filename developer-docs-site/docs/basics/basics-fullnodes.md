---
title: "FullNodes"
slug: "basics-fullnodes"
hidden: false
---
A Aptos node is a peer entity of the Aptos ecosystem that tracks the [state](/reference/glossary#state) of the Aptos Blockchain. Clients interact with the blockchain via Aptos nodes. There are two types of nodes:
* [Validator nodes](basics-validator-nodes.md)
* FullNodes

Each Aptos node comprises several logical components:
* [JSON-RPC service](/reference/glossary#json-rpc-service) (disabled in validator nodes)
* [Mempool](basics-validator-nodes.md#mempool)
* [Consensus](basics-validator-nodes.md#consensus)
* [Execution](basics-validator-nodes.md#execution)
* [Virtual Machine](basics-validator-nodes.md#virtual-machine)
* [Storage](basics-validator-nodes.md#storage)
* [State synchronizer](basics-validator-nodes.md#state-synchronizer)

The [Aptos Core](/reference/glossary#aptos-core) software can be configured to run as a validator node or as a FullNode.

## Introduction

FullNodes can be run by anyone who wants to verify the state of the Aptos Blockchain and synchronize to it. FullNodes replicate the full state of the blockchain by querying each other or by querying the validator nodes directly.  They can also accept transactions submitted by Aptos clients and forward them to validator nodes.

Additionally, FullNodes are an external validation resource for finalized transaction history. They receive transactions from upstream nodes and then re-execute them locally (the same way a validator node executes transactions). FullNodes store the results of the re-execution to local storage. In doing so, they will notice and can provide evidence if there is any attempt to rewrite history. This helps to ensure that the validator nodes are not colluding on arbitrary transaction execution.

## Public FullNodes
A public FullNode uses the same software as a validator node and connects directly to one or more validator nodes to submit transactions and synchronize to the [state](/reference/glossary#state) of the Aptos Blockchain.

A public FullNode has all Aptos node components, with the consensus component being disabled.

Third-party blockchain explorers, wallets, exchanges, and DApps may run a local FullNode to:
* Leverage the JSON-RPC protocol for richer blockchain interactions.
* Get a consistent view of the Aptos Payment Network.
* Avoid rate limitations on read traffic.
* Run custom analytics on historical data.
* Get notifications about particular on-chain events.
