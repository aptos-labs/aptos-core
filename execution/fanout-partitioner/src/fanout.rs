// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Copyright © Aptos Foundation

use aptos_block_partitioner::{BlockPartitioner, PartitionerConfig};
use aptos_types::{
    block_executor::partitioner::PartitionedTransactions,
    state_store::state_key::StateKey,
    transaction::analyzed_transaction::{AnalyzedTransaction, StorageLocation},
};
use std::{cmp::Reverse, collections::HashMap, mem};
use aptos_block_partitioner::v3::build_partitioning_result;
use itertools::Itertools;
use aptos_types::account_address::AccountAddress;

/// A partitioner that does not reorder and assign txns to shards in a round-robin way.
/// Only for testing the correctness or sharded execution V3.
#[derive(Default)]
pub struct FanoutPartitioner {
    pub print_debug_stats: bool,
}

impl BlockPartitioner for FanoutPartitioner {
    fn partition(
        &self,
        transactions: Vec<AnalyzedTransaction>,
        num_shards: usize,
    ) -> PartitionedTransactions {

        let compressed_graph = CompressedGraph::new(&transactions);

        if self.print_debug_stats {
            println!("Senders: {}, accesses: {}", compressed_graph.sender_to_idx.len(), compressed_graph.access_to_idx.len());
        }

        let sender_to_shard_idxs = priority_bfs_preassign(&compressed_graph, num_shards as u16);
        // let sender_to_shard_idxs = (compressed_graph.sender_start_idx..compressed_graph.sender_end_idx).map(|i| i as usize % num_shards).collect::<Vec<_>>();

        let shard_idxs = transactions.iter().map(|txn| sender_to_shard_idxs[*compressed_graph.sender_to_idx.get(&txn.sender().unwrap()).unwrap() as usize] as usize).collect();

        PartitionedTransactions::V3(build_partitioning_result(num_shards, transactions, shard_idxs, self.print_debug_stats))
    }
}

type Idx = u32;

struct CompressedGraph {
    sender_start_idx: Idx,
    sender_end_idx: Idx,
    access_start_idx: Idx,
    access_end_idx: Idx,

    sender_to_idx: HashMap<AccountAddress, Idx>,
    access_to_idx: HashMap<StateKey, Idx>,

    sender_weights: Vec<u32>,

    edges: Edges<Idx>,
}

struct Edges<T> {
    starts: Vec<u32>,
    destinations: Vec<T>,
}

impl<T: std::cmp::Ord> Edges<T> {
    fn get_edges(&self, from: u32) -> &[T] {
        &self.destinations[(self.starts[from as usize] as usize)..(self.starts[(from+1) as usize] as usize)]
    }

    fn new(num_sources: u32, mut source_to_dsts: HashMap<u32, Vec<T>>) -> Self {
        let mut starts = vec![];
        let mut destinations = vec![];

        let mut num_with_duplicates = 0;
        let mut num_deduped = 0;

        for i in 0..num_sources {
            starts.push(destinations.len() as u32);
            let mut dsts = source_to_dsts.remove(&i).unwrap_or_default();

            num_with_duplicates += dsts.len();

            dsts.sort_unstable();
            dsts.dedup();

            num_deduped += dsts.len();

            destinations.append(&mut dsts);
        }

        starts.push(destinations.len() as u32);

        println!("Created edges, went from {} to {}, after dedup", num_with_duplicates, num_deduped);
        Self {
            starts,
            destinations,
        }
    }
}

impl CompressedGraph {
    pub fn new(transactions: &[AnalyzedTransaction]) -> Self {
        let mut sender_to_idx = HashMap::new();
        let mut access_to_temp_idx = HashMap::new();

        let mut temp_access_to_senders = HashMap::<Idx, Vec<Idx>>::new();

        for txn in transactions {
            let sender = txn.transaction().sender().unwrap();
            let next_sender_idx = sender_to_idx.len() as Idx;
            sender_to_idx.entry(sender).or_insert(next_sender_idx);
        }

        let mut sender_weights = vec![0; sender_to_idx.len()];

        for txn in transactions {
            let sender = txn.transaction().sender().unwrap();
            let sender_idx = *sender_to_idx.get(&sender).unwrap();
            sender_weights[sender_idx as usize] += 1;
            for hint in txn.write_hints().iter() {
                match hint {
                    StorageLocation::Specific(state_key) => {
                        let next_access_idx = access_to_temp_idx.len() as Idx;
                        let access_idx = *access_to_temp_idx.entry(state_key.clone()).or_insert(next_access_idx);
                        temp_access_to_senders.entry(access_idx).or_default().push(sender_idx);
                    },
                    _ => panic!("unkown hint {:?}", hint),
                }
            }
        }

        let mut temp_access_to_senders = temp_access_to_senders
            .into_iter()
            .filter_map(|(k, mut v)| {
                v.sort_unstable();
                v.dedup();
                if v.len() <= 1 {
                    None
                } else {
                    Some((k, v))
                }
            }).collect::<HashMap<Idx, Vec<Idx>>>();

        let mut access_to_idx = HashMap::new();
        let mut edges = HashMap::<Idx, Vec<Idx>>::new();
        for (state_key, temp_idx) in access_to_temp_idx.into_iter() {
            if let Some(senders) = temp_access_to_senders.remove(&temp_idx) {
                let access_idx = (sender_to_idx.len() + access_to_idx.len()) as Idx;
                assert!(access_to_idx.insert(state_key, access_idx).is_none());
                edges.insert(access_idx, senders);
            }
        }

        for access_idx in access_to_idx.values() {
            let senders = edges.get(access_idx).unwrap().clone();
            for sender_idx in senders {
                edges.entry(sender_idx).or_default().push(*access_idx);
            }
        }

        let edges = Edges::new((sender_to_idx.len() + access_to_idx.len()) as u32, edges);

        Self {
            sender_start_idx: 0 as Idx,
            sender_end_idx: sender_to_idx.len() as Idx,
            access_start_idx: sender_to_idx.len() as Idx,
            access_end_idx: (sender_to_idx.len() + access_to_idx.len()) as Idx,
            sender_weights,
            sender_to_idx,
            access_to_idx,
            edges,
        }
    }

