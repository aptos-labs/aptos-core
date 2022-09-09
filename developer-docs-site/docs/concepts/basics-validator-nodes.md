---
title: "Validator Nodes"
slug: "basics-validator-nodes"
---
import BlockQuote from "@site/src/components/BlockQuote";

An Aptos node is an entity of the Aptos ecosystem that tracks the state of the Aptos blockchain. Clients interact with the blockchain via Aptos nodes. There are two types of nodes:
* Validator nodes
* [Fullnodes](basics-fullnodes.md)

Each Aptos node comprises several logical components:
* [REST service](/reference/glossary#rest-service)
* [Mempool](#mempool)
* [Consensus (disabled in fullnodes)](#consensus)
* [Execution](#execution)
* [Virtual Machine](#virtual-machine)
* [Storage](#storage)
* [State synchronizer](#state-synchronizer)

The [Aptos-core](/reference/glossary#aptos-core) software can be configured to run as a validator node or as a fullnode.

# Overview

When a transaction is submitted to the Aptos blockchain, validator nodes run a distributed [consensus protocol](/reference/glossary#consensus-protocol), execute the transaction, and store the transaction and the execution results on the blockchain. Validator nodes decide which transactions will be added to the blockchain and in which order.

The Aptos blockchain uses a Byzantine Fault Tolerance (BFT) consensus protocol for validator nodes to agree on the ledger of finalized transactions and their execution results. Validator nodes process these transactions and include them in their local copy of the blockchain database. This means that up-to-date validator nodes always maintain a copy of the current [state](/reference/glossary#state) of the blockchain, locally.

Validator nodes communicate directly with other validator nodes over a private network. [Fullnodes](basics-fullnodes.md) are an external validation and/or dissemination resource for the finalized transaction history. They receive transactions from peers and may re-execute them locally (the same way a validator executes transactions). Fullnodes store the results of re-executed transactions to local storage. In doing so, they can challenge any foul-play by validators and provide evidence if there is any attempt to re-write or modify the blockchain history. This helps to mitigate against validator corruption and/or collusion.

<BlockQuote type="info">
The AptosBFT consensus protocol provides fault tolerance of up to one-third of malicious validator nodes.
</BlockQuote>

## Validator node components

![validator.svg](/img/docs/validator.svg)
### Mempool

Mempool is a component within each node that holds an in-memory buffer of transactions that have been submitted to the blockchain, but not yet agreed upon or executed. This buffer is replicated between validator nodes and fullnodes.

The JSON-RPC service of a fullnode sends transactions to a validator node's mempool. Mempool performs various checks on the transactions to ensure transaction validity and protect against DOS attacks. When a new transaction passes initial verification and is added to mempool, it is then distributed to the mempools of other validator nodes in the network.

When a validator node temporarily becomes a leader in the consensus protocol, consensus pulls the transactions from mempool and proposes a new transaction block. This block is broadcasted to other validators and contains a total ordering over all transactions in the block. Each validator then executes the block and submits votes on whether or not to accept the new block proposal.

### Consensus

Consensus is the component that is responsible for ordering blocks of transactions and agreeing on the results of execution by participating in the consensus protocol with other validator nodes in the network.

### Execution

Execution is the component that coordinates the execution of a block of transactions and maintains a transient state. Consensus votes on this transient state. Execution maintains an in-memory representation of the execution results until consensus commits the block to the distributed database. Execution uses the virtual machine to execute transactions. Execution acts as the glue layer between the inputs of the system (represented by transactions), storage (providing a persistency layer), and the virtual machine (for execution).

### Virtual machine (VM)

The virtual machine (VM) is used to run the Move program within each transaction and determine execution results. A node's mempool uses the VM to perform verification checks on transactions, while execution uses the VM to execute transactions.

### Storage

The storage component is used to persist agreed upon blocks of transactions and their execution results to the local database.

### State synchronizer

Nodes use their state synchronizer component to “catch up” to the latest state of the blockchain and stay up-to-date.
