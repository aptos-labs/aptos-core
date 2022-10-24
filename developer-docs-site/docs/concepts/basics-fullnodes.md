---
title: "Fullnodes"
slug: "basics-fullnodes"
---
An Aptos node is an entity of the Aptos ecosystem that tracks the [state](/reference/glossary#state) of the Aptos blockchain. Clients interact with the blockchain via Aptos nodes. There are two types of nodes:
* [Validator nodes](basics-validator-nodes.md)
* Fullnodes

Each Aptos node comprises several logical components:
* [REST service](/reference/glossary#rest-service)
* [Mempool](basics-validator-nodes.md#mempool)
* [Execution](basics-validator-nodes.md#execution)
* [Virtual Machine](basics-validator-nodes.md#virtual-machine)
* [Storage](basics-validator-nodes.md#storage)
* [State synchronizer](basics-validator-nodes.md#state-synchronizer)

The [Aptos-core](/reference/glossary#aptos-core) software can be configured to run as a validator node or as a fullnode.

## Overview

Fullnodes can be run by anyone. Fullnodes re-execute all transactions in the history of the Aptos blockchain. Fullnodes replicate the entire state of the blockchain by synchronizing with upstream participants, e.g., other fullnodes or validator nodes. To verify blockchain state, fullnodes receive the set of transactions and the [accumulator hash root](/reference/glossary#accumulator-root-hash) of the ledger signed by the validators. In addition, fullnodes accept transactions submitted by Aptos clients and forward them directly (or indirectly) to validator nodes. While fullnodes and validators share the same code, fullnodes do not participate in consensus.

Depending on the fullnode upstream, a fullnode can be called as a validator fullnode, or a public fullnode:
* **Validator fullnode** state sync from a validator node directly.
* **Public fullnode** state sync from other fullnodes.

There's no difference in their functionality, only whether their upstream node is a validator or another fullnode. Read more details about network topology [here](basics-node-networks-sync.md)

Third-party blockchain explorers, wallets, exchanges, and DApps may run a local fullnode to:
* Leverage the REST interface for blockchain interactions.
* Get a consistent view of the Aptos ledger.
* Avoid rate limitations on read traffic.
* Run custom analytics on historical data.
* Get notifications about particular on-chain events.
