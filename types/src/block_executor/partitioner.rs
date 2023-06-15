// Copyright © Aptos Foundation

use crate::transaction::{analyzed_transaction::StorageLocation, Transaction};
use aptos_crypto::HashValue;
use std::collections::HashMap;

pub type ShardId = usize;
pub type TxnIndex = usize;

#[derive(Debug, Clone, Eq, Hash, PartialEq)]
pub struct TxnIdxWithShardId {
    pub txn_index: TxnIndex,
    pub shard_id: ShardId,
}

impl TxnIdxWithShardId {
    pub fn new(txn_index: TxnIndex, shard_id: ShardId) -> Self {
        Self {
            shard_id,
            txn_index,
        }
    }
}

#[derive(Debug, Default, Clone)]
/// Denotes a set of cross shard edges, which contains the set (required or dependent) transaction
/// indices and the relevant storage locations that are conflicting.
pub struct CrossShardEdges {
    edges: HashMap<TxnIdxWithShardId, Vec<StorageLocation>>,
}

impl CrossShardEdges {
    pub fn new(txn_idx: TxnIdxWithShardId, storage_locations: Vec<StorageLocation>) -> Self {
        let mut edges = HashMap::new();
        edges.insert(txn_idx, storage_locations);
        Self { edges }
    }

    pub fn add_edge(
        &mut self,
        txn_idx: TxnIdxWithShardId,
        storage_locations: Vec<StorageLocation>,
    ) {
        self.edges
            .entry(txn_idx)
            .or_insert_with(Vec::new)
            .extend(storage_locations.into_iter());
    }

    pub fn iter(&self) -> impl Iterator<Item = (&TxnIdxWithShardId, &Vec<StorageLocation>)> {
        self.edges.iter()
    }

    pub fn len(&self) -> usize {
        self.edges.len()
    }

    pub fn contains_idx(&self, txn_idx: &TxnIdxWithShardId) -> bool {
        self.edges.contains_key(txn_idx)
    }

    pub fn is_empty(&self) -> bool {
        self.edges.is_empty()
    }
}

impl IntoIterator for CrossShardEdges {
    type IntoIter = std::collections::hash_map::IntoIter<TxnIdxWithShardId, Vec<StorageLocation>>;
    type Item = (TxnIdxWithShardId, Vec<StorageLocation>);

    fn into_iter(self) -> Self::IntoIter {
        self.edges.into_iter()
    }
}

#[derive(Default, Debug, Clone)]
/// Represents the dependencies of a transaction on other transactions across shards. Two types
/// of dependencies are supported:
/// 1. `required_edges`: The transaction depends on the execution of the transactions in the set. In this
/// case, the transaction can only be executed after the transactions in the set have been executed.
/// 2. `dependent_edges`: The transactions in the set depend on the execution of the transaction. In this
/// case, the transactions in the set can only be executed after the transaction has been executed.
/// Dependent edge is a reverse of required edge, for example if txn 20 in shard 2 requires txn 10 in shard 1,
/// then txn 10 in shard 1 will have a dependent edge to txn 20 in shard 2.
pub struct CrossShardDependencies {
    required_edges: CrossShardEdges,
    dependent_edges: CrossShardEdges,
}

impl CrossShardDependencies {
    pub fn num_required_edges(&self) -> usize {
        self.required_edges.len()
    }

    pub fn required_edges_iter(
        &self,
    ) -> impl Iterator<Item = (&TxnIdxWithShardId, &Vec<StorageLocation>)> {
        self.required_edges.iter()
    }

    pub fn has_required_txn(&self, txn_idx: TxnIdxWithShardId) -> bool {
        self.required_edges.contains_idx(&txn_idx)
    }

    pub fn get_required_edge_for(
        &self,
        txn_idx: TxnIdxWithShardId,
    ) -> Option<&Vec<StorageLocation>> {
        self.required_edges.edges.get(&txn_idx)
    }

    pub fn get_dependent_edge_for(
        &self,
        txn_idx: TxnIdxWithShardId,
    ) -> Option<&Vec<StorageLocation>> {
        self.dependent_edges.edges.get(&txn_idx)
    }

    pub fn has_dependent_txn(&self, txn_ids: TxnIdxWithShardId) -> bool {
        self.dependent_edges.contains_idx(&txn_ids)
    }

    pub fn add_required_edge(
        &mut self,
        txn_idx: TxnIdxWithShardId,
        storage_location: StorageLocation,
    ) {
        self.required_edges
            .add_edge(txn_idx, vec![storage_location]);
    }

