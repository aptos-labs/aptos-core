// Copyright © Aptos Foundation

use crate::transaction::{analyzed_transaction::StorageLocation, Transaction};
use aptos_crypto::HashValue;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub type ShardId = usize;
pub type TxnIndex = usize;
pub type RoundId = usize;

#[derive(Debug, Clone, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub struct ShardedTxnIndex {
    pub txn_index: TxnIndex,
    pub shard_id: ShardId,
    pub round_id: RoundId,
}

impl ShardedTxnIndex {
    pub fn new(txn_index: TxnIndex, shard_id: ShardId, round_id: RoundId) -> Self {
        Self {
            shard_id,
            txn_index,
            round_id,
        }
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
/// Denotes a set of cross shard edges, which contains the set (required or dependent) transaction
/// indices and the relevant storage locations that are conflicting.
pub struct CrossShardEdges {
    edges: HashMap<ShardedTxnIndex, Vec<StorageLocation>>,
}

impl CrossShardEdges {
    pub fn new(txn_idx: ShardedTxnIndex, storage_locations: Vec<StorageLocation>) -> Self {
        let mut edges = HashMap::new();
        edges.insert(txn_idx, storage_locations);
        Self { edges }
    }

    pub fn add_edge(&mut self, txn_idx: ShardedTxnIndex, storage_locations: Vec<StorageLocation>) {
        self.edges
            .entry(txn_idx)
            .or_insert_with(Vec::new)
            .extend(storage_locations.into_iter());
    }

    pub fn iter(&self) -> impl Iterator<Item = (&ShardedTxnIndex, &Vec<StorageLocation>)> {
        self.edges.iter()
    }

    pub fn len(&self) -> usize {
        self.edges.len()
    }

    pub fn contains_idx(&self, txn_idx: &ShardedTxnIndex) -> bool {
        self.edges.contains_key(txn_idx)
    }

    pub fn is_empty(&self) -> bool {
        self.edges.is_empty()
    }
}

impl IntoIterator for CrossShardEdges {
    type IntoIter = std::collections::hash_map::IntoIter<ShardedTxnIndex, Vec<StorageLocation>>;
    type Item = (ShardedTxnIndex, Vec<StorageLocation>);

    fn into_iter(self) -> Self::IntoIter {
        self.edges.into_iter()
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
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
    pub fn required_edges(&self) -> &CrossShardEdges {
        &self.required_edges
    }

    pub fn dependent_edges(&self) -> &CrossShardEdges {
        &self.dependent_edges
    }

    pub fn num_required_edges(&self) -> usize {
        self.required_edges.len()
    }

    pub fn required_edges_iter(
        &self,
    ) -> impl Iterator<Item = (&ShardedTxnIndex, &Vec<StorageLocation>)> {
        self.required_edges.iter()
    }

    pub fn has_required_txn(&self, txn_idx: ShardedTxnIndex) -> bool {
        self.required_edges.contains_idx(&txn_idx)
    }

    pub fn get_required_edge_for(&self, txn_idx: ShardedTxnIndex) -> Option<&Vec<StorageLocation>> {
        self.required_edges.edges.get(&txn_idx)
    }

    pub fn get_dependent_edge_for(
        &self,
        txn_idx: ShardedTxnIndex,
    ) -> Option<&Vec<StorageLocation>> {
        self.dependent_edges.edges.get(&txn_idx)
    }

    pub fn has_dependent_txn(&self, txn_ids: ShardedTxnIndex) -> bool {
        self.dependent_edges.contains_idx(&txn_ids)
    }

    pub fn add_required_edge(
        &mut self,
        txn_idx: ShardedTxnIndex,
        storage_location: StorageLocation,
    ) {
        self.required_edges
            .add_edge(txn_idx, vec![storage_location]);
    }

    pub fn add_dependent_edge(
        &mut self,
        txn_idx: ShardedTxnIndex,
        storage_locations: Vec<StorageLocation>,
    ) {
        self.dependent_edges.add_edge(txn_idx, storage_locations);
    }
}

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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubBlock<T> {
    // This is the index of first transaction relative to the block.
    pub start_index: TxnIndex,
    pub transactions: Vec<TransactionWithDependencies<T>>,
}

impl<T: Clone> SubBlock<T> {
    pub fn new(start_index: TxnIndex, transactions: Vec<TransactionWithDependencies<T>>) -> Self {
        Self {
            start_index,
            transactions,
        }
    }

    pub fn empty() -> Self {
        Self {
            start_index: 0,
            transactions: vec![],
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

    pub fn txn_with_index_iter(
        &self,
    ) -> impl Iterator<Item = (TxnIndex, &TransactionWithDependencies<T>)> {
        self.transactions
            .iter()
            .enumerate()
            .map(move |(i, txn)| (self.start_index + i, txn))
    }

    pub fn into_transactions_with_deps(self) -> Vec<TransactionWithDependencies<T>> {
        self.transactions
    }

    pub fn into_txns(self) -> Vec<T> {
        self.transactions
            .into_iter()
            .map(|txn_with_deps| txn_with_deps.into_txn())
            .collect()
    }

    pub fn add_dependent_edge(
        &mut self,
        source_index: TxnIndex,
        txn_idx: ShardedTxnIndex,
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

impl<T: Clone> IntoIterator for SubBlock<T> {
    type IntoIter = std::vec::IntoIter<TransactionWithDependencies<T>>;
    type Item = TransactionWithDependencies<T>;

    fn into_iter(self) -> Self::IntoIter {
        self.transactions.into_iter()
    }
}

// A set of sub blocks assigned to a shard.
#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct SubBlocksForShard<T> {
    pub shard_id: ShardId,
    pub sub_blocks: Vec<SubBlock<T>>,
}

impl<T: Clone> SubBlocksForShard<T> {
    pub fn new(shard_id: ShardId, sub_blocks: Vec<SubBlock<T>>) -> Self {
        Self {
            shard_id,
            sub_blocks,
        }
    }

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

    pub fn into_sub_blocks(self) -> Vec<SubBlock<T>> {
        self.sub_blocks
    }

    pub fn into_txns(self) -> Vec<T> {
        self.sub_blocks
            .into_iter()
            .flat_map(|sub_block| sub_block.into_txns())
            .collect()
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

    // Flattens a vector of `SubBlocksForShard` into a vector of transactions in the order they
    // appear in the block.
    pub fn flatten(block: Vec<SubBlocksForShard<T>>) -> Vec<T> {
        let num_shards = block.len();
        let mut flattened_txns = Vec::new();
        let num_rounds = block[0].num_sub_blocks();
        let mut ordered_blocks = vec![SubBlock::empty(); num_shards * num_rounds];
        for (shard_id, sub_blocks) in block.into_iter().enumerate() {
            for (round, sub_block) in sub_blocks.into_sub_blocks().into_iter().enumerate() {
                ordered_blocks[round * num_shards + shard_id] = sub_block;
            }
        }

        for sub_block in ordered_blocks.into_iter() {
            flattened_txns.extend(sub_block.into_txns());
        }

        flattened_txns
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionWithDependencies<T> {
    pub txn: T,
    pub cross_shard_dependencies: CrossShardDependencies,
}

impl<T: Clone> TransactionWithDependencies<T> {
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

    pub fn into_txn(self) -> T {
        self.txn
    }

    pub fn add_dependent_edge(
        &mut self,
        txn_idx: ShardedTxnIndex,
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

impl<T: Clone> ExecutableBlock<T> {
    pub fn new(block_id: HashValue, transactions: ExecutableTransactions<T>) -> Self {
        Self {
            block_id,
            transactions,
        }
    }
}

impl<T: Clone> From<(HashValue, Vec<T>)> for ExecutableBlock<T> {
    fn from((block_id, transactions): (HashValue, Vec<T>)) -> Self {
        Self::new(block_id, ExecutableTransactions::Unsharded(transactions))
    }
}

// Represents the transactions in a block that are ready to be executed.
pub enum ExecutableTransactions<T> {
    Unsharded(Vec<T>),
    Sharded(Vec<SubBlocksForShard<T>>),
}

impl<T: Clone> ExecutableTransactions<T> {
    pub fn num_transactions(&self) -> usize {
        match self {
            ExecutableTransactions::Unsharded(transactions) => transactions.len(),
            ExecutableTransactions::Sharded(sub_blocks) => sub_blocks
                .iter()
                .map(|sub_block| sub_block.num_txns())
                .sum(),
        }
    }
}

impl From<Vec<Transaction>> for ExecutableTransactions<Transaction> {
    fn from(txns: Vec<Transaction>) -> Self {
        Self::Unsharded(txns)
    }
}

// Represents the transactions that are executed on a particular block executor shard. Unsharded
// transactions represents the entire block. Sharded transactions represents the transactions
// that are assigned to this shard.
pub enum BlockExecutorTransactions<T> {
    Unsharded(Vec<T>),
    Sharded(SubBlocksForShard<T>),
}

impl<T: Clone> BlockExecutorTransactions<T> {
    pub fn num_txns(&self) -> usize {
        match self {
            BlockExecutorTransactions::Unsharded(transactions) => transactions.len(),
            BlockExecutorTransactions::Sharded(sub_blocks) => sub_blocks.num_txns(),
        }
    }

    pub fn get_unsharded_transactions(&self) -> Option<&Vec<T>> {
        match self {
            BlockExecutorTransactions::Unsharded(transactions) => Some(transactions),
            BlockExecutorTransactions::Sharded(_) => None,
        }
    }

    pub fn into_txns(self) -> Vec<T> {
        match self {
            BlockExecutorTransactions::Unsharded(transactions) => transactions,
            BlockExecutorTransactions::Sharded(sub_blocks) => sub_blocks.into_txns(),
        }
    }
}
