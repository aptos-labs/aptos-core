// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

pub mod scheduling;
pub mod no_op;
pub mod sharded_block_partitioner;
pub mod simple_partitioner;
pub mod beta;
mod union_find;

pub mod test_utils;

use std::collections::HashMap;
use aptos_metrics_core::{exponential_buckets, Histogram, HistogramVec, IntCounterVec, IntGaugeVec, register_histogram, register_histogram_vec, register_int_counter_vec};
use aptos_types::{block_executor::partitioner::SubBlocksForShard, transaction::Transaction};
use once_cell::sync::Lazy;
use aptos_logger::info;
use aptos_types::block_executor::partitioner::{RoundId, ShardId};
use aptos_types::state_store::state_key::StateKey;
use aptos_types::transaction::analyzed_transaction::{AnalyzedTransaction, StorageLocation};

pub trait BlockPartitioner: Send {
    fn partition(
        &self,
        transactions: Vec<Transaction>,
        num_executor_shards: usize,
    ) -> Vec<SubBlocksForShard<Transaction>>;
}

/// An implementation of partitioner that splits the transactions into equal-sized chunks.
pub struct UniformPartitioner {}

pub static APTOS_BLOCK_PARTITIONER_SECONDS: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        // metric name
        "aptos_block_partitioner_seconds",
        // metric description
        "The total time spent in seconds of block partitioning in the sharded block partitioner.",
        exponential_buckets(/*start=*/ 1e-3, /*factor=*/ 2.0, /*count=*/ 20).unwrap(),
    )
    .unwrap()
});

pub static APTOS_BLOCK_ANALYZER_SECONDS: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        // metric name
        "aptos_block_analyzer_seconds",
        // metric description
        "The total time spent in seconds of block analyzing.",
        exponential_buckets(/*start=*/ 1e-3, /*factor=*/ 2.0, /*count=*/ 20).unwrap(),
    )
        .unwrap()
});

pub static APTOS_SUB_BLOCK_SIZE: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        // metric name
        "aptos_sub_block_size",
        // metric description
        "foo",
        &["shard_id", "round_id"],
    ).unwrap()
});

pub static APTOS_SUB_BLOCK_INBOUND_COST: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        // metric name
        "aptos_sub_block_inbound_cost",
        // metric description
        "foo",
        &["shard_id", "round_id"],
    ).unwrap()
});

pub static APTOS_SUB_BLOCK_OUTBOUND_COST: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        // metric name
        "aptos_sub_block_outbound_cost",
        // metric description
        "foo",
        &["shard_id", "round_id"],
    ).unwrap()
});


pub fn analyze_block(txns: Vec<Transaction>) -> Vec<AnalyzedTransaction> {
    let _timer = APTOS_BLOCK_ANALYZER_SECONDS.start_timer();
    txns.into_iter().map(|t| t.into()).collect()
}


pub fn report_sub_block_matrix(matrix: &Vec<SubBlocksForShard<Transaction>>) {
    let mut total_comm_cost = 0;
    for (shard_id, sub_block_list) in matrix.iter().enumerate() {
        for (round_id, sub_block) in sub_block_list.sub_blocks.iter().enumerate() {
            let mut cur_sub_block_inbound_costs_by_key_src_pair: HashMap<(RoundId, ShardId, StateKey), u64> = HashMap::new();
            let mut cur_sub_block_connectivity_by_key_dst_pair: HashMap<(RoundId, ShardId, StateKey), u64> = HashMap::new();
            for (local_tid, td) in sub_block.transactions.iter().enumerate() {
                let tid = sub_block.start_index + local_tid;
                for (src_tid, locs) in td.cross_shard_dependencies.required_edges().iter() {
                    for loc in locs.iter() {
                        let key = loc.clone().into_state_key();
                        let value = cur_sub_block_inbound_costs_by_key_src_pair.entry((src_tid.round_id, src_tid.shard_id, key)).or_insert_with(||0);
                        *value += 1;
                        // let key_str = key.hash().to_hex();
                        // println!("PAREND - round={}, shard={}, tid={}, wait for key={} from round=???, shard={}, tid={}", round_id, shard_id, tid, key_str, src_tid.shard_id, src_tid.txn_index);

                    }
                }
                for (dst_tid, locs) in td.cross_shard_dependencies.dependent_edges().iter() {
                    for loc in locs.iter() {
                        let key = loc.clone().into_state_key();
                        let value = cur_sub_block_connectivity_by_key_dst_pair.entry((dst_tid.round_id, dst_tid.shard_id, key)).or_insert_with(||0);
                        *value += 1;
                        // let key_str = key.hash().to_hex();
                        // println!("PAREND - round={}, shard={}, tid={}, unblock key={} for round=???, shard={}, tid={}", round_id, shard_id, tid, key_str, src_tid.shard_id, src_tid.txn_index);
                    }
                }
            }
            let inbound_cost: u64 = cur_sub_block_inbound_costs_by_key_src_pair.iter().map(|(_,b)| *b).sum();
            let outbound_cost: u64 = cur_sub_block_connectivity_by_key_dst_pair.iter().map(|(_,b)| *b).sum();
            println!("MATRIX_REPORT: round={}, shard={}, sub_block_size={}, inbound_cost={}, outbound_cost={}", round_id, shard_id, sub_block.num_txns(), inbound_cost, outbound_cost);
            let shard_id_str = shard_id.to_string();
            let round_id_str = round_id.to_string();
            let label_vec = [shard_id_str.as_str(), round_id_str.as_str()];
            APTOS_SUB_BLOCK_SIZE.with_label_values(&label_vec).observe(sub_block.num_txns() as f64);
            APTOS_SUB_BLOCK_INBOUND_COST.with_label_values(&label_vec).observe(inbound_cost as f64);
            APTOS_SUB_BLOCK_OUTBOUND_COST.with_label_values(&label_vec).observe(outbound_cost as f64);
            total_comm_cost += inbound_cost + outbound_cost;
        }
    }
    println!("MATRIX_REPORT: total_comm_cost={}", total_comm_cost);
}
