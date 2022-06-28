---
title: "FullNodes"
slug: "basics-fullnodes"
---
An Aptos node is an entity of the Aptos ecosystem that tracks the [state](/reference/glossary#state) of the Aptos Blockchain. Clients interact with the blockchain via Aptos nodes. There are two types of nodes:
* [Validator nodes](basics-validator-nodes.md)
* FullNodes

Each Aptos node comprises several logical components:
* [REST service](/reference/glossary#rest-service)
* [Mempool](basics-validator-nodes.md#mempool)
* [Execution](basics-validator-nodes.md#execution)
* [Virtual Machine](basics-validator-nodes.md#virtual-machine)
* [Storage](basics-validator-nodes.md#storage)
* [State synchronizer](basics-validator-nodes.md#state-synchronizer)

The [Aptos-core](/reference/glossary#aptos-core) software can be configured to run as a validator node or as a FullNode.

## Overview

FullNodes can be run by anyone. FullNodes re-execute all transactions in the history of the Aptos Blockchain. FullNodes replicate the entire state of the blockchain by synchronizing with upstream participants, e.g., other FullNodes or validator nodes. To verify blockchain state, FullNodes receive the set of transactions and the [accumulator hash root](/reference/glossary#accumulator-root-hash) of the ledger signed by the validators. In addition, FullNodes accept transactions submitted by Aptos clients and forward them directly (or indirecly) to validator nodes. While FullNodes and validators share the same code, FullNodes do not participate in consensus.

Third-party blockchain explorers, wallets, exchanges, and DApps may run a local FullNode to:
* Leverage the REST interface for blockchain interactions.
* Get a consistent view of the Aptos ledger.
* Avoid rate limitations on read traffic.
* Run custom analytics on historical data.
* Get notifications about particular on-chain events.
