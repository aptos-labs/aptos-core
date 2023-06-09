// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    sharded_block_partitioner::cross_shard_messages::{
        CrossShardClient, CrossShardClientInterface, CrossShardDependentEdges,
    },
    types::{
        CrossShardDependencies, CrossShardEdges, ShardId, SubBlocksForShard, TxnIdxWithShardId,
        TxnIndex,
    },
};
use itertools::Itertools;
use std::{collections::HashMap, sync::Arc};

pub struct DependentEdgeCreator {
    shard_id: ShardId,
    cross_shard_client: Arc<CrossShardClient>,
    froze_sub_blocks: SubBlocksForShard,
    num_shards: usize,
}

impl DependentEdgeCreator {
    pub fn new(
        shard_id: ShardId,
        cross_shard_client: Arc<CrossShardClient>,
        froze_sub_blocks: SubBlocksForShard,
        num_shards: usize,
    ) -> Self {
        Self {
            shard_id,
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
        let mut dependent_edges: Vec<HashMap<TxnIndex, CrossShardEdges>> =
            vec![HashMap::new(); self.num_shards];
        for (index, cross_shard_deps) in curr_cross_shard_deps.iter().enumerate() {
            let dependent_index = index + index_offset;
            self.insert_dependent_edges_for_txn(
                dependent_index,
                cross_shard_deps,
                &mut dependent_edges,
            );
        }
        let dep_edges_vec = self.send_and_collect_dependent_edges(dependent_edges);
        let dep_edges = self.group_dependent_edges_by_source_idx(dep_edges_vec);
        self.add_dependent_edges_to_sub_blocks(dep_edges);
    }

    fn insert_dependent_edges_for_txn(
        &mut self,
        dependent_index: TxnIndex,
        cross_shard_deps: &CrossShardDependencies,
        back_edges: &mut [HashMap<TxnIndex, CrossShardEdges>],
    ) {
        for (index_with_shard, storage_locations) in cross_shard_deps.required_edges_iter() {
            let back_edges_for_shard = back_edges.get_mut(index_with_shard.shard_id).unwrap();
            let back_edges = back_edges_for_shard
                .entry(index_with_shard.txn_index)
                .or_insert_with(CrossShardEdges::default);
            back_edges.add_edge(
                TxnIdxWithShardId::new(dependent_index, self.shard_id),
                storage_locations.clone(),
            );
        }
    }

    fn send_and_collect_dependent_edges(
        &self,
        dependent_edges: Vec<HashMap<TxnIndex, CrossShardEdges>>,
    ) -> Vec<Vec<CrossShardDependentEdges>> {
        let mut back_edges_vec = Vec::new();
        for (_, back_edges_for_shard) in dependent_edges.into_iter().enumerate() {
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

    fn group_dependent_edges_by_source_idx(
        &self,
        dependent_edges_vec: Vec<Vec<CrossShardDependentEdges>>,
    ) -> Vec<(TxnIndex, CrossShardEdges)> {
        // combine the back edges from different shards by source txn index
        let mut dependent_edges_by_source_index = HashMap::new();
        for (_, dependent_edges) in dependent_edges_vec.into_iter().enumerate() {
            for dependent_edge in dependent_edges {
                let source_index = dependent_edge.source_txn_index;
                let dep_edges_for_idx = dependent_edges_by_source_index
                    .entry(source_index)
                    .or_insert_with(CrossShardEdges::default);
                for (dependent_idx, storage_locations) in dependent_edge.dependent_edges.into_iter()
                {
                    dep_edges_for_idx.add_edge(dependent_idx, storage_locations);
                }
            }
        }
        // sort the back edges by source txn index and return a vector
        let mut dep_edges_vec = dependent_edges_by_source_index.into_iter().collect_vec();
        dep_edges_vec.sort_by_key(|(source_index, _)| *source_index);
        dep_edges_vec
    }

    fn add_dependent_edges_to_sub_blocks(
        &mut self,
        dependent_edges: Vec<(TxnIndex, CrossShardEdges)>,
    ) {
        let mut current_sub_block_index = 0;
        let mut current_sub_block = self.froze_sub_blocks.get_sub_block_mut(0).unwrap();
        // Since the dependent edges are sorted by source txn index, we can iterate through the sub blocks and add the back edges to the sub blocks
        for (source_index, dependent_edges) in dependent_edges.into_iter() {
            while source_index >= current_sub_block.end_index() {
                current_sub_block_index += 1;
                current_sub_block = self
                    .froze_sub_blocks
                    .get_sub_block_mut(current_sub_block_index)
                    .unwrap();
            }

            for (dependent_idx, storage_locations) in dependent_edges.into_iter() {
                current_sub_block.add_dependent_edge(
                    source_index,
                    dependent_idx,
                    storage_locations,
                );
            }
        }
    }

    pub fn into_frozen_sub_blocks(self) -> SubBlocksForShard {
        self.froze_sub_blocks
    }
}
