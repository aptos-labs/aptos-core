// Copyright © Aptos Foundation

mod graph_partitioner;
mod fennel;
mod graph;
mod batched_stream;

use std::collections::HashMap;
use std::hash::Hash;
use std::marker::PhantomData;
use nonmax::NonMaxU32;
use aptos_transaction_orderer::common::PTransaction;
// use crate::TxnInfo::{Partitioned, Pending};

// /// Indicates the position of the transaction in the serialization order of the block.
// pub type SerializationIdx = u32;
//
// pub type ShardId = NonMaxU32;
//
// trait FromUsize {
//     fn from_usize(val: usize) -> Option<Self> where Self: Sized;
// }
//
// trait AsUsize {
//     fn as_usize(&self) -> usize;
// }
//
// impl AsUsize for NonMaxU32 {
//     fn as_usize(&self) -> usize {
//         self.get() as usize
//     }
// }
//
// type AffinityScore = f64;
//
// type CommunityId = SerializationIdx;
//
// type CommunityDepth = usize;
//
// const CLUSTERING_DEPTH: usize = 3;
//
// struct Communities([CommunityId; CLUSTERING_DEPTH]);
//
// enum TxnInfo<T: PTransaction> {
//     Partitioned(PartitionedTxnInfo<T>),
//     Pending(PendingTxnInfo<T>),
// }
//
// impl<T: PTransaction> TxnInfo<T>
// where
//     T::Key: Clone,
// {
//     fn take_for_partitioning(&mut self) -> Option<PendingTxnInfo<T>> {
//         let Pending(txn_info) = self else { return None; };
//         let mut tmp = Partitioned(PartitionedTxnInfo {
//             write_set: txn_info.transaction.write_set().cloned().collect(),
//             _phantom: PhantomData,
//         });
//         std::mem::swap(self, &mut tmp);
//         let Pending(txn_info) = tmp else { unreachable!() };
//         Some(txn_info)
//     }
// }
//
// struct PartitionedTxnInfo<T: PTransaction> {
//     write_set: Vec<T::Key>,
//     cluster_ids: Communities,
//
//     _phantom: PhantomData<T>
// }
//
// struct PendingTxnInfo<T: PTransaction> {
//     transaction: T,
//     communities: Communities,
//
//     _phantom: PhantomData<T>,
// }
//
// impl<T: PTransaction> PendingTxnInfo<T> {
//     fn new(transaction: T) -> Self {
//         // Self {
//         //     transaction,
//         //     shard_id: None,
//         //     dependants: Vec::new(),
//         //     count_dependencies_by_shard: Vec::new(),
//         //     _phantom: PhantomData,
//         // }
//         todo!()
//     }
// }
//
// struct ShardInfo {
//     /// Counts how many transactions from different communities have been assigned to this shard.
//     community_stats: HashMap<(CommunityId, CommunityDepth), usize>,
//
//     /// Tracks the number of transactions assigned to this shard that have not yet been committed.
//     load: usize,
// }
//
// impl ShardInfo {
//     fn compute_community_score(&self, communities: &Communities) -> AffinityScore {
//         let mut score = 0.;
//         for (community_depth, &community_id) in communities.0.iter().enumerate() {
//             let count = self.community_stats.get(&(community_id, community_depth)).unwrap_or(&0);
//             score += *count as AffinityScore / community_depth as AffinityScore;
//         }
//         score
//     }
//
//     fn compute_load_score(&self) -> AffinityScore {
//         // The formula is inspired by the Fennel paper:
//         // https://dl.acm.org/doi/pdf/10.1145/2556195.2556213
//         -(self.load as AffinityScore)
//     }
// }
//
// struct WriteInfo {
//     shard_id: ShardId,
//     txn_idx: SerializationIdx,
// }
//
// struct LocationInfo {
//     /// Indicates the shard and the transaction that last wrote to this location
//     /// (in the serialization order).
//     last_write: WriteInfo,
//
//     /// Indicates whether the last considered transaction in the serialization order
//     /// that has a write to this location has been committed.
//     write_complete: bool,
// }
//
// impl LocationInfo {
//     fn written_by(shard_id: ShardId, serialization_idx: SerializationIdx) -> Self {
//         assert!(shard_id.get() <= 63);
//         // Self {
//         //     last_write_txn: serialization_idx,
//         //     write_complete: false,
//         //     last_version_available: (1 << shard_id),
//         // }
//         todo!()
//     }
//
//     fn compute_dependency_score(&self, idx: SerializationIdx, n_shards: usize) -> AffinityScore {
//         let dist = (idx - self.last_write.txn_idx) as AffinityScore;
//         let normalized_dist = dist / (n_shards as AffinityScore);
//         let mut score = 10. / (1. + normalized_dist);
//
//         // If the last write to this location has been committed,
//         // this dependency is much less important.
//         if self.write_complete {
//             score /= 5.;
//         }
//
//         score
//     }
// }
//
// pub struct StreamingPartitioner<T: PTransaction> {
//     transactions: Vec<TxnInfo<T>>,
//     partitioned_idx: usize,
//
//     partitioned_prefix_location_info: HashMap<T::Key, LocationInfo>,
//
//     /// Indicates the last transaction in the serialization order that has a write to this location
//     /// among all transactions added to the partitioner.
//     /// Used for determining transaction dependencies when adding new transactions
//     /// for consideration.
//     last_write_txn: HashMap<T::Key, SerializationIdx>,
//
//     shard_info: Vec<ShardInfo>,
//
//     // /// Tracks the number of transactions that are currently being processed by each shard.
//     // /// Used for the purposes of load balancing.
//     // load_manager: ShardLoadManager,
// }
//
// impl<T> StreamingPartitioner<T>
// where
//     T: PTransaction,
//     T::Key: Eq + Hash + Clone,
// {
//     pub fn new(n_shards: usize) -> Self {
//         todo!()
//     }
//
//     /// Marks all writes made by this transaction as complete.
//     fn transactions_complete(&mut self, shard_id: ShardId, transactions: &[SerializationIdx]) {
//         self.load_manager.remove(shard_id, transactions.len());
//
//         for &idx in transactions.into_iter() {
//             let Partitioned(tx_info) = &self.transactions[idx as usize]
//                 else { panic!("transaction cannot be completed before it has been partitioned") };
//
//             for key in tx_info.write_set.iter() {
//                 let loc_info = self.partitioned_prefix_location_info.get_mut(key).unwrap();
//                 if loc_info.last_write.txn_idx == idx {
//                     loc_info.write_complete = true;
//                 }
//             }
//         }
//     }
//
//     fn add_transactions(&mut self, transactions: Vec<T>) {
//         // Add inverse dependency links.
//         for tx in transactions {
//             let idx = self.transactions.len() as u32;
//             let tx_info = PendingTxnInfo::new(tx);
//
//             for k in tx_info.transaction.read_set() {
//                 if let Some(&dependency_idx) = self.last_write_txn.get(&k) {
//                     if let Pending(txn_info) = &mut self.transactions[dependency_idx as usize] {
//                         txn_info.dependants.push(idx);
//                     }
//                 }
//             }
//
//             self.transactions.push(Pending(tx_info));
//         }
//     }
//
//     fn all_transactions_partitioned(&self) -> bool {
//         self.partitioned_idx == self.transactions.len()
//     }
//
//     fn n_shards(&self) -> usize {
//         self.load_manager.n_shards()
//     }
//
//     fn compute_score(&self, idx: SerializationIdx, tx_info: &PendingTxnInfo<T>) -> Vec<AffinityScore> {
//         let n_shards = self.n_shards();
//         let mut affinity_score = vec![0.; n_shards];
//
//         // Compute the dependency scores.
//         for k in tx_info.transaction.read_set() {
//             if let Some(loc_info) = self.partitioned_prefix_location_info.get(&k) {
//                 let loc_shard = loc_info.last_write.shard_id;
//                 let dependency_score = loc_info.compute_dependency_score(idx, n_shards);
//                 affinity_score[loc_shard.as_usize()] += dependency_score;
//             }
//         }
//
//         // Compute the community and load balancing scores.
//         for (shard_id, shard_info) in self.shard_info.iter().enumerate() {
//             let community_score = shard_info.compute_community_score(&tx_info.communities);
//             let load_score = shard_info.compute_load_score();
//             affinity_score[shard_id] += community_score;
//         }
//
//         todo!()
//     }
//
//     fn partition_transactions(&mut self) {
//         while !self.all_transactions_partitioned() && !self.load_manager.all_shards_satisfied() {
//             let tx_info = self.transactions[self.partitioned_idx].take_for_partitioning().unwrap();
//             self.partitioned_idx += 1;
//
//
//
//         }
//     }
// }
