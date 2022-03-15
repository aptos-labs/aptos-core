---
title: "Transactions and states"
slug: "basics-txns-states"
---
The two fundamental concepts at the heart of the Aptos Blockchain are transactions and states:

* [Transactions](#transactions): Transactions represent the exchange of data (e.g., Aptos Coins or NFTs) between accounts on the Aptos Blockchain.
* [States](#ledger-state): The state (i.e., current blockchain ledger state) represents a snapshot of the blockchain as it currently stands.

When a transaction is executed, the state of the Aptos Blockchain changes.

# Transactions

When an Aptos Blockchain client submits a transaction, they are requesting that the ledger state be updated with their transaction.

A [signed transaction](/reference/glossary#transaction) on the blockchain contains the following information:

- **Signature**: The sender uses a digital signature to verify that they signed the transaction (i.e., authentication).
- **Sender address**: The sender's [account address](/reference/glossary#account-address).
- **Sender public key**: The public authentication key that corresponds to the private authentication key used to sign the transaction.
- **Program**: The program comprises:
  - A Move module and function name or a move bytecode transaction script.
  - An optional list of inputs to the script. For a peer-to-peer transaction, these inputs contain the recipient's information and the amount transferred to them.
  - An optional list of Move bytecode modules to publish.
- **Gas price** (in specified currency/gas units): This is the amount the sender is willing to pay per unit of [gas](/reference/glossary#gas) to execute the transaction. [Gas](basics-gas-txn-fee.md) is a way to pay for computation and storage. A gas unit is an abstract measurement of computation with no inherent real-world value.
- **Maximum gas amount**: The [maximum gas amount](/reference/glossary#maximum-gas-amount) is the maximum gas units the transaction is allowed to consume.
- **Gas currency code**: The currency code used to pay for gas.
- **Sequence number**: This is an unsigned integer that must be equal to the sender's account [sequence number](/reference/glossary#sequence-number) at the time of execution.
- **Expiration time**: A timestamp after which the transaction ceases to be valid (i.e., expires).

# Ledger state

The Aptos Blockchain's ledger state (or global [state](/reference/glossary#state)) comprises the state of all accounts in the blockchain. Each validator node in the blockchain must know the global state of the latest version of the blockchain's distributed database (versioned database) to execute any transaction.

## Versioned database

All of the data in the Aptos Blockchain is persisted in a single-versioned distributed database. A version number is an unsigned 64-bit integer that corresponds to the number of transactions the system has executed.

This versioned database allows validator nodes to:

- Execute a transaction against the ledger state at the latest version.
- Respond to client queries about ledger history at both current and previous versions.

## Transactions change state

![FIGURE 1.0 TRANSACTIONS CHANGE STATE](/img/docs/transactions.svg)
<small className="figure">FIGURE 1.0 TRANSACTIONS CHANGE STATE</small>

Figure 1.0 represents how executing transaction T<sub>N</sub> changes the state of the Aptos Blockchain from S<sub>N-1</sub> to S<sub>N</sub>.

In the figure:

| Name | Description |
| ---- | ----------- |
| Accounts **A** and **B** | Represent Alice's and Bob's accounts on the Aptos Blockchain |
| **S<sub>N-1</sub>** | Represents the (**N-1**)th state of the blockchain. In this state, Alice's account **A** has a balance of 110 Aptos Coins, and Bob's account **B** has a balance of 52 Aptos Coins. |
| **T<sub>N</sub>** | This is the **N**th transaction executed on the blockchain. In this example, it represents Alice sending 10 Aptos Coins to Bob. |
| **F** | It is a deterministic function. **F** always returns the same final state for a specific initial state and a specific transaction. If the current state of the blockchain is **S<sub>N-1</sub>**, and transaction **T<sub>N</sub>** is executed on state **S<sub>N-1</sub>**, the new state of the blockchain is always **S<sub>N</sub>**. The Aptos Blockchain uses the [Move language](https://aptos.github.io/move) to implement the deterministic execution function **F**. |
| **S<sub>N</sub>** | This is the **N**th state of the blockchain. When the transaction **T<sub>N</sub>** is applied to the blockchain, it generates the new state **S<sub>N</sub>** (an outcome of applying **F** to **S<sub>N-1</sub>** and **T<sub>N</sub>**). This causes Alice’s account balance to be reduced by 10 to 100 Aptos Coins and Bob’s account balance to be increased by 10 to 62 Aptos Coins. The new state **S<sub>N</sub>** shows these updated balances. |
