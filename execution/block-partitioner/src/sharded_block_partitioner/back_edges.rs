// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use itertools::Itertools;
use aptos_types::transaction::analyzed_transaction::AnalyzedTransaction;
use crate::sharded_block_partitioner::cross_shard_messages::{CrossShardBackEdges, CrossShardClient};
use crate::types::{CrossShardDependencies, SubBlock, TxnIdxWithShardId, TxnIndex};

pub struct BackEdgeCreator {
    cross_shard_client: Arc<CrossShardClient>,
    froze_sub_blocks: Vec<SubBlock>,
}

impl BackEdgeCreator {
    pub fn new(cross_shard_client: Arc<CrossShardClient>, froze_sub_blocks: Vec<SubBlock>, num_shards: usize) -> Self {
        Self {
            cross_shard_client,
            froze_sub_blocks,
        }
    }

    pub fn create_back_edges(&mut self, cross_shard_deps: &[CrossShardDependencies], index_offset:usize) {
        if self.froze_sub_blocks.is_empty() {
            // early return in case this is the first round (no previous sub blocks, so no back edges)
            return;
        }
        // List of back edges for each shard and by source txn index
        let mut back_edges: Vec<HashMap<TxnIndex, HashSet<TxnIndex>>> =  vec![HashMap::new(); self.num_shards];
        for (index, cross_shard_deps) in cross_shard_deps.iter().enumerate() {
            let dependent_index = index + index_offset;
            self.insert_back_edges_for_txn(dependent_index, cross_shard_deps, &mut back_edges);
        }
        let back_edges_vec = self.send_and_collect_back_edges(back_edges);
        let back_edges = self.combine_back_edges(back_edges_vec);
        self.add_back_edges_to_sub_blocks(back_edges);
    }

    fn insert_back_edges_for_txn(&mut self, dependent_index: TxnIndex, cross_shard_deps: &CrossShardDependencies, back_edges: &mut Vec<HashMap<TxnIndex, HashSet<TxnIndex>>>) {
        for (shard_id, source_index) in cross_shard_deps.depends_on() {
            let back_edges_for_shard = back_edges.get_mut(*shard_id).unwrap();
            let back_edges = back_edges_for_shard.entry(*source_index).or_insert_with(|| HashSet::new());
            back_edges.insert(dependent_index);
        }
    }

    fn send_and_collect_back_edges(&self, back_edges: Vec<HashMap<TxnIndex, HashSet<TxnIndex>>>) -> Vec<Vec<CrossShardBackEdges>> {
        let mut back_edges_vec = Vec::new();
        for (shard_id, back_edges_for_shard) in back_edges.into_iter().enumerate() {
            let mut back_edges = Vec::new();
            for (source_index, dependent_indices) in back_edges_for_shard {
                back_edges.push(CrossShardBackEdges::new(source_index, dependent_indices));
            }
            back_edges_vec.push(back_edges);
        }
        self.cross_shard_client.broadcast_and_collect_back_edges(back_edges_vec)
    }

    fn combine_back_edges(&self, back_edges_vec: Vec<Vec<CrossShardBackEdges>>) -> Vec<(TxnIndex, HashSet<TxnIdxWithShardId>)> {
        // combine the back edges from different shards by source txn index
        let mut back_edges_by_source_index = HashMap::new();
        for (shard_id, back_edges) in back_edges_vec.into_iter().enumerate() {
            for back_edge in back_edges {
                let source_index = back_edge.source_txn_index;
                let back_edges = back_edges_by_source_index.entry(source_index).or_insert_with(|| HashSet::new());
                for dependent_idx in back_edge.dependent_txn_indices {
                    back_edges.insert(TxnIdxWithShardId::new(dependent_idx, shard_id));
                }
            }
        }
        // sort the back edges by source txn index and return a vector
        let mut back_edges_vec = back_edges_by_source_index.into_iter().collect_vec();
        back_edges_vec.sort_by_key(|(source_index, _)| *source_index);
        back_edges_vec
    }

    fn add_back_edges_to_sub_blocks(&mut self, back_edges: Vec<(TxnIndex, HashSet<TxnIdxWithShardId>)>) {
        let mut current_sub_block_index = 0;
        let mut current_sub_block = &mut self.froze_sub_blocks[0];
        for (source_index, dependent_indices) in back_edges.into_iter() {
            while source_index >= current_sub_block.end_index() {
                current_sub_block_index += 1;
                current_sub_block_end = &mut self.froze_sub_blocks[current_sub_block_index];
            }

            for dependent_idx in dependent_indices {
                current_sub_block.add_dependent_txn(source_index, dependent_idx);
            }
        }
    }

}