    fn get_weight(&self, idx: Idx) -> u32 {
        self.sender_weights.get(idx as usize).copied().unwrap_or(0)
    }
}

#[derive(Debug, Default)]
pub struct V3FanoutPartitionerConfig {}

impl PartitionerConfig for V3FanoutPartitionerConfig {
    fn build(&self) -> Box<dyn BlockPartitioner> {
        Box::new(FanoutPartitioner::default())
    }
}

const UNASSIGNED_SHARD: u16 = u16::MAX;

fn priority_bfs_preassign(graph: &CompressedGraph, num_shards: u16) -> Vec<u16> {
    let any_vertex_end_index = graph.access_end_idx.max(graph.sender_end_idx);

    let total_weight = graph.sender_weights.iter().sum::<u32>();
    let max_shard_weight = 1 + total_weight / num_shards as u32;

    println!("Total {} txns, to split across {} shards, with at most {} in each.", total_weight, num_shards, max_shard_weight);
    let mut assigned = vec![UNASSIGNED_SHARD; any_vertex_end_index as usize];
    let mut cur_bucket_idx = 0;
    let mut bucket_weights = vec![0u32; (num_shards + 1) as usize];

    // let mut start_vertex_to_degree = (0 .. any_vertex_end_index).map(|sender_idx| (sender_idx, graph.edges.get_edges(sender_idx).len() as u32)).collect::<Vec<_>>();
    let mut start_vertex_to_degree = (graph.sender_start_idx .. graph.sender_end_idx).map(|sender_idx| (sender_idx, graph.edges.get_edges(sender_idx).len() as u32)).collect::<Vec<_>>();
    start_vertex_to_degree.sort_by_key(|(_idx, degree)| Reverse(*degree));

    for (sender_idx, _) in start_vertex_to_degree {
        if assigned[sender_idx as usize] == UNASSIGNED_SHARD {
            let mut current_to_visit = vec![sender_idx];
            let mut next_to_visit = HashMap::new();

            while !current_to_visit.is_empty()  {
                for cur in &current_to_visit {
                    assigned[*cur as usize] = cur_bucket_idx;
                    let weight = graph.get_weight(*cur);
                    if weight > 0 {
                        assert!(cur_bucket_idx < num_shards, "{:?}, {}, {}", bucket_weights, max_shard_weight, weight);
                        bucket_weights[cur_bucket_idx as usize] += weight;

                        if bucket_weights[cur_bucket_idx as usize] >= max_shard_weight {
                            println!("Done {} with {}", cur_bucket_idx, weight);
                            break;
                        }
                    }

                    for connected in graph.edges.get_edges(*cur) {
                        if assigned[*connected as usize] == UNASSIGNED_SHARD {
                            *next_to_visit.entry(*connected).or_insert(0) += 1;
                        }
                    }
                }

                if bucket_weights[cur_bucket_idx as usize] >= max_shard_weight {
                    break;
                }

                let mut to_sort = HashMap::new();
                mem::swap(&mut to_sort, &mut next_to_visit);
                let mut new_to_visit = to_sort.into_iter().sorted_by_key(|(_idx, connections)| Reverse(*connections)).map(|(idx, _connections)| idx).collect::<Vec<_>>();
                mem::swap(&mut current_to_visit, &mut new_to_visit);
            }

            if bucket_weights[cur_bucket_idx as usize] >= max_shard_weight {
                println!("Filled another bucket {}, going to next {:?}", cur_bucket_idx, bucket_weights);
                cur_bucket_idx += 1;
            }
        }
    }

    assigned
}
