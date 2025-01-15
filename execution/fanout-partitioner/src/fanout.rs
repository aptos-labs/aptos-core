// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Copyright © Aptos Foundation

use aptos_block_partitioner::{BlockPartitioner, PartitionerConfig};
use aptos_types::{
    block_executor::partitioner::PartitionedTransactions,
    state_store::state_key::StateKey,
    transaction::analyzed_transaction::{AnalyzedTransaction, StorageLocation},
};
use ordered_float::NotNan;
use rand::Rng;
use std::{cmp::Reverse, collections::{BinaryHeap, HashMap, VecDeque}, mem, ops::Range};
use std::cmp::min;
use aptos_block_partitioner::v3::build_partitioning_result;
use itertools::Itertools;
use aptos_logger::{debug, info};
use aptos_types::account_address::AccountAddress;

#[derive(Clone, Debug)]
pub enum InitStrategy {
    Random,
    PriorityBfs,
}

#[derive(Clone, Debug)]
pub struct V3FanoutPartitionerConfig {
    pub print_debug_stats: bool,
    pub fanout_detailed_debug_logs: bool,
    pub fanout_num_iterations: usize,
    pub fanout_init_randomly: bool,
    pub fanout_move_probability: f64,
    pub fanout_probability: f32,
    pub acceptable_shard_imbalance: f32,
}

impl PartitionerConfig for V3FanoutPartitionerConfig {
    fn build(&self) -> Box<dyn BlockPartitioner> {
        Box::new(FanoutPartitioner {
            print_debug_stats: self.print_debug_stats,
            print_detailed_debug_stats: self.fanout_detailed_debug_logs,
            num_iterations: self.fanout_num_iterations,
            init_strategy: if self.fanout_init_randomly { InitStrategy::Random } else { InitStrategy::PriorityBfs },
            move_probability: self.fanout_move_probability,
            init_fanout_formula: FanoutFormula::new(self.fanout_probability),
            acceptable_shard_imbalance: self.acceptable_shard_imbalance,
        })
    }
}

/// A partitioner that does not reorder and assign txns to shards in a round-robin way.
/// Only for testing the correctness or sharded execution V3.
pub struct FanoutPartitioner {
    pub print_debug_stats: bool,
    pub print_detailed_debug_stats: bool,
    pub num_iterations: usize,
    pub init_strategy: InitStrategy,
    pub move_probability: f64,
    pub init_fanout_formula: FanoutFormula,
    pub acceptable_shard_imbalance: f32,
}

impl BlockPartitioner for FanoutPartitioner {
    fn partition(
        &self,
        mut transactions: Vec<AnalyzedTransaction>,
        num_shards: usize,
    ) -> PartitionedTransactions {

        let compressed_graph = CompressedGraph::new(&transactions);

        if self.print_debug_stats {
            println!("Senders: {}, accesses: {}", compressed_graph.sender_to_idx.len(), compressed_graph.access_to_idx.len());
        }

        if compressed_graph.are_all_txns_independent() {
            if self.print_debug_stats {
                info!("All transactions are independent, assigning round robin.");
            }
            let num_txns = transactions.len();
            let stop_round_robin_threshold = min(num_txns / num_shards / 10, 800);
            let shard_idxs = (0..transactions.len()).map(|i| {
                if i >= num_txns - stop_round_robin_threshold {
                    0
                } else {
                    i % num_shards
                }
            }).collect();
            return PartitionedTransactions::V3(build_partitioning_result(num_shards, transactions, shard_idxs, self.print_debug_stats, self.print_detailed_debug_stats));
        }

        let sender_to_shard_idxs = match self.init_strategy {
            InitStrategy::Random => (compressed_graph.sender_start_idx..compressed_graph.sender_end_idx).map(|i| i as u16 % num_shards as u16).collect::<Vec<_>>(),
            InitStrategy::PriorityBfs => self.priority_bfs_preassign(&compressed_graph, num_shards as u16),
        };

        let sender_to_shard_idxs = self.optimize_probabilistic_fanout(sender_to_shard_idxs, &compressed_graph, num_shards as u16);

        self.optimize_transaction_order(&mut transactions, &sender_to_shard_idxs, &compressed_graph, num_shards as u16);
        self.correct_same_sender_transactions_order(&mut transactions);
        //transactions.sort_by_key(|txn| sender_to_shard_idxs[*compressed_graph.sender_to_idx.get(&txn.sender().unwrap()).unwrap() as usize]);
        //transactions = Self::optimize_order(transactions, &sender_to_shard_idxs, &compressed_graph, num_shards);

        let shard_idxs = transactions.iter().map(|txn| sender_to_shard_idxs[*compressed_graph.sender_to_idx.get(&txn.sender().unwrap()).unwrap() as usize] as usize).collect();

        PartitionedTransactions::V3(build_partitioning_result(num_shards, transactions, shard_idxs, self.print_debug_stats, self.print_detailed_debug_stats))
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

        debug!("Created edges, went from {} to {}, after dedup", num_with_duplicates, num_deduped);
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

    fn num_senders(&self) -> u32 {
        self.sender_end_idx - self.sender_start_idx
    }

    fn num_accesses(&self) -> u32 {
        self.access_end_idx - self.access_start_idx
    }

    fn access_to_vec_index(&self, access_idx: u32) -> usize {
        (access_idx - self.access_start_idx) as usize
    }

    fn access_range(&self) -> Range<u32> {
        self.access_start_idx..self.access_end_idx
    }

    fn sender_range(&self) -> Range<u32> {
        self.sender_start_idx..self.sender_end_idx
    }

    fn are_all_txns_independent(&self) -> bool {
        self.num_accesses() == 0 && self.sender_weights.iter().all(|&w| w == 1)
    }
}

const UNASSIGNED_SHARD: u16 = u16::MAX;

#[derive(Clone, Debug)]
pub struct FanoutFormula {
    fanout_probability: f32,
    fanout_1_minus_p: f32,
    fanout_move_out_multiplier: f32,
    fanout_move_in_multiplier: f32,

