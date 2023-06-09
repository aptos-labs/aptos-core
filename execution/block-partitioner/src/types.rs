// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_types::transaction::analyzed_transaction::AnalyzedTransaction;
use std::collections::HashSet;

pub type ShardId = usize;
pub type TxnIndex = usize;

#[derive(Debug, Clone, Eq, Hash, PartialEq)]
pub struct CrossShardDependency {
    pub txn_index: TxnIndex,
    pub shard_id: ShardId,
}

impl CrossShardDependency {
    pub fn new(txn_index: TxnIndex, shard_id: ShardId) -> Self {
        Self {
            shard_id,
            txn_index,
        }
    }
}

#[derive(Default, Debug, Clone)]
/// Represents the dependencies of a transaction on other transactions across shards. Two types
/// of dependencies are supported:
/// 1. `required_txns`: The transaction depends on the execution of the transactions in the set. In this
/// case, the transaction can only be executed after the transactions in the set have been executed.
/// 2. `dependent_txns`: The transactions in the set depend on the execution of the transaction. In this
/// case, the transactions in the set can only be executed after the transaction has been executed.
pub struct CrossShardDependencies {
    required_txns: HashSet<CrossShardDependency>,
    dependent_txns: HashSet<CrossShardDependency>,
}

impl CrossShardDependencies {
    pub fn len(&self) -> usize {
        self.required_txns.len()
    }

    pub fn is_empty(&self) -> bool {
        self.required_txns.is_empty()
    }

    pub fn required_txns(&self) -> &HashSet<CrossShardDependency> {
        &self.required_txns
    }

    pub fn is_required_txn(&self, dep: CrossShardDependency) -> bool {
        self.required_txns.contains(&dep)
    }

    pub fn is_dependent_txn(&self, dep: CrossShardDependency) -> bool {
        self.dependent_txns.contains(&dep)
    }

    pub fn add_required_txn(&mut self, dep: CrossShardDependency) {
        self.required_txns.insert(dep);
    }

    pub fn add_dependent_txn(&mut self, dep: CrossShardDependency) {
        self.dependent_txns.insert(dep);
    }
}

#[derive(Debug, Clone)]
/// A contiguous chunk of transactions (along with their dependencies) in a block.
///
/// Each `SubBlock` represents a sequential section of transactions within a block.
/// The sub block includes the index of the first transaction relative to the block and a vector
/// of `TransactionWithDependencies` representing the transactions included in the chunk.
///
/// Illustration:
/// ```plaintext
///  Block (Split into 3 transactions chunks):
///  +----------------+------------------+------------------+
///  | Chunk 1        | Chunk 2          | Chunk 3          |
///  +----------------+------------------+------------------+
///  | Transaction 1  | Transaction 4    | Transaction 7    |
///  | Transaction 2  | Transaction 5    | Transaction 8    |
///  | Transaction 3  | Transaction 6    | Transaction 9    |
///  +----------------+------------------+------------------+
/// ```
pub struct SubBlock {
    // This is the index of first transaction relative to the block.
    pub start_index: TxnIndex,
    pub transactions: Vec<TransactionWithDependencies>,
}

impl SubBlock {
    pub fn new(start_index: TxnIndex, transactions: Vec<TransactionWithDependencies>) -> Self {
        Self {
            start_index,
            transactions,
        }
    }

    pub fn num_txns(&self) -> usize {
        self.transactions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.transactions.is_empty()
    }

    pub fn end_index(&self) -> TxnIndex {
        self.start_index + self.num_txns()
    }

    pub fn transactions_with_deps(&self) -> &Vec<TransactionWithDependencies> {
        &self.transactions
    }

    pub fn add_dependent_txn(
        &mut self,
        source_index: TxnIndex,
        txn_idx_with_shard_id: CrossShardDependency,
    ) {
        let source_txn = self
            .transactions
            .get_mut(source_index - self.start_index)
            .unwrap();
        source_txn.add_dependent_txn(txn_idx_with_shard_id);
    }

    pub fn iter(&self) -> impl Iterator<Item = &TransactionWithDependencies> {
        self.transactions.iter()
    }
}

// A set of sub blocks assigned to a shard.
#[derive(Default)]
pub struct SubBlocksForShard {
    pub shard_id: ShardId,
    pub sub_blocks: Vec<SubBlock>,
}

impl SubBlocksForShard {
    pub fn empty(shard_id: ShardId) -> Self {
        Self {
            shard_id,
            sub_blocks: Vec::new(),
        }
    }

    pub fn add_sub_block(&mut self, sub_block: SubBlock) {
        self.sub_blocks.push(sub_block);
    }

    pub fn num_txns(&self) -> usize {
        self.sub_blocks
            .iter()
            .map(|sub_block| sub_block.num_txns())
            .sum()
    }

    pub fn num_sub_blocks(&self) -> usize {
        self.sub_blocks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.sub_blocks.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &TransactionWithDependencies> {
        self.sub_blocks
            .iter()
            .flat_map(|sub_block| sub_block.iter())
    }

    pub fn sub_block_iter(&self) -> impl Iterator<Item = &SubBlock> {
        self.sub_blocks.iter()
    }

    pub fn get_sub_block(&self, round: usize) -> Option<&SubBlock> {
        self.sub_blocks.get(round)
    }

    pub fn get_sub_block_mut(&mut self, round: usize) -> Option<&mut SubBlock> {
        self.sub_blocks.get_mut(round)
    }
}

#[derive(Debug, Clone)]
pub struct TransactionWithDependencies {
    pub txn: AnalyzedTransaction,
    pub cross_shard_dependencies: CrossShardDependencies,
}

impl TransactionWithDependencies {
    pub fn new(txn: AnalyzedTransaction, cross_shard_dependencies: CrossShardDependencies) -> Self {
        Self {
            txn,
            cross_shard_dependencies,
        }
    }

    #[cfg(test)]
    pub fn txn(&self) -> &AnalyzedTransaction {
        &self.txn
    }

    #[cfg(test)]
    pub fn cross_shard_dependencies(&self) -> &CrossShardDependencies {
        &self.cross_shard_dependencies
    }

    pub fn add_dependent_txn(&mut self, txn_idx_with_shard_id: CrossShardDependency) {
        self.cross_shard_dependencies
            .add_dependent_txn(txn_idx_with_shard_id);
    }
}
