---
title: "Life of a Transaction"
slug: "basics-life-of-txn"
---
import BlockQuote from "@site/src/components/BlockQuote";

To get a deeper understanding of the lifecycle of an Aptos transaction (from an operational perspective), we will follow a transaction on its journey, from being submitted to an Aptos fullnode, to being committed to the Aptos blockchain. We will then *zoom-in* on the logical components of Aptos nodes and take a look how the transaction interacts with these components.

# Assumptions

For the purpose of this doc, we will assume that:

* Alice and Bob are two users who each have an [account](/reference/glossary#account) on the Aptos blockchain.
* Alice's account has 110 Aptos Coins.
* Alice is sending 10 Aptos Coins to Bob.
* The current [sequence number](/reference/glossary#sequence-number) of Alice's account is 5 (which indicates that 5 transactions have already been sent from Alice's account).
* There are a total of 100 validator nodes &mdash; V<sub>1</sub> to V<sub>100</sub> on the network.
* An Aptos client submits Alice's transaction to a REST service on an Aptos Fullnode. The fullnode forwards this transaction to a validator fullnode which in turn forwards it to validator V<sub>1</sub>.
* Validator V<sub>1</sub> is a proposer/leader for the current round.

# Client submits a transaction

An Aptos **client constructs a raw transaction** (let's call it Traw<sub>5</sub>) to transfer 10 Aptos Coins from Alice’s account to Bob’s account. The Aptos client signs the transaction with Alice's private key. The signed transaction T<sub>5</sub> includes the following:

* The raw transaction.
* Alice's public key.
* Alice's signature.

The raw transaction includes the following fields:

| Fields | Description |
| ------ | ----------- |
| [Account address](/reference/glossary#account-address) | Alice's account address |
| Move module | A module (or program) that indicates the actions to be performed on Alice's behalf. In this case, it contains:  <br />- A Move bytecode peer-to-peer [transaction script](/reference/glossary#transaction-script) <br />- A list of inputs to the script (for this example the list would contain Bob's account address and the payment amount in Aptos Coins). |
| [Maximum gas amount](/reference/glossary#maximum-gas-amount) | The maximum gas amount Alice is willing to pay for this transaction. Gas is a way to pay for computation and storage. A gas unit is an abstract measurement of computation. |
| [Gas price](/reference/glossary#gas-price) | The amount (in Aptos Coins) Alice is willing to pay per unit of gas, to execute the transaction. |
| [Expiration time](/reference/glossary#expiration-time) | Expiration time of the transaction. |
| [Sequence number](/reference/glossary#sequence-number)  | The sequence number (5, in this example) for an account indicates the number of transactions that have been submitted and committed on-chain from that account. In this case, 5 transactions have been submitted from Alice’s account, including Traw<sub>5</sub>. Note: a transaction with sequence number 5 can only be committed on-chain if the account sequence number is 5. |
| [Chain ID](https://github.com/aptos-labs/aptos-core/blob/main/types/src/chain_id.rs) | An identifier that distinguishes the Aptos network deployments (to prevent cross-network attacks). |

# Lifecycle of the transaction

In this section, we will describe the lifecycle of transaction T<sub>5</sub>, from when the client submits it to when it is committed to the Aptos blockchain.

For the relevant steps, we've included a link to the corresponding inter-component interactions of the validator node. After you are familiar with all the steps in the lifecycle of the transaction, you may want to refer to the information on the corresponding inter-component interactions for each step.
![Figure 1.0 Lifecycle of a transaction](/img/docs/validator-sequence.svg)
<small className="figure">Figure 1.0 Lifecycle of a transaction</small>

<BlockQuote type="warning">
 The arrows in all the visuals in this article originate on the component initiating an interaction/action and terminate on the component on which the action is being performed. The arrows do not represent data read, written, or returned.
</BlockQuote>

The lifecycle of a transaction has five stages:

* **Accepting**: [Accepting the transaction](#accepting-the-transaction)
* **Sharing**: [Sharing the transaction with other validator nodes](#sharing-the-transaction-with-other-validator-nodes)
* **Proposing**: [Proposing the block](#proposing-the-block)
* **Executing and Consensus**: [Executing the block and reaching consensus](#executing-the-block-and-reaching-consensus)
* **Committing**: [Committing the block](#committing-the-block)

We've described what happens in each stage below, along with links to the corresponding Aptos node component interactions.

## Accepting the transaction

| Description                                                  | Aptos Node Component Interactions                           |
| ------------------------------------------------------------ | ---------------------------------------------------------- |
| 1. **Client → REST service**: The client submits transaction T<sub>5</sub> to the REST service of an Aptos fullnode. The fullnode uses the REST service to forward the transaction to its own mempool, which then forwards the transaction to mempools running on other nodes in the network. The transaction will eventually be forwarded to a mempool running on a validator Fullnode, which will send it to a validator node (V<sub>1</sub> in this case). | [1. REST Service](#1-client--rest-service)                  |
| 2. **REST service → Mempool**: The fullnode's REST service transmits transaction T<sub>5</sub> to validator V<sub>1</sub>'s mempool. | [2. REST Service](#2-rest-service--mempool), [1. Mempool](#1-rest-service--mempool) |
| 3. **Mempool → Virtual Machine (VM)**: Mempool will use the virtual machine (VM) component to perform transaction validation, such as signature verification, account balance verification and replay resistance using the sequence number. | [4. Mempool](#4-mempool--vm), [3. Virtual Machine](#3-mempool--virtual-machine) |


## Sharing the transaction with other validator nodes

| Description                                                  | Aptos Node Component Interactions |
| ------------------------------------------------------------ | -------------------------------- |
| 4. **Mempool**: The mempool will hold T<sub>5</sub> in an in-memory buffer. Mempool may already contain multiple transactions sent from Alice's address. | [Mempool](#mempool)                  |
| 5. **Mempool → Other Validators**: Using the shared-mempool protocol, V<sub>1</sub> will share the transactions (including T<sub>5</sub>) in its mempool with other validator nodes and place transactions received from them into its own (V<sub>1</sub>) mempool. | [2. Mempool](#2-mempool--other-validator-nodes)               |

## Proposing the block

| Description                                                  | Aptos Node Component Interactions         |
| ------------------------------------------------------------ | ---------------------------------------- |
| 6. **Consensus → Mempool**: &mdash; As validator V<sub>1</sub> is a proposer/leader for this transaction, it will pull a block of transactions from its mempool and replicate this block as a proposal to other validator nodes via its consensus component. | [1. Consensus](#1-consensus--mempool), [3. Mempool](#3-consensus--mempool) |
| 7. **Consensus → Other Validators**: The consensus component of V<sub>1</sub> is responsible for coordinating agreement among all validators on the order of transactions in the proposed block. | [2. Consensus](#2-consensus--other-validators)                     |

## Executing the block and reaching consensus

| Description                                                  | Aptos Node Component Interactions                 |
| ------------------------------------------------------------ | ------------------------------------------------ |
| 8. **Consensus → Execution**: As part of reaching agreement, the block of transactions (containing T<sub>5</sub>) is shared with the execution component. | [3. Consensus](#3-consensus--execution-consensus--other-validators), [1. Execution](#1-consensus--execution)       |
| 9. **Execution → Virtual Machine**: The execution component manages the execution of transactions in the VM. Note that this execution happens speculatively before the transactions in the block have been agreed upon. | [2. Execution](#2-execution--vm), [3. Virtual Machine](#3-mempool--virtual-machine) |
| 10. **Consensus → Execution**: After executing the transactions in the block, the execution component appends the transactions in the block (including T<sub>5</sub>) to the [Merkle accumulator](/reference/glossary#merkle-accumulator) (of the ledger history). This is an in-memory/temporary version of the Merkle accumulator. The necessary part of the proposed/speculative result of executing these transactions is returned to the consensus component to agree on. The arrow from "consensus" to "execution" indicates that the request to execute transactions was made by the consensus component. | [3. Consensus](#3-consensus--execution-consensus--other-validators), [1. Execution](#1-consensus--execution)       |
| 11. **Consensus → Other Validators**: V<sub>1</sub> (the consensus leader) attempts to reach consensus on the proposed block's execution result with the other validator nodes participating in consensus. | [3. Consensus](#3-consensus--execution-consensus--other-validators)                             |

## Committing the block

| Description                                                  | Aptos Node Component Interactions                             |
| ------------------------------------------------------------ | ------------------------------------------------------------ |
| 12. **Consensus → Execution**, **Execution → Storage**: If the proposed block's execution result is agreed upon and signed by a set of validators that have the quorum of votes, validator V<sub>1</sub>'s execution component reads the full result of the proposed block execution from the speculative execution cache and commits all the transactions in the proposed block to persistent storage with their results. | [4. Consensus](#4-consensus--execution), [3. Execution](#3-consensus--execution), [4. Execution](#4-execution--storage), [3. Storage](#3-execution--storage) |

Alice's account will now have 100 Aptos Coins, and its sequence number will be 6. If T<sub>5</sub> is replayed by Bob, it will be rejected as the sequence number of Alice's account (6) is greater than the sequence number of the replayed transaction (5).

# Aptos node component interactions

In the [previous section](#lifecycle-of-the-transaction), we described the typical lifecycle of a transaction (from transaction submission to transaction commit). Now let's look at the inter-component interactions of Aptos nodes as the blockchain processes transactions and responds to queries. This information will be most useful to those who:

* Would like to get an idea of how the system works under the covers.
* Are interested in eventually contributing to the Aptos blockchain.

You can learn more about the different types of Aptos nodes here:
* [Validator nodes](/concepts/basics-validator-nodes)
* [Fullnodes](/concepts/basics-fullnodes)

For our narrative, we will assume that a client submits a transaction T<sub>N</sub> to a validator V<sub>X</sub>. For each validator component, we will describe each of its inter-component interactions in subsections under the respective component's section. Note that subsections describing the inter-component interactions are not listed strictly in the order in which they are performed. Most of the interactions are relevant to the processing of a transaction, and some are relevant to clients querying the blockchain (queries for existing information on the blockchain).

The following are the core components of an Aptos node used in the lifecycle of a transaction:

**Fullnode**

* [REST Service](#rest-service)

**Validator node**

* [Mempool](#mempool)
* [Consensus](#consensus)
* [Execution](#execution)
* [Virtual Machine](#virtual-machine-vm)
* [Storage](#storage)

## REST Service
![Figure 1.1 REST Service](/img/rest-service.svg)
<small className="figure">Figure 1.1 REST Service</small>

Any request made by a client goes to the REST Service of a fullnode first. Then, the submitted transaction is forwarded to the validator fullnode, which then sends it to the validator node V<sub>X</sub>.

### 1. Client → REST Service

A client submits a transaction to the REST service of an Aptos fullnode.

### 2. REST Service → Mempool

The REST service forwards the transaction to a validator fullnode, which then sends it to validator node V<sub>X</sub>'s mempool. The mempool will accept the transaction T<sub>N</sub> only if the sequence number of T<sub>N</sub> is greater than or equal to the current sequence number of the sender's account (note that the transaction will not be passed to consensus until the sequence number matches the sequence number of the sender’s account).

### 3. REST Service → Storage

When a client performs a read query on the Aptos blockchain (for example, to get the balance of Alice's account), the REST service interacts with the storage component directly to obtain the requested information.


## Virtual Machine (VM)
![Figure 1.2 Virtual machine](/img/docs/virtual-machine.svg)
<small className="figure">Figure 1.2 Virtual machine</small>

The Move VM verifies and executes transaction scripts written in Move bytecode.

### 1. Virtual Machine → Storage

When mempool requests the VM to validate a transaction via `VM::ValidateTransaction()`, the VM loads the transaction sender's account from storage and performs verifications, some of which have been described in the list below. View the entire list of verifications [here](https://github.com/aptos-labs/aptos-core/tree/main/documentation/specifications/move_adapter).

* Checks that the input signature on the signed transaction is correct (to reject incorrectly signed transactions).
* Checks that the sender's account authentication key is the same as the hash of the public key (corresponding to the private key used to sign the transaction).
* Verifies that the sequence number for the transaction is greater than or equal to the current sequence number for the sender's account. Completing this check prevents the replay of the same transaction against the sender's account.
* Verifies that the program in the signed transaction is not malformed, as a malformed program cannot be executed by the VM.
* Verifies that the  sender's account balance contains at least the maximum gas amount multiplied by the gas price specified in the transaction, which ensures that the transaction can pay for the resources it uses.

### 2. Execution → Virtual Machine

The execution component utilizes the VM to execute a transaction via `VM::ExecuteTransaction()`.

It is important to understand that executing a transaction is different from updating the state of the ledger and persisting the results in storage. A transaction T<sub>N</sub> is first executed as part of an attempt to reach agreement on blocks during consensus. If agreement is reached with the other validators on the ordering of transactions and their execution results, the results are persisted in storage and the state of the ledger is updated.

### 3. Mempool → Virtual Machine

When mempool receives a transaction from other validators via shared mempool or from the REST service, mempool invokes `VM::ValidateTransaction()` on the VM to validate the transaction.

For implementation details refer to the [Virtual Machine README](https://github.com/move-language/move/tree/main/language/move-vm).

## Mempool


![Figure 1.3 Mempool](/img/docs/mempool.svg)
<small className="figure">Figure 1.3 Mempool</small>

Mempool is a shared buffer that holds the transactions that are “waiting” to be executed. When a new transaction is added to the mempool, the mempool shares this transaction with other validator nodes in the system. To reduce network consumption in the “shared mempool,” each validator is responsible for delivering its own transactions to other validators. When a validator receives a transaction from the mempool of another validator, the transaction is added to the mempool of the recipient validator.

### 1. REST Service → Mempool

* After receiving a transaction from the client, the REST service proxies the transaction to a validator fullnode. The transaction is then sent to the validator node’s mempool.
* The mempool for validator node V<sub>X</sub> accepts transaction T<sub>N</sub> for the sender's account only if the sequence number of T<sub>N</sub> is greater than or equal to the current sequence number of the sender's account.

### 2. Mempool → Other validator nodes

* The mempool of validator node V<sub>X</sub> shares transaction T<sub>N</sub> with the other validators on the same network.
* Other validators share the transactions in their respective mempools with V<sub>X</sub>’s mempool.

### 3. Consensus → Mempool

* When the transaction is forwarded to a validator node and once the validator node becomes the leader, its consensus component will pull a block of transactions from its mempool and replicate the proposed block to other validators. It does this to arrive at a consensus on the ordering of transactions and the execution results of the transactions in the proposed block.
* Note that just because a transaction T<sub>N</sub> was included in a proposed consensus block, it does not guarantee that T<sub>N </sub>will eventually be persisted in the distributed database of the Aptos blockchain.


### 4. Mempool → VM

When mempool receives a transaction from other validators, mempool invokes <code>VM::ValidateTransaction()</code> on the VM to validate the transaction.


For implementation details refer to the [Mempool README](https://github.com/aptos-labs/aptos-core/tree/main/documentation/specifications/mempool).

## Consensus
![Figure 1.4 Consensus](/img/docs/consensus.svg)
<small className="figure">Figure 1.4 Consensus</small>

The consensus component is responsible for ordering blocks of transactions and agreeing on the results of execution by participating in the [consensus protocol](/reference/glossary/#consensus-protocol) with other validators in the network.


### 1. Consensus → Mempool

When validator V<sub>X</sub> is a leader/proposer, the consensus component of V<sub>X</sub> pulls a block of transactions from its mempool via: `Mempool::GetBlock()`, and forms a proposed block of transactions.

### 2. Consensus → Other Validators

If V<sub>X</sub> is a proposer/leader, its consensus component replicates the proposed block of transactions to other validators.

### 3. Consensus → Execution, Consensus → Other Validators

* To execute a block of transactions, consensus interacts with the execution component. Consensus executes a block of transactions via `Execution:ExecuteBlock()` (Refer to [Consensus → execution](#1-consensus--execution))
* After executing the transactions in the proposed block, the execution component responds to the consensus component with the result of executing these transactions.
* The consensus component signs the execution result and attempts to reach agreement on this result with other validators.

## 4. Consensus → Execution

If enough validators vote for the same execution result, the consensus component of V<sub>X</sub> informs execution via `Execution::CommitBlock()` that this block is ready to be committed.


For implementation details refer to the [Consensus README](https://github.com/aptos-labs/aptos-core/tree/main/documentation/specifications/consensus).

## Execution
![Figure 1.5 Execution](/img/docs/execution.svg)
<small className="figure">Figure 1.5 Execution</small>

The execution component coordinates the execution of a block of transactions and maintains a transient state that can be voted upon by consensus. If these transactions are successful, they are committed to storage.

### 1. Consensus → Execution

* Consensus requests execution to execute a block of transactions via: `Execution::ExecuteBlock()`.
* Execution maintains a “scratchpad,” which holds in-memory copies of the relevant portions of the [Merkle accumulator](/reference/glossary#merkle-accumulator). This information is used to calculate the root hash of the current state of the Aptos blockchain.
* The root hash of the current state is combined with the information about the transactions in the proposed block to determine the new root hash of the accumulator. This is done prior to persisting any data, and to ensure that no state or transaction is stored until agreement is reached by a quorum of validators.
* Execution computes the speculative root hash and then the consensus component of V<sub>X</sub> signs this root hash and attempts to reach agreement on this root hash with other validators.

### 2. Execution → VM

When consensus requests execution to execute a block of transactions via `Execution::ExecuteBlock()`, execution uses the VM to determine the results of executing the block of transactions.

### 3. Consensus → Execution

If a quorum of validators agrees on the block execution results, the consensus component of each validator informs its execution component via `Execution::CommitBlock()` that this block is ready to be committed. This call to the execution component will include the signatures of the validators to provide proof of their agreement.

### 4. Execution → Storage

Execution takes the values from its “scratchpad” and sends them to storage for persistence via `Storage::SaveTransactions()`. Execution then prunes the old values from the “scratchpad” that are no longer needed (for example, parallel blocks that cannot be committed).

For implementation details refer to the [Execution README](https://github.com/aptos-labs/aptos-core/tree/main/execution).

## Storage
![Figure 1.6 Storage](/img/docs/storage.svg)
<small className="figure">Figure 1.6 Storage</small>

The storage component persists agreed upon blocks of transactions and their execution results to the Aptos blockchain. A block of transactions (which includes transaction T<sub>N</sub>) will be saved via storage when there is agreement between more than a quorum (2f+1) of the validators participating in consensus. Agreement must include all of the following:
* The transactions to include in the block
* The order of the transactions
* The execution results of the transactions in the block

Refer to [Merkle accumulator](/reference/glossary#merkle-accumulator) for information on how a transaction is appended to the data structure representing the Aptos blockchain.

### 1. VM → Storage

When mempool invokes `VM::ValidateTransaction()` to validate a transaction, `VM::ValidateTransaction()` loads the sender's account from storage and performs read-only validity checks on the transaction.

### 2. Execution → Storage

When the consensus component calls `Execution::ExecuteBlock()`, execution reads the current state from storage combined with the in-memory “scratchpad” data to determine the execution results.

### 3. Execution → Storage

Once consensus is reached on a block of transactions, execution calls storage via `Storage::SaveTransactions()` to save the block of transactions and permanently record them. This will also store the signatures from the validator nodes that agreed on this block of transactions. The in-memory data in “scratchpad” for this block is passed to update storage and persist the transactions. When the storage is updated, every account that was modified by these transactions will have its sequence number incremented by one.

Note: The sequence number of an account on the Aptos blockchain increments by one for each committed transaction originating from that account.

### 4. REST Service → Storage

For client queries that read information from the blockchain, the REST service directly interacts with storage to read the requested information.

For implementation details refer to the [Storage README](https://github.com/aptos-labs/aptos-core/tree/main/storage).
