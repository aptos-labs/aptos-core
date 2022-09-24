---
title: "Proof"
slug: "basics-merkle-proof"
---

# Proof

The Aptos blockchain uses proofs to verify the authenticity and correctness of the blockchain data.

All the data in the Aptos blockchain is stored in a single-version distributed database. Each validator and fullnode's [storage](basics-validator-nodes.md#storage) is responsible for persisting the agreed upon blocks of transactions and their execution results to the database. 

The blockchain is represented as an ever-growing [Merkle tree](/reference/glossary#merkle-trees), where each leaf appended to the tree represents a single transaction executed by the blockchain.

All the operations executed by the blockchain and all the account states can be verified cryptographically. These cryptographic proofs ensure that:
- The validator nodes agree on the states. 
- The client does not need to trust the entity from which it is receiving data. For example, if a client fetches the last **n** transactions from an account, a proof can attest that no transactions were added, omitted or modified in the response. The client may also query for the state of an account, ask whether a specific transaction was processed, and so on.
