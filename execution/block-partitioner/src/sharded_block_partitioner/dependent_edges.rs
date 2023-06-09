// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    sharded_block_partitioner::cross_shard_messages::{
        CrossShardClient, CrossShardClientInterface, CrossShardDependentEdges,
    },
    types::{CrossShardDependencies, CrossShardDependency, SubBlocksForShard, TxnIndex},
};
use itertools::Itertools;
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

pub struct DependentEdgeCreator {
    cross_shard_client: Arc<CrossShardClient>,
    froze_sub_blocks: SubBlocksForShard,
    num_shards: usize,
}

impl DependentEdgeCreator {
    pub fn new(
        cross_shard_client: Arc<CrossShardClient>,
        froze_sub_blocks: SubBlocksForShard,
        num_shards: usize,
    ) -> Self {
        Self {
            cross_shard_client,
            froze_sub_blocks,
            num_shards,
        }
    }

    pub fn create_dependent_edges(
        &mut self,
        curr_cross_shard_deps: &[CrossShardDependencies],
        index_offset: usize,
    ) {
        if self.froze_sub_blocks.is_empty() {
            // early return in case this is the first round (no previous sub blocks, so no back edges)
            return;
        }
        // List of dependent edges for each shard and by source txn index
        let mut dependent_edges: Vec<HashMap<TxnIndex, HashSet<TxnIndex>>> =
            vec![HashMap::new(); self.num_shards];
        for (index, cross_shard_deps) in curr_cross_shard_deps.iter().enumerate() {
            let dependent_index = index + index_offset;
            self.insert_dependent_edges_for_txn(
                dependent_index,
                cross_shard_deps,
                &mut dependent_edges,
            );
        }
        let back_edges_vec = self.send_and_collect_dependent_edges(dependent_edges);
        let back_edges = self.group_dependent_edges_by_source(back_edges_vec);
        self.add_dependent_edges_to_sub_blocks(back_edges);
    }

    fn insert_dependent_edges_for_txn(
        &mut self,
        dependent_index: TxnIndex,
        cross_shard_deps: &CrossShardDependencies,
        back_edges: &mut [HashMap<TxnIndex, HashSet<TxnIndex>>],
    ) {
        for index_with_shard in cross_shard_deps.required_txns().iter() {
            let back_edges_for_shard = back_edges.get_mut(index_with_shard.shard_id).unwrap();
            let back_edges = back_edges_for_shard
                .entry(index_with_shard.txn_index)
                .or_insert_with(HashSet::new);
            back_edges.insert(dependent_index);
        }
    }

    fn send_and_collect_dependent_edges(
        &self,
        back_edges: Vec<HashMap<TxnIndex, HashSet<TxnIndex>>>,
    ) -> Vec<Vec<CrossShardDependentEdges>> {
        let mut back_edges_vec = Vec::new();
        for (_, back_edges_for_shard) in back_edges.into_iter().enumerate() {
            let mut back_edges = Vec::new();
            for (source_index, dependent_indices) in back_edges_for_shard {
                back_edges.push(CrossShardDependentEdges::new(
                    source_index,
                    dependent_indices,
                ));
            }
            back_edges_vec.push(back_edges);
        }
        self.cross_shard_client
            .broadcast_and_collect_dependent_edges(back_edges_vec)
    }

    fn group_dependent_edges_by_source(
        &self,
        back_edges_vec: Vec<Vec<CrossShardDependentEdges>>,
    ) -> Vec<(TxnIndex, HashSet<CrossShardDependency>)> {
        // combine the back edges from different shards by source txn index
        let mut back_edges_by_source_index = HashMap::new();
        for (shard_id, back_edges) in back_edges_vec.into_iter().enumerate() {
            for back_edge in back_edges {
                let source_index = back_edge.source_txn_index;
                let back_edges = back_edges_by_source_index
                    .entry(source_index)
                    .or_insert_with(HashSet::new);
                for dependent_idx in back_edge.dependent_txn_indices {
                    back_edges.insert(CrossShardDependency::new(dependent_idx, shard_id));
                }
            }
        }
        // sort the back edges by source txn index and return a vector
        let mut back_edges_vec = back_edges_by_source_index.into_iter().collect_vec();
        back_edges_vec.sort_by_key(|(source_index, _)| *source_index);
        back_edges_vec
    }

    fn add_dependent_edges_to_sub_blocks(
        &mut self,
        dependent_edges: Vec<(TxnIndex, HashSet<CrossShardDependency>)>,
    ) {
        let mut current_sub_block_index = 0;
        let mut current_sub_block = self.froze_sub_blocks.get_sub_block_mut(0).unwrap();
        // Since the dependent edges are sorted by source txn index, we can iterate through the sub blocks and add the back edges to the sub blocks
        for (source_index, dependent_indices) in dependent_edges.into_iter() {
            while source_index >= current_sub_block.end_index() {
                current_sub_block_index += 1;
                current_sub_block = self
                    .froze_sub_blocks
                    .get_sub_block_mut(current_sub_block_index)
                    .unwrap();
            }

            for dependent_idx in dependent_indices {
                current_sub_block.add_dependent_txn(source_index, dependent_idx);
            }
        }
    }

    pub fn into_frozen_sub_blocks(self) -> SubBlocksForShard {
        self.froze_sub_blocks
    }
}