    pub fn add_dependent_edge(
        &mut self,
        txn_idx: TxnIdxWithShardId,
        storage_locations: Vec<StorageLocation>,
    ) {
        self.dependent_edges.add_edge(txn_idx, storage_locations);
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
pub struct SubBlock<T> {
    // This is the index of first transaction relative to the block.
    pub start_index: TxnIndex,
    pub transactions: Vec<TransactionWithDependencies<T>>,
}

impl<T> SubBlock<T> {
    pub fn new(start_index: TxnIndex, transactions: Vec<TransactionWithDependencies<T>>) -> Self {
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

    pub fn transactions_with_deps(&self) -> &Vec<TransactionWithDependencies<T>> {
        &self.transactions
    }

    pub fn into_transactions_with_deps(self) -> Vec<TransactionWithDependencies<T>> {
        self.transactions
    }

    pub fn add_dependent_edge(
        &mut self,
        source_index: TxnIndex,
        txn_idx: TxnIdxWithShardId,
        storage_locations: Vec<StorageLocation>,
    ) {
        let source_txn = self
            .transactions
            .get_mut(source_index - self.start_index)
            .unwrap();
        source_txn.add_dependent_edge(txn_idx, storage_locations);
    }

    pub fn iter(&self) -> impl Iterator<Item = &TransactionWithDependencies<T>> {
        self.transactions.iter()
    }
}

impl<T> IntoIterator for SubBlock<T> {
    type IntoIter = std::vec::IntoIter<TransactionWithDependencies<T>>;
    type Item = TransactionWithDependencies<T>;

    fn into_iter(self) -> Self::IntoIter {
        self.transactions.into_iter()
    }
}

// A set of sub blocks assigned to a shard.
#[derive(Default)]
pub struct SubBlocksForShard<T> {
    pub shard_id: ShardId,
    pub sub_blocks: Vec<SubBlock<T>>,
}

impl<T> SubBlocksForShard<T> {
    pub fn empty(shard_id: ShardId) -> Self {
        Self {
            shard_id,
            sub_blocks: Vec::new(),
        }
    }

    pub fn add_sub_block(&mut self, sub_block: SubBlock<T>) {
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

    pub fn iter(&self) -> impl Iterator<Item = &TransactionWithDependencies<T>> {
        self.sub_blocks
            .iter()
            .flat_map(|sub_block| sub_block.iter())
    }

    pub fn sub_block_iter(&self) -> impl Iterator<Item = &SubBlock<T>> {
        self.sub_blocks.iter()
    }

    pub fn get_sub_block(&self, round: usize) -> Option<&SubBlock<T>> {
        self.sub_blocks.get(round)
    }

    pub fn get_sub_block_mut(&mut self, round: usize) -> Option<&mut SubBlock<T>> {
        self.sub_blocks.get_mut(round)
    }
}

#[derive(Debug, Clone)]
pub struct TransactionWithDependencies<T> {
    pub txn: T,
    pub cross_shard_dependencies: CrossShardDependencies,
}

impl<T> TransactionWithDependencies<T> {
    pub fn new(txn: T, cross_shard_dependencies: CrossShardDependencies) -> Self {
        Self {
            txn,
            cross_shard_dependencies,
        }
    }

    pub fn txn(&self) -> &T {
        &self.txn
    }

    pub fn cross_shard_dependencies(&self) -> &CrossShardDependencies {
        &self.cross_shard_dependencies
    }

    pub fn add_dependent_edge(
        &mut self,
        txn_idx: TxnIdxWithShardId,
        storage_locations: Vec<StorageLocation>,
    ) {
        self.cross_shard_dependencies
            .add_dependent_edge(txn_idx, storage_locations);
    }
}

pub struct ExecutableBlock<T> {
    pub block_id: HashValue,
    pub transactions: ExecutableTransactions<T>,
}

impl<T> ExecutableBlock<T> {
    pub fn new(block_id: HashValue, transactions: ExecutableTransactions<T>) -> Self {
        Self {
            block_id,
            transactions,
        }
    }
}

impl<T> From<(HashValue, Vec<T>)> for ExecutableBlock<T> {
    fn from((block_id, transactions): (HashValue, Vec<T>)) -> Self {
        Self::new(block_id, ExecutableTransactions::Unsharded(transactions))
    }
}

pub enum ExecutableTransactions<T> {
    Unsharded(Vec<T>),
    Sharded(Vec<SubBlock<T>>),
}

impl<T> ExecutableTransactions<T> {
    pub fn num_transactions(&self) -> usize {
        match self {
            ExecutableTransactions::Unsharded(transactions) => transactions.len(),
            ExecutableTransactions::Sharded(sub_blocks) => sub_blocks
                .iter()
                .map(|sub_block| sub_block.num_txns())
                .sum(),
        }
    }

    pub fn get_unsharded_transactions(&self) -> Option<&Vec<T>> {
        match self {
            ExecutableTransactions::Unsharded(transactions) => Some(transactions),
            ExecutableTransactions::Sharded(_) => None,
        }
    }
}

impl From<Vec<Transaction>> for ExecutableTransactions<Transaction> {
    fn from(txns: Vec<Transaction>) -> Self {
        Self::Unsharded(txns)
    }
}
