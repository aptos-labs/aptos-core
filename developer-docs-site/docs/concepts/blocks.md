---
title: "Blocks"
id: "blocks"
---

# Blocks

Aptos is a per-transaction versioned database. When transactions are executed, the resulting state of each transaction is stored separately and thus allows for more granular data access. This is different from other blockchains where only the resulting state of a block (a group of transactions) is stored.

Blocks are still a fundamental unit within Aptos. Transactions are batched and executed together in a block. In addition, the [proofs](../concepts/txns-states/#proofs) within storage are at the block-level granularity. The number of transactions within a block varies depending on network activity and a configurable maximum block size limit. As the blockchain becomes busier, blocks will likely contain more transactions.

## System transactions

Each Aptos block contains both user transactions and special system transactions to *mark* the beginning and end of the transaction batch. Specifically, there are two system transactions:
1. `BlockMetadataTransaction` - is inserted at the beginning of the block. A `BlockMetadata` transaction can also mark the end of an [epoch](#epoch) and trigger reward distribution to validators.
2. `StateCheckpointTransaction` - is appended at the end of the block and is used as a checkpoint milestone.

## Epoch

In Aptos, epochs represent a longer period of time in order to safely synchronize major changes such as validator set additions/removals. An epoch is a fixed duration of time, currently defined as two hours on mainnet. The number of blocks in an epoch depends on how many blocks can execute within this period of time. It is only at the start of a new epoch that major changes such as a validator joining the validator set don't immediately take effect among the validators.
