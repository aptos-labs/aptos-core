// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::transaction::{
    analyzed_transaction::{AnalyzedTransaction, StorageLocation},
    signature_verified_transaction::SignatureVerifiedTransaction,
    AuxiliaryInfo, Transaction,
};
use aptos_crypto::HashValue;
use serde::{Deserialize, Serialize};
use std::{
    cmp::Ordering,
    collections::{HashMap, HashSet},
};

pub type ShardId = usize;
pub type TxnIndex = usize;
pub type RoundId = usize;

pub static MAX_ALLOWED_PARTITIONING_ROUNDS: usize = 8;
pub static GLOBAL_ROUND_ID: usize = MAX_ALLOWED_PARTITIONING_ROUNDS + 1;
pub static GLOBAL_SHARD_ID: usize = usize::MAX;

#[derive(Debug, Clone, Copy, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub struct ShardedTxnIndex {
    pub txn_index: TxnIndex,
    pub shard_id: ShardId,
    pub round_id: RoundId,
}

impl PartialOrd for ShardedTxnIndex {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        (self.round_id, self.shard_id, self.txn_index).partial_cmp(&(
            other.round_id,
            other.shard_id,
            other.txn_index,
        ))
    }
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
    pub edges: HashMap<ShardedTxnIndex, Vec<StorageLocation>>,
}

impl PartialEq for CrossShardEdges {
    fn eq(&self, other: &Self) -> bool {
        let my_key_set = self.edges.keys().copied().collect::<HashSet<_>>();
        let other_key_set = other.edges.keys().copied().collect::<HashSet<_>>();
        if my_key_set != other_key_set {
            return false;
        }
        for key in my_key_set {
            let my_value = self
                .edges
                .get(&key)
                .unwrap()
                .clone()
                .into_iter()
                .collect::<HashSet<_>>();
            let other_value = other
                .edges
                .get(&key)
                .unwrap()
                .clone()
                .into_iter()
                .collect::<HashSet<_>>();
            if my_value != other_value {
                return false;
            }
        }
        true
    }
}

impl Eq for CrossShardEdges {}

impl CrossShardEdges {
    pub fn new(txn_idx: ShardedTxnIndex, storage_locations: Vec<StorageLocation>) -> Self {
        let mut edges = HashMap::new();
        edges.insert(txn_idx, storage_locations);
        Self { edges }
    }