    // TODO add cached power table.
}

impl FanoutFormula {
    pub fn new(fanout_probability: f32) -> Self {
        let fanout_1_minus_p = 1.0 - fanout_probability;
        Self {
            fanout_probability,
            fanout_1_minus_p,
            fanout_move_out_multiplier: 1.0 - 1.0 / fanout_1_minus_p,
            fanout_move_in_multiplier: 1.0 - fanout_1_minus_p,
        }
    }

    fn calc_gain(&self, weights: &[f32], from: u16, to: u16) -> f32 {
        // negative of probabilistic fanout change, as we are minimizing
        -(weights[from as usize] * self.fanout_move_out_multiplier + weights[to as usize] * self.fanout_move_in_multiplier)
    }

    fn calc_gain_lower_limit(&self, weights: &[f32], from: u16) -> f32 {
        // negative of probabilistic fanout change, as we are minimizing
        -(weights[from as usize] * self.fanout_move_out_multiplier + self.fanout_move_in_multiplier)
    }

    fn calc_aggregatable_weight(&self, num_neighbors: u32) -> f32 {
        self.fanout_1_minus_p.powi(num_neighbors as i32)
    }

    fn calc_prob_fanout(&self, num_neighbors: u32) -> f32 {
        1.0 - self.fanout_1_minus_p.powi(num_neighbors as i32)
    }
}

impl FanoutPartitioner {
    fn priority_bfs_preassign(&self, graph: &CompressedGraph, num_shards: u16) -> Vec<u16> {
        let any_vertex_end_index = graph.access_end_idx.max(graph.sender_end_idx);

        let total_weight = graph.sender_weights.iter().sum::<u32>();
        let max_shard_weight = 1 + total_weight / num_shards as u32; // this can be higher like 1.2x

        if self.print_debug_stats {
            println!("Total {} txns, to split across {} shards, with at most {} in each.", total_weight, num_shards, max_shard_weight);
        }
        let mut assigned = vec![UNASSIGNED_SHARD; any_vertex_end_index as usize];
        let mut cur_bucket_idx = 0;
        let mut bucket_weights = vec![0u32; (num_shards + 1) as usize];

        // let mut start_vertex_to_degree = (0 .. any_vertex_end_index).map(|sender_idx| (sender_idx, graph.edges.get_edges(sender_idx).len() as u32)).collect::<Vec<_>>();
        let mut start_vertex_to_degree = graph.sender_range().map(|sender_idx| (sender_idx, graph.edges.get_edges(sender_idx).len() as u32)).collect::<Vec<_>>();
        start_vertex_to_degree.sort_by_key(|(_idx, degree)| Reverse(*degree));

        for (sender_idx, _) in start_vertex_to_degree {
            if assigned[sender_idx as usize] == UNASSIGNED_SHARD {
                let mut current_to_visit = vec![sender_idx];
                let mut next_to_visit: HashMap<u32, u32> = HashMap::new();

                while !current_to_visit.is_empty()  {
                    for cur in &current_to_visit {
                        assigned[*cur as usize] = cur_bucket_idx;
                        let weight = graph.get_weight(*cur);
                        if weight > 0 {
                            assert!(cur_bucket_idx < num_shards, "{:?}, {}, {}", bucket_weights, max_shard_weight, weight);
                            bucket_weights[cur_bucket_idx as usize] += weight;

                            if bucket_weights[cur_bucket_idx as usize] >= max_shard_weight {
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
                    let threshold = to_sort.iter().map(|(_idx, connections)| connections).max().map_or(0, |max| f64::sqrt((*max / 5) as f64) as u32);
                    let mut new_to_visit = to_sort.into_iter().filter(|(_idx, connections)| *connections >= threshold).sorted_by_key(|(_idx, connections)| Reverse(*connections)).map(|(idx, _connections)| idx).collect::<Vec<_>>();
                    mem::swap(&mut current_to_visit, &mut new_to_visit);
                }

                cur_bucket_idx = (cur_bucket_idx + 1) % num_shards;

                while bucket_weights[cur_bucket_idx as usize] >= max_shard_weight {
                    cur_bucket_idx = (cur_bucket_idx + 1) % num_shards;
                }
            }
        }

        if self.print_debug_stats {
            println!("Init shard sizes: {:?}", bucket_weights);
        }
        assigned
    }

    fn optimize_probabilistic_fanout(&self, mut sender_shard: Vec<u16>, graph: &CompressedGraph, num_shards: u16) -> Vec<u16> {
        let mut fanout_formula = self.init_fanout_formula.clone();
        let mut shard_weights = Self::compute_shard_weights(graph, num_shards, &sender_shard);
        let mut access_shards = Self::compute_access_shards(graph, num_shards, &sender_shard);
        let mut sender_shard_weights = self.compute_sender_shard_weights(graph, num_shards, &access_shards, &fanout_formula);

        let mut prev_fanout = self.print_fanout_stats(&access_shards, graph, "init", &fanout_formula);
        if self.print_debug_stats {
            self.print_weight_stats(&shard_weights);
        }
        for iter in 0..self.num_iterations {
            let num_moves = self.optimize_probabilistic_fanout_iteration(&mut sender_shard, &mut shard_weights, &mut access_shards, &mut sender_shard_weights, graph, num_shards, &fanout_formula);
            let cur_fanout = self.print_fanout_stats(&access_shards, graph, format!("iter {} with {} moves", iter, num_moves).as_str(), &fanout_formula);
            if (cur_fanout - 1.0) > (prev_fanout - 1.0) * 0.998 && fanout_formula.fanout_probability < 0.99 {
                let new_fanout_prob = fanout_formula.fanout_probability.powf(0.75);
                println!("Not enough improvement, increasing probability to {}", new_fanout_prob);
                fanout_formula = FanoutFormula::new(new_fanout_prob);
                sender_shard_weights = self.compute_sender_shard_weights(graph, num_shards, &access_shards, &fanout_formula);
            }
            prev_fanout = cur_fanout;
            if self.print_debug_stats {
                self.print_weight_stats(&shard_weights);
            }
        }

        sender_shard
    }

    fn compute_shard_weights(graph: &CompressedGraph, num_shards: u16, sender_shard: &Vec<u16>) -> Vec<u32> {
        let mut shard_weights = vec![0; num_shards as usize];
        for sender_id in graph.sender_range() {
            shard_weights[sender_shard[sender_id as usize] as usize] += graph.sender_weights[sender_id as usize]
        }
        shard_weights
    }

    fn compute_access_shards(graph: &CompressedGraph, num_shards: u16, sender_shard: &Vec<u16>) -> Vec<Vec<u32>> {
        let mut access_shards = vec![vec![]; graph.num_accesses() as usize];

        for access_idx in graph.access_range() {
            access_shards[graph.access_to_vec_index(access_idx)] = vec![0u32; num_shards as usize];
            for sender_idx in graph.edges.get_edges(access_idx) {
                access_shards[graph.access_to_vec_index(access_idx)][sender_shard[*sender_idx as usize] as usize] += 1;
            }
        }
        access_shards
    }

    fn optimize_order(mut transactions: Vec<AnalyzedTransaction>, sender_to_shard_idxs: &Vec<u16>, graph: &CompressedGraph, num_shards: usize) -> Vec<AnalyzedTransaction> {
        if num_shards == 1 {
            return transactions;
        }
        // assumes that transactions are sorted by shard_idx
        let num_txns = transactions.len();
        let mut shard_st_idx: Vec<usize> = vec![0; num_shards + 1];
        let mut last_shard = 0;
        let mut dependency_info: Vec<bool> = vec![false; num_txns];
        let mut owner_info: Vec<bool> = vec![false; num_txns];
        let mut txn_index_to_shard: Vec<u16> = vec![0; num_txns];
        // sender -> {txn_idxs}
        let mut sender_idx_to_txn_idxs: Vec<Vec<usize>> = vec![vec![]; graph.num_senders() as usize];

        for (txn_idx, txn) in transactions.iter().enumerate() {
            let sender_idx = *graph.sender_to_idx.get(&txn.sender().unwrap()).unwrap();
            sender_idx_to_txn_idxs[sender_idx as usize].push(txn_idx);
            let curr_shard = sender_to_shard_idxs[sender_idx as usize];
            txn_index_to_shard[txn_idx] = curr_shard;
            assert!(curr_shard >= last_shard);
            if curr_shard > last_shard {
                shard_st_idx[curr_shard as usize] = txn_idx;
                last_shard = curr_shard;
            }
        }
        assert_eq!(num_txns, sender_idx_to_txn_idxs.iter().map(|v| v.len()).sum::<usize>());
        // Fill all the trailing zeros with num_txns
        for i in (1..num_shards + 1).rev() {
            if shard_st_idx[i] == 0 {
                shard_st_idx[i] = num_txns;
            }
        }

        // access_idx -> [all_txns_shard_0, all_txns_shard_1, ...]
        //let empty_vec: Vec<usize> = vec![];
        let mut access_idx_to_txn_idxs: Vec<Vec<Vec<usize>>> = vec![vec![vec![]; num_shards]; graph.num_accesses() as usize];
        for access_idx in graph.access_range() {
            let access_vec_idx = graph.access_to_vec_index(access_idx);
            for sender_idx in graph.edges.get_edges(access_idx) {
                let mut txn_idxs = sender_idx_to_txn_idxs[*sender_idx as usize].clone(); // todo avoid clone
                let shard_num = sender_to_shard_idxs[*sender_idx as usize] as usize;
                assert_eq!(shard_num, txn_index_to_shard[txn_idxs[0]] as usize);
                //access_idx_to_txn_idxs[access_vec_idx][shard_num] = txn_idxs;
                access_idx_to_txn_idxs[access_vec_idx][shard_num].append(&mut txn_idxs);
            }

            let mut potential_owner_txns = &access_idx_to_txn_idxs[access_vec_idx][0];
            let mut potential_owner_txns_shard_num = 0;
            for shard_num in 1..num_shards {
                let dependent_txns = &access_idx_to_txn_idxs[access_vec_idx][shard_num];
                if dependent_txns.is_empty() {
                    continue;
                } else if !potential_owner_txns.is_empty() {
                    for txn_idx in potential_owner_txns {
                        assert_eq!(txn_index_to_shard[*txn_idx] as usize, potential_owner_txns_shard_num);
                        owner_info[*txn_idx] = true;
                    }
                    for txn_idx in dependent_txns {
                        assert_eq!(txn_index_to_shard[*txn_idx] as usize, shard_num);
                        dependency_info[*txn_idx] = true;
                    }
                }
                potential_owner_txns = dependent_txns;
                potential_owner_txns_shard_num = shard_num;
            }
        }



        let mut shard_rank_marker: Vec<usize> = vec![0; num_shards];
        info!("Shard start indices: {:?}", shard_st_idx);
        for shard_num in 0..num_shards {
            let shard_size = shard_st_idx[shard_num + 1] - shard_st_idx[shard_num];
            shard_rank_marker[shard_num] = shard_st_idx[shard_num] + (shard_size * shard_num) / num_shards;
        }

        let mut last_shard: isize = -1;

        let (num_owner_txns_by_shard, num_dependent_txns_by_shard, both_owner_dependent_txns_by_shard): (Vec<usize>, Vec<usize>, Vec<usize>) = {
            let mut num_owner_txns_by_shard = Vec::with_capacity(num_shards);
            let mut num_dependent_txns_by_shard = Vec::with_capacity(num_shards);
            let mut both_owner_dependent_txns_by_shard = Vec::with_capacity(num_shards);

            for shard_num in 0..num_shards {
                let shard_start_idx = shard_st_idx[shard_num];
                let shard_end_idx = shard_st_idx[shard_num + 1];
                let (num_owner_txns, num_dependent_txns, num_both_txns) = (shard_start_idx..shard_end_idx).fold((0, 0, 0), |(owner_count, dep_count, both_count), i| {
                    (
                        owner_count + owner_info[i] as usize,
                        dep_count + dependency_info[i] as usize,
                        both_count + (owner_info[i] && dependency_info[i]) as usize,
                    )
                });
                num_owner_txns_by_shard.push(num_owner_txns);
                num_dependent_txns_by_shard.push(num_dependent_txns);
                both_owner_dependent_txns_by_shard.push(num_both_txns);
            }

            (num_owner_txns_by_shard, num_dependent_txns_by_shard, both_owner_dependent_txns_by_shard)
        };

        let num_owner_txns = owner_info.iter().filter(|&&x| x).count();
        let num_dependent_txns = dependency_info.iter().filter(|&&x| x).count();
        info!("Owner txns: {}, Dependent txns: {}", num_owner_txns, num_dependent_txns);
        //info!("Pre swap: Dependency info: {:?}, Owner info: {:?}", dependency_info, owner_info);

        let mut num_owner_swaps = 0;
        let mut num_dependency_swaps = 0;
        let mut num_both_swaps = 0;
        let mut num_owner_swaps_by_shard: Vec<usize> = vec![0; num_shards];
        let mut num_dependency_swaps_by_shard: Vec<usize> = vec![0; num_shards];
        let mut num_both_swaps_by_shard: Vec<usize> = vec![0; num_shards];

        // Case 1: if dependency_info[idx] && owner_info[idx]
        for idx in 0..num_txns {
            let curr_shard = txn_index_to_shard[idx] as usize;
            let shard_start_idx = shard_st_idx[curr_shard];
            let shard_end_idx = shard_st_idx[curr_shard + 1];
            let shard_size = shard_end_idx - shard_start_idx;

            if curr_shard as isize > last_shard {
                info!("Re-ordering txns: shard_num {}, shard_start_idx {}, shard_rank_marker {}, shard_end_idx {}, shard_size {}, num_owner_txns_by_shard {}, num_dependent_txns_by_shard {}, both_owner_dependent_txns_by_shard {}",
                    curr_shard, shard_start_idx, shard_rank_marker[curr_shard], shard_end_idx, shard_size, num_owner_txns_by_shard[curr_shard], num_dependent_txns_by_shard[curr_shard], both_owner_dependent_txns_by_shard[curr_shard]);
                last_shard = curr_shard as isize;
            }

            if dependency_info[idx] && owner_info[idx] {
                let shard_size_ten_percent = shard_size / 10;
                let shard_rank_marker_end = shard_rank_marker[curr_shard] + shard_size_ten_percent + 1;
                if let Some(swap_idx) = (shard_rank_marker[curr_shard]..shard_rank_marker_end).find(|&i| !dependency_info[i] && !owner_info[i]) {
                    num_both_swaps += 1;
                    num_both_swaps_by_shard[curr_shard] += 1;
                    assert_eq!(txn_index_to_shard[idx], txn_index_to_shard[swap_idx]);
                    transactions.swap(idx, swap_idx);
                    dependency_info.swap(idx, swap_idx);
                    owner_info.swap(idx, swap_idx);
                    txn_index_to_shard.swap(idx, swap_idx);
                }
            }
        }

        // Case 2: if dependency_info[idx]
        for idx in 0..num_txns {
            if dependency_info[idx] && !owner_info[idx] {
                let curr_shard = txn_index_to_shard[idx] as usize;
                let shard_start_idx = shard_st_idx[curr_shard];
                let shard_end_idx = shard_st_idx[curr_shard + 1];
                if let Some(swap_idx) = (shard_start_idx..shard_end_idx).rfind(|&i| !dependency_info[i]) {
                    if swap_idx <= idx {
                        continue;
                    }
                    num_dependency_swaps += 1;
                    num_dependency_swaps_by_shard[curr_shard] += 1;
                    assert_eq!(txn_index_to_shard[idx], txn_index_to_shard[swap_idx]);
                    transactions.swap(idx, swap_idx);
                    dependency_info.swap(idx, swap_idx);
                    owner_info.swap(idx, swap_idx);
                    txn_index_to_shard.swap(idx, swap_idx);
                }
            }
        }

        // Case 3: if owner_info[idx]
        for idx in 0..num_txns {
            if owner_info[idx] && !dependency_info[idx] {
                let curr_shard = txn_index_to_shard[idx] as usize;
                let shard_start_idx = shard_st_idx[curr_shard];
                let shard_end_idx = shard_st_idx[curr_shard + 1];
                if let Some(swap_idx) = (shard_start_idx..shard_end_idx).find(|&i| !owner_info[i]) {
                    if swap_idx >= idx {
                        continue;
                    }
                    num_owner_swaps += 1;
                    num_owner_swaps_by_shard[curr_shard] += 1;
                    assert_eq!(txn_index_to_shard[idx], txn_index_to_shard[swap_idx]);
                    transactions.swap(idx, swap_idx);
                    dependency_info.swap(idx, swap_idx);
                    owner_info.swap(idx, swap_idx);
                    txn_index_to_shard.swap(idx, swap_idx);
                }
            }
        }

        info!("Owner swaps: {}, Dependency swaps: {}, Both swaps: {}", num_owner_swaps, num_dependency_swaps, num_both_swaps);
        //info!("Post swap: Dependency info: {:?}, Owner info: {:?}", dependency_info, owner_info);
        for shard_num in 0..num_shards {
            info!("Shard {}: Owner swaps: {}, Dependency swaps: {}, Both swaps: {}", shard_num, num_owner_swaps_by_shard[shard_num], num_dependency_swaps_by_shard[shard_num], num_both_swaps_by_shard[shard_num]);
        }

        transactions
    }

    fn compute_sender_shard_weights(&self, graph: &CompressedGraph, num_shards: u16, access_shards: &Vec<Vec<u32>>, fanout_formula: &FanoutFormula) -> Vec<Vec<f32>> {
        let mut sender_shard_weights = Vec::with_capacity(graph.num_senders() as usize);

        for sender in graph.sender_range() {
            let mut weights = vec![0.0f32; num_shards as usize];
            for access in graph.edges.get_edges(sender) {
                let cur_shards = &*access_shards[graph.access_to_vec_index(*access)];
                for i in 0..(num_shards as usize) {
                    weights[i] += fanout_formula.calc_aggregatable_weight(cur_shards[i]);
                }
            }
            sender_shard_weights.push(weights);
        }
        sender_shard_weights
    }

    fn print_weight_stats(&self, shard_weights: &[u32]) {
        let max = *shard_weights.iter().max().unwrap();
        let min = *shard_weights.iter().min().unwrap();
        let sum = shard_weights.iter().sum::<u32>();
        let avg = sum / shard_weights.len() as u32;
        info!("Shard weights: avg={}, min={}, max={}, min_ratio={}, max_ratio={} ", avg, min, max, min as f32 / avg as f32, max as f32 / avg as f32);
    }

    fn print_fanout_stats(&self, access_shards: &[Vec<u32>], graph: &CompressedGraph, desc: &str, fanout_formula: &FanoutFormula) -> f32 {
        let mut fanout = 0;
        let mut prob_fanout = 0.0f32;
        let mut in_max = 0;
        let mut in_total = 0;
        let num_accesses = (graph.access_end_idx - graph.access_start_idx) as f32;
        for access_idx in graph.access_range() {
            for count in &access_shards[graph.access_to_vec_index(access_idx)] {
                if *count > 0 {
                    fanout += 1;
                    prob_fanout += fanout_formula.calc_prob_fanout(*count);
                }
            }
            let max = *access_shards[graph.access_to_vec_index(access_idx)].iter().max().unwrap();
            let total = access_shards[graph.access_to_vec_index(access_idx)].iter().sum::<u32>();
            in_max += max;
            in_total += total;
        }
        if self.print_debug_stats {
            info!("{}: fanuot: {}, prob fanout: {}, avg degree in max: {}, avg degree in rest {}", desc, fanout as f32 / num_accesses, prob_fanout / num_accesses, in_max as f32 / num_accesses, (in_total - in_max) as f32 / num_accesses);
        }
        fanout as f32 / num_accesses
    }

    fn optimize_probabilistic_fanout_iteration(&self, sender_shard: &mut Vec<u16>, shard_weights: &mut Vec<u32>, access_shards: &mut Vec<Vec<u32>>, sender_shard_weights: &mut Vec<Vec<f32>>, graph: &CompressedGraph, num_shards: u16, fanout_formula: &FanoutFormula) -> usize {
        let target_shard_weight = shard_weights.iter().sum::<u32>() / shard_weights.len() as u32;
        let max_shard_weight = (1f32 + target_shard_weight as f32 * (100.0 + self.acceptable_shard_imbalance) / 100.0) as u32;

        let mut overall_queue: BinaryHeap<(NotNan<f32>, u32, u16, u16)> = BinaryHeap::new();
        let mut best_queue: Vec<Vec<BinaryHeap<(NotNan<f32>, u32)>>> = vec![vec![BinaryHeap::new(); num_shards as usize]; num_shards as usize];

        let mut least_worst_queue: Vec<BinaryHeap<(NotNan<f32>, u32)>> = vec![BinaryHeap::new(); num_shards as usize];

        for sender in graph.sender_range() {
            let weights =  &sender_shard_weights[sender as usize];
            let cur_sender_shard = sender_shard[sender as usize];

            let best_end_shard = weights
                .iter()
                .enumerate()
                .filter(|(index, _)| *index != cur_sender_shard as usize)
                .min_by(|(_, a), (_, b)| a.total_cmp(b))
                .map(|(index, _)| index)
                .unwrap() as u16;

            let gain = fanout_formula.calc_gain(&weights, cur_sender_shard, best_end_shard);
            let gain_not_nan = NotNan::new(gain).unwrap();

            if &gain > &0.0 {
                overall_queue.push((gain_not_nan, sender, cur_sender_shard, best_end_shard));
            }
            best_queue[cur_sender_shard as usize][best_end_shard as usize].push((gain_not_nan, sender));

            let gain_lower_limit = NotNan::new(fanout_formula.calc_gain_lower_limit(&weights, cur_sender_shard)).unwrap();
            least_worst_queue[cur_sender_shard as usize].push((gain_lower_limit, sender));
        }

        let mut moved = vec![false; graph.num_senders() as usize];
        let mut num_moves = 0;

        let mut best_gain: f32 = 0.0;
        let mut rng = rand::thread_rng();

        let move_sender = |sender: u32, sender_weight: u32, to_shard: u16, moved: &mut Vec<bool>, sender_shard: &mut Vec<u16>, shard_weights: &mut Vec<u32>| {
            let sender = sender as usize;
            moved[sender] = true;
            let from_shard = sender_shard[sender];
            shard_weights[from_shard as usize] -= sender_weight;
            shard_weights[to_shard as usize] += sender_weight;

            sender_shard[sender] = to_shard;
        };

        while !overall_queue.is_empty() {
            let (gain, sender, from_shard, to_shard) = overall_queue.pop().unwrap();
            if moved[sender as usize] {
                continue;
            }
            let sender_weight = graph.sender_weights[sender as usize];
            let from_shard_weight = shard_weights[from_shard as usize];
            let to_shard_weight = shard_weights[to_shard as usize];

            let alone_gain: f32 = gain.clone().into_inner();
            if alone_gain > 0.0 && from_shard_weight.max(to_shard_weight) >= (from_shard_weight - sender_weight).max(to_shard_weight + sender_weight) {

                if self.print_detailed_debug_stats && num_moves == 0 {
                    println!("matched alone: {} {}=>{}: {}, all: {:?}", sender, from_shard, to_shard, gain,
                        graph.edges.get_edges(sender).iter().map(|access| &*access_shards[graph.access_to_vec_index(*access)]).collect::<Vec<_>>()
                    );
                }
                move_sender(sender, sender_weight, to_shard, &mut moved, sender_shard, shard_weights);
                num_moves += 1;
                best_gain = best_gain.max(alone_gain);
                continue;
            }

            let cur_best_queue = &mut best_queue[to_shard as usize][from_shard as usize];
            loop {
                if let Some((_, other_sender)) = cur_best_queue.peek() {
                    if moved[*other_sender as usize] {
                        cur_best_queue.pop();
                        continue;
                    }
                }
                break;
            }

            if let Some((other_gain, other_sender)) = cur_best_queue.peek() {
                let other_weight = graph.sender_weights[*other_sender as usize];
                let pass_weight_check = from_shard_weight.max(to_shard_weight).max(max_shard_weight) >=
                    (from_shard_weight - sender_weight + other_weight).max(to_shard_weight + sender_weight - other_weight);

                let total_gain = (gain + other_gain).into_inner();

                if pass_weight_check && !moved[*other_sender as usize] && total_gain > best_gain / 100.0 && rng.gen_bool(self.move_probability) {
                    if self.print_detailed_debug_stats && num_moves == 0 {
                        println!("{} {}=>{}: {}, all: {:?}", sender, from_shard, to_shard, gain,
                            graph.edges.get_edges(sender).iter().map(|access| &*access_shards[graph.access_to_vec_index(*access)]).collect::<Vec<_>>()
                        );

                        println!("matched from best queue, {} {}=>{}: {}, all: {:?}", other_sender, to_shard, from_shard, other_gain,
                            graph.edges.get_edges(*other_sender).iter().map(|access| &*access_shards[graph.access_to_vec_index(*access)]).collect::<Vec<_>>()
                        );
                    }
                    move_sender(sender, sender_weight, to_shard, &mut moved, sender_shard, shard_weights);
                    move_sender(*other_sender, other_weight, from_shard, &mut moved, sender_shard, shard_weights);
                    num_moves += 2;
                    best_gain = best_gain.max(total_gain);
                    cur_best_queue.pop();
                    continue;
                }
            }

            let cur_least_worst_queue = &mut least_worst_queue[to_shard as usize];
            loop {
                if let Some((_, other_sender)) = cur_least_worst_queue.peek() {
                    if moved[*other_sender as usize] {
                        cur_least_worst_queue.pop();
                        continue;
                    }
                }
                break;
            }

            if let Some((other_gain_lower_limit, other_sender)) = cur_least_worst_queue.peek() {
                let other_weight = graph.sender_weights[*other_sender as usize];
                let pass_weight_check = from_shard_weight.max(to_shard_weight).max(max_shard_weight) >=
                    (from_shard_weight - sender_weight + other_weight).max(to_shard_weight + sender_weight - other_weight);

                let other_gain = fanout_formula.calc_gain(&sender_shard_weights[*other_sender as usize], to_shard, from_shard);
                let total_gain = (gain + other_gain).into_inner();
                if pass_weight_check && !moved[*other_sender as usize] && total_gain > best_gain / 100.0 && rng.gen_bool(self.move_probability) {
                    if self.print_detailed_debug_stats && num_moves == 0 {
                        println!("{} {}=>{}: {}, all: {:?}", sender, from_shard, to_shard, gain,
                            graph.edges.get_edges(sender).iter().map(|access| &*access_shards[graph.access_to_vec_index(*access)]).collect::<Vec<_>>()
                        );

                        println!("matched from least worst queue {} {}=>{}: {} (ll={}), all: {:?}", other_sender, to_shard, from_shard, other_gain, other_gain_lower_limit,
                            graph.edges.get_edges(*other_sender).iter().map(|access| &*access_shards[graph.access_to_vec_index(*access)]).collect::<Vec<_>>()
                        );
                    }
                    move_sender(sender, sender_weight, to_shard, &mut moved, sender_shard, shard_weights);
                    move_sender(*other_sender, other_weight, from_shard, &mut moved, sender_shard, shard_weights);
                    num_moves += 2;
                    best_gain = best_gain.max(total_gain);
                    cur_least_worst_queue.pop();
                    continue;
                }
            }
        }

        // this can be done incrementally.
        // *shard_weights = Self::compute_shard_weights(graph, num_shards, &sender_shard);
        *access_shards = Self::compute_access_shards(graph, num_shards, sender_shard);
        *sender_shard_weights = self.compute_sender_shard_weights(graph, num_shards, &access_shards, fanout_formula);

        num_moves
    }

    fn optimize_transaction_order(&self, transactions: &mut Vec<AnalyzedTransaction>, sender_shard: &Vec<u16>, graph: &CompressedGraph, num_shards: u16) {
        let access_shards = Self::compute_access_shards(graph, num_shards, sender_shard);

        let mut sender_to_positions = vec![(0.0, 0usize); graph.num_senders() as usize];

        for access_idx in graph.access_range() {
            let sorted = access_shards[graph.access_to_vec_index(access_idx)]
                .iter()
                .cloned()
                .enumerate()
                .filter(|(_idx, count)| *count > 0)
                .sorted_by_key(|(_idx, count)| Reverse(*count))
                .enumerate()
                .collect::<Vec<_>>();

            let chain_len = sorted.len();
            if chain_len > 1 {
                let mut shard_position = vec![num_shards as usize; num_shards as usize];
                for (position, (shard_idx, _count)) in &sorted {
                    shard_position[*shard_idx] = *position;
                }

                for sender in graph.edges.get_edges(access_idx) {
                    let (sum, count) = sender_to_positions.get_mut(*sender as usize).unwrap();
                    let position = shard_position[sender_shard[*sender as usize] as usize];
                    assert!(position < num_shards as usize, "{:?}, sorted: {:?}, shard_position: {:?}, sender_shard: {:?}", access_shards[graph.access_to_vec_index(access_idx)], sorted, shard_position, sender_shard[*sender as usize]);
                    let position = position as f32 / (chain_len - 1) as f32;
                    *sum += position;
                    *count += 1;
                }
            }
        }

        let mut tmp = vec![];
        mem::swap(&mut tmp, transactions);
        let (unconstrained, mut constrained): (Vec<_>, Vec<_>) = tmp.into_iter().map(|txn| {
            let sender = *graph.sender_to_idx.get(&txn.sender().unwrap()).unwrap() as usize;
            let (sum, count) = sender_to_positions[sender];
            (txn, sum, count, sender_shard[sender])
        }).partition(|(_txn, _sum, count, _shard)| *count == 0);

        constrained.sort_by_key(|(_txn, sum, count, shard)| (Reverse(NotNan::new(*sum / (*count as f32)).unwrap()), *shard));

        //let mut shard_to_unconstrained = vec![vec![]; num_shards as usize];
        let mut shard_to_unconstrained: Vec<VecDeque<AnalyzedTransaction>> = vec![VecDeque::new(); num_shards as usize];
        for (txn, _sum, _count, shard) in unconstrained {
            shard_to_unconstrained[shard as usize].push_back(txn);
        }

        if self.print_debug_stats {
            info!("Starting constrained counts {:?}", shard_to_unconstrained.iter().map(|v| v.len()).collect::<Vec<_>>());
        }

        // transactions.sort_by_key(|txn| {
        //     let sender = *graph.sender_to_idx.get(&txn.sender().unwrap()).unwrap() as usize;
        //     let (sum, count) = sender_to_positions[sender];
        //     Reverse((NotNan::new(if count > 0 { sum / (count as f32) } else { 0.5 }).unwrap(), sender_shard[sender]))
        // });

        // if self.print_debug_stats {

        let mut prev = 1.0;
        let mut last = 1.0 + 1.0 / num_shards as f32;
        let mut shard_fill = vec![0u32; num_shards as usize];
        for (txn, sum, count, shard) in constrained.into_iter() {
            let cur = sum / (count as f32);
            if prev > cur {
                if self.print_detailed_debug_stats {
                    println!("Ending {} at {}, cur {}={}/{}. shard fill: {:?}", prev, transactions.len(), cur, sum, count, shard_fill);
                }

                if (last == 1.0 && last > cur) || (last > cur as f32 + 1.0 / num_shards as f32) {
                    if !self.print_detailed_debug_stats && self.print_debug_stats {
                        println!("Ending {} at {}, cur {}={}/{}. shard fill: {:?}", prev, transactions.len(), cur, sum, count, shard_fill);
                    }

                    let fill_max = *shard_fill.iter().max().unwrap();

                    // let fill_max = (0..num_shards).map(|shard| shard_to_unconstrained[shard as usize].len() as u32 + shard_fill[shard as usize]).min().unwrap().min(fill_max);

                    for shard in 0..num_shards {
                        let cur_unconstrained = &mut shard_to_unconstrained[shard as usize];
                        while !cur_unconstrained.is_empty() && shard_fill[shard as usize] < fill_max {
                            transactions.push(cur_unconstrained.pop_front().unwrap());
                            shard_fill[shard as usize] += 1;
                            if cur_unconstrained.is_empty() {
                                println!("Run out of unconstrained on {}", shard);
                            }
                        }
                    }

                    last = cur;
                }

                prev = cur;
            }
            // if self.print_debug_stats {
            //     if transactions.len() % 10000 == 0 {
            //         println!("running value {} at {}, cur {}={}/{}. shard fill: {:?}", prev, transactions.len(), cur, sum, count, shard_fill);
            //     }
            // }

            transactions.push(txn);
            shard_fill[shard as usize] += 1;
        }
        if self.print_debug_stats {
            println!("Ending {} at {}. shard fill: {:?}", prev, transactions.len(), shard_fill);
        }

        for shard in 0..num_shards {
            shard_fill[shard as usize] += shard_to_unconstrained[shard as usize].len() as u32;
            transactions.append(&mut shard_to_unconstrained[shard as usize].make_contiguous().to_vec());
        }

        if self.print_debug_stats {
            println!("Ending after top-up {} at {}. shard fill: {:?}", prev, transactions.len(), shard_fill);
        }

        // transactions.sort_by_key(|txn| sender_to_shard_idxs[*compressed_graph.sender_to_idx.get(&txn.sender().unwrap()).unwrap() as usize]);

    }

    fn correct_same_sender_transactions_order(&self, transactions: &mut Vec<AnalyzedTransaction>) {
        let mut sender_to_idxs_and_seqnos: HashMap<AccountAddress, Vec<(usize, u64)>>  = HashMap::new();

        for (idx, txn) in transactions.iter().enumerate() {
            let sender = txn.sender().unwrap();
            let seqno = txn.transaction().expect_valid().try_as_signed_user_txn().unwrap().sequence_number();;
            sender_to_idxs_and_seqnos.entry(sender).or_insert(vec![]).push((idx, seqno));
        }

        let mut count_non_monotonic_sender_txns = 0;
        for (_sender, idxs_and_seqnos) in sender_to_idxs_and_seqnos.iter_mut() {
            let mut prev_seqno = 0;
            let mut prev_idx = 0;
            for (idx, seqno) in idxs_and_seqnos.iter() {
                if *seqno < prev_seqno {
                    count_non_monotonic_sender_txns += 1;
                    if self.print_detailed_debug_stats {
                        info!("Non-monotonic sender txns: sender {:?}", idxs_and_seqnos);
                    }
                    transactions.swap(*idx, prev_idx);
                }
                prev_seqno = *seqno;
                prev_idx = *idx;
            }
        }
        if self.print_debug_stats {
            info!("Non-monotonic senders txns count: {}", count_non_monotonic_sender_txns);
        }
    }
}
