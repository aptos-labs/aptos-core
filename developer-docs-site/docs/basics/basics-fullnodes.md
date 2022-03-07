---
title: "FullNodes"
slug: "basics-fullnodes"
---
An Aptos node is a peer entity of the Aptos ecosystem that tracks the [state](/reference/glossary#state) of the Aptos Blockchain. Clients interact with the blockchain via Aptos nodes. There are two types of nodes:
* [Validator nodes](basics-validator-nodes.md)
* FullNodes

Each Aptos node comprises several logical components:
* [REST service](/reference/glossary#rest-service)
* [Mempool](basics-validator-nodes.md#mempool)
* [Consensus](basics-validator-nodes.md#consensus)
* [Execution](basics-validator-nodes.md#execution)
* [Virtual Machine](basics-validator-nodes.md#virtual-machine)
* [Storage](basics-validator-nodes.md#storage)
* [State synchronizer](basics-validator-nodes.md#state-synchronizer)

The [Aptos Core](/reference/glossary#aptos-core) software can be configured to run as a validator node or as a FullNode.

## Overview

FullNodes can be run by anyone. FullNodes re-execute all transactions in the history of the Aptos blockchain. FullNodes replicate the full state of the blockchain by synchronizing with an upstream participant either other FullNodes or validators, receiving the set of transactions and the [accumulator hash root](/reference/glossary#accumulator-root-hash) of the ledger after their execution signed by the validtors. In addition, FullNodes accept transactions submitted by Aptos clients and forward them upstream, eventually to validator nodes. While FullNodes and validators share the same code, FullNodes do not participate in consensus and only verify it.

Third-party blockchain explorers, wallets, exchanges, and DApps may run a local FullNode to:
* Leverage the REST interface for blockchain interactions.
* Get a consistent view of the Aptos ledger.
* Avoid rate limitations on read traffic.
* Run custom analytics on historical data.
* Get notifications about particular on-chain events.