    pub fn add_edge(&mut self, txn_idx: ShardedTxnIndex, storage_locations: Vec<StorageLocation>) {
        self.edges
            .entry(txn_idx)
            .or_default()
            .extend(storage_locations);
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

#[derive(Default, Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
/// Represents the dependencies of a transaction on other transactions across shards. Two types
/// of dependencies are supported:
/// 1. `required_edges`: The transaction depends on the execution of the transactions in the set. In this
/// case, the transaction can only be executed after the transactions in the set have been executed.
/// 2. `dependent_edges`: The transactions in the set depend on the execution of the transaction. In this
/// case, the transactions in the set can only be executed after the transaction has been executed.
/// Dependent edge is a reverse of required edge, for example if txn 20 in shard 2 requires txn 10 in shard 1,
/// then txn 10 in shard 1 will have a dependent edge to txn 20 in shard 2.
pub struct CrossShardDependencies {
    pub required_edges: CrossShardEdges,
    pub dependent_edges: CrossShardEdges,
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
#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
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
#[derive(Default, Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
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

    pub fn remove_last_sub_block(&mut self) -> Option<SubBlock<T>> {
        self.sub_blocks.pop()
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

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
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

pub struct ExecutableBlock {
    pub block_id: HashValue,
    pub transactions: ExecutableTransactions,
    pub auxiliary_info: Vec<AuxiliaryInfo>,
}

impl ExecutableBlock {
    pub fn new(
        block_id: HashValue,
        transactions: ExecutableTransactions,
        auxiliary_info: Vec<AuxiliaryInfo>,
    ) -> Self {
        match &transactions {
            ExecutableTransactions::Unsharded(txns) => {
                assert!(txns.len() == auxiliary_info.len());
            },
            ExecutableTransactions::Sharded(_) => {
                // Not supporting auxiliary info here because the sharded executor is only for
                // benchmark purpose right now.
                // TODO: Revisit when we need it.
                assert!(auxiliary_info.is_empty());
            },
        }
        Self {
            block_id,
            transactions,
            auxiliary_info,
        }
    }
}

impl From<(HashValue, Vec<SignatureVerifiedTransaction>)> for ExecutableBlock {
    fn from((block_id, transactions): (HashValue, Vec<SignatureVerifiedTransaction>)) -> Self {
        let auxiliary_info = transactions
            .iter()
            .map(|_| AuxiliaryInfo::new_empty())
            .collect();
        Self::new(
            block_id,
            ExecutableTransactions::Unsharded(transactions),
            auxiliary_info,
        )
    }
}

impl
    From<(
        HashValue,
        Vec<SignatureVerifiedTransaction>,
        Vec<AuxiliaryInfo>,
    )> for ExecutableBlock
{
    fn from(
        (block_id, transactions, auxiliary_info): (
            HashValue,
            Vec<SignatureVerifiedTransaction>,
            Vec<AuxiliaryInfo>,
        ),
    ) -> Self {
        Self::new(
            block_id,
            ExecutableTransactions::Unsharded(transactions),
            auxiliary_info,
        )
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PartitionedTransactions {
    pub sharded_txns: Vec<SubBlocksForShard<AnalyzedTransaction>>,
    pub global_txns: Vec<TransactionWithDependencies<AnalyzedTransaction>>,
}

impl PartitionedTransactions {
    pub fn new(
        sharded_txns: Vec<SubBlocksForShard<AnalyzedTransaction>>,
        global_txns: Vec<TransactionWithDependencies<AnalyzedTransaction>>,
    ) -> Self {
        Self {
            sharded_txns,
            global_txns,
        }
    }

    pub fn empty() -> Self {
        Self {
            sharded_txns: Vec::new(),
            global_txns: Vec::new(),
        }
    }

    pub fn into(
        self,
    ) -> (
        Vec<SubBlocksForShard<AnalyzedTransaction>>,
        Vec<TransactionWithDependencies<AnalyzedTransaction>>,
    ) {
        (self.sharded_txns, self.global_txns)
    }

    pub fn num_shards(&self) -> usize {
        self.sharded_txns.len()
    }

    pub fn sharded_txns(&self) -> &[SubBlocksForShard<AnalyzedTransaction>] {
        &self.sharded_txns
    }

    pub fn num_sharded_txns(&self) -> usize {
        self.sharded_txns
            .iter()
            .map(|sub_blocks| sub_blocks.num_txns())
            .sum::<usize>()
    }

    pub fn num_txns(&self) -> usize {
        self.num_sharded_txns() + self.global_txns.len()
    }

    pub fn add_checkpoint_txn(&mut self, last_txn: SignatureVerifiedTransaction) {
        assert!(matches!(
            last_txn.expect_valid(),
            Transaction::StateCheckpoint(_)
        ));
        let txn_with_deps =
            TransactionWithDependencies::new(last_txn.into(), CrossShardDependencies::default());
        if !self.global_txns.is_empty() {
            self.global_txns.push(txn_with_deps);
        } else {
            self.sharded_txns
                .last_mut()
                .unwrap()
                .sub_blocks
                .last_mut()
                .unwrap()
                .transactions
                .push(txn_with_deps)
        }
    }

    pub fn flatten(transactions: PartitionedTransactions) -> Vec<AnalyzedTransaction> {
        SubBlocksForShard::flatten(transactions.sharded_txns)
            .into_iter()
            .chain(
                transactions
                    .global_txns
                    .into_iter()
                    .map(|txn| txn.into_txn()),
            )
            .collect()
    }
}

// Represents the transactions in a block that are ready to be executed.
#[derive(Clone)]
pub enum ExecutableTransactions {
    Unsharded(Vec<SignatureVerifiedTransaction>),
    Sharded(PartitionedTransactions),
}

impl ExecutableTransactions {
    pub fn num_transactions(&self) -> usize {
        match self {
            ExecutableTransactions::Unsharded(transactions) => transactions.len(),
            ExecutableTransactions::Sharded(partitioned_txns) => partitioned_txns.num_txns(),
        }
    }

    pub fn txns(&self) -> Vec<&SignatureVerifiedTransaction> {
        match self {
            ExecutableTransactions::Unsharded(txns) => txns.iter().collect(),
            ExecutableTransactions::Sharded(_partitioned) => unimplemented!(""),
        }
    }

    pub fn into_txns(self) -> Vec<SignatureVerifiedTransaction> {
        match self {
            ExecutableTransactions::Unsharded(txns) => txns,
            ExecutableTransactions::Sharded(partitioned) => {
                PartitionedTransactions::flatten(partitioned)
                    .into_iter()
                    .map(|t| t.into_txn())
                    .collect()
            },
        }
    }
}

impl From<Vec<SignatureVerifiedTransaction>> for ExecutableTransactions {
    fn from(txns: Vec<SignatureVerifiedTransaction>) -> Self {
        Self::Unsharded(txns)
    }
}
