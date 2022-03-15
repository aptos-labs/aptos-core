---
title: "Proof"
slug: "basics-merkle-proof"
---

The Aptos Blockchain uses proofs as a way to verify the authenticity and correctness of blockchain data.

All data in the Aptos Blockchain is stored in a single-version distributed database. Each validator and FullNode's [storage](basics-validator-nodes.md#storage) is responsible for persisting agreed upon blocks of transactions and their execution results to the database. The blockchain is represented as an ever-growing [Merkle tree](/reference/glossary#merkle-trees), where each leaf appended to the tree represents a single transaction executed by the blockchain.

All operations executed by the blockchain and all account states can be verified cryptographically. These cryptographic proofs ensure that the validator nodes agree on the states. By supporting proofs, the client does not need to trust the entity from which it is receiving data. For example, if a client fetches the last _n_ transactions from an account, a proof can attest that no transactions were added, omitted or modified in the response. The client could also query for the state of an account, ask whether a specific transaction was processed, and so on.
