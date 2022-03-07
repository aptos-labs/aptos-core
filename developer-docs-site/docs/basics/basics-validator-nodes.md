---
title: "Validator nodes"
slug: "basics-validator-nodes"
---
import BlockQuote from "@site/src/components/BlockQuote";

An Aptos node is a peer entity of the Aptos ecosystem that tracks the [state](/reference/glossary#state) of the Aptos Blockchain. Clients interact with the blockchain via Aptos nodes. There are two types of nodes:
* Validator nodes
* [FullNodes](basics-fullnodes.md)

Each Aptos node comprises several logical components:
* [REST service](/reference/glossary#rest-service)
* [Mempool](#mempool)
* [Consensus (disabled in FullNodes)](#consensus)
* [Execution](#execution)
* [Virtual Machine](#virtual-machine)
* [Storage](#storage)
* [State synchronizer](#state-synchronizer)

The [Aptos Core](/reference/glossary#aptos-core) software can be configured to run as a validator node or as a FullNode.

# Overview

When a transaction is submitted to the Aptos Blockchain, validator nodes run a distributed [consensus protocol](/reference/glossary#consensus-protocol), execute the transaction, and store the transaction and the execution results on the blockchain. Validator nodes decide which transactions will be added to the blockchain and in which order.

The Aptos Payment Network uses a Byzantine Fault Tolerance (BFT) consensus protocol for validator nodes to agree on the ledger of finalized transactions and their execution. Validator nodes process these transactions to include them in the blockchain’s database, which they maintain. This means that validator nodes always have the current [state](/reference/glossary#state) of the blockchain.

A validator node communicates directly with other validator nodes over a private network. [FullNodes](basics-fullnodes.md) are an external validation resource for finalized transaction history. They receive transactions from upstream nodes and then re-execute them locally (the same way a validator node executes transactions). FullNodes store the results of the re-execution to local storage. In doing so, they will notice and can provide evidence if there is any attempt to rewrite history. This helps to ensure that the validator nodes are not colluding on arbitrary transaction execution.

<BlockQuote type="info">
The AptosBFT consensus protocol provides fault tolerance of up to one-third of malicious validator nodes.
</BlockQuote>

## Validator node components

![validator.svg](/img/docs/validator.svg)
### Mempool

Mempool is a validator node component that holds an in-memory buffer of transactions that have been submitted but not yet agreed upon and executed. This buffer is replicated between validator nodes.

The JSON-RPC service of a FullNode sends transactions to a validator node's mempool. Mempool performs initial checks on the requests to protect the other parts of the validator node from corrupt or high volume input. When a new transaction passes the initial checks and is added to the mempool, it is then shared to the mempools of other validator nodes in the Aptos Payment Network.

When a validator node is the leader, its consensus component pulls the transactions from its mempool and proposes the order of the transactions that form a block by broadcasting it to other validators. The validators then execute and submit votes on the proposal.

### Consensus

Consensus is the component that is responsible for ordering blocks of transactions and agreeing on the results of execution by participating in the consensus protocol with other validator nodes in the network.

### Execution

Execution is the component that coordinates the execution of a block of transactions and maintains a transient state. The consensus component votes on this transient state. The execution component maintains an in-memory representation of the execution results until the consensus component commits the block to the distributed database. The execution component uses the virtual machine to execute transactions. Execution acts as the glue layer between the input of the system represented by transactions, storage providing a persistency layer, and the virtual machine for execution.

### Virtual machine

The virtual machine component is used to run the Move program included in a submitted transaction and determine the results.  A node's mempool uses the virtual machine component to perform validation checks on transactions, while its execution component uses it to execute transactions.

### Storage

The storage component is used to persist agreed upon blocks of transactions and their execution results.

### State synchronizer

Nodes use their state synchronizer component to “catch up” to the latest state of the blockchain.
