// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::sharded_block_partitioner::cross_shard_messages::{
    CrossShardClientInterface, CrossShardDependentEdges,
};
use aptos_types::{
    block_executor::partitioner::{
        CrossShardDependencies, CrossShardEdges, ShardId, ShardedTxnIndex, SubBlocksForShard,
        TxnIndex,
    },
    transaction::analyzed_transaction::AnalyzedTransaction,
};
use itertools::Itertools;
use std::{collections::HashMap, sync::Arc};

pub struct DependentEdgeCreator {
    shard_id: ShardId,
    cross_shard_client: Arc<dyn CrossShardClientInterface>,
    froze_sub_blocks: SubBlocksForShard<AnalyzedTransaction>,
    num_shards: usize,
    round_id: usize,
}

/// Creates a list of dependent edges for each sub block in the current round. It works in following steps
/// 1. For the current block, it creates a dependent edge list by txn index based on newly required edges in cross shard
/// dependencies. Dependent edge is a reverse of required edge, for example if txn 20 in shard 2 requires txn 10 in shard 1,
/// then txn 10 in shard 1 will have a dependent edge to txn 20 in shard 2.
/// 2. It sends the dependent edge list to all shards and collects the dependent edge list from all shards.
/// 3. It groups the dependent edge list by source txn index.
/// 4. It adds the dependent edge list to the sub blocks in the current round.
///
impl DependentEdgeCreator {
    pub fn new(
        shard_id: ShardId,
        cross_shard_client: Arc<dyn CrossShardClientInterface>,
        froze_sub_blocks: SubBlocksForShard<AnalyzedTransaction>,
        num_shards: usize,
        round_id: usize,
    ) -> Self {
        Self {
            shard_id,
            cross_shard_client,
            froze_sub_blocks,
            num_shards,
            round_id,
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
                ShardedTxnIndex::new(dependent_index, self.shard_id, self.round_id),
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

    pub fn into_frozen_sub_blocks(self) -> SubBlocksForShard<AnalyzedTransaction> {
        self.froze_sub_blocks
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        sharded_block_partitioner::{
            cross_shard_messages::{CrossShardDependentEdges, MockCrossShardClient},
            dependent_edges::DependentEdgeCreator,
        },
        test_utils::create_non_conflicting_p2p_transaction,
    };
    use aptos_types::{
        block_executor::partitioner::{
            CrossShardDependencies, CrossShardEdges, ShardedTxnIndex, SubBlock, SubBlocksForShard,
            TransactionWithDependencies,
        },
        transaction::analyzed_transaction::StorageLocation,
    };
    use itertools::Itertools;
    use std::sync::Arc;

    #[test]
    fn test_create_dependent_edges() {
        let shard_id = 0;
        let start_index = 0;
        let num_shards = 3;
        let round_id = 999;
        let mut transactions_with_deps = Vec::new();
        for _ in 0..10 {
            transactions_with_deps.push(TransactionWithDependencies::new(
                create_non_conflicting_p2p_transaction(),
                CrossShardDependencies::default(),
            ));
        }

        // cross shard dependent edges from shard 1
        let mut dependent_edges_from_shard_1 = vec![];
        let txn_4_storgae_location: Vec<StorageLocation> =
            transactions_with_deps[4].txn.write_hints().to_vec();
        let txn_5_storgae_location: Vec<StorageLocation> =
            transactions_with_deps[5].txn.write_hints().to_vec();
        // Txn 11 is dependent on Txn 4
        dependent_edges_from_shard_1.push(CrossShardDependentEdges::new(
            4,
            CrossShardEdges::new(
                ShardedTxnIndex::new(11, 1, round_id),
                txn_4_storgae_location.clone(),
            ),
        ));
        // Txn 12 is dependent on Txn 5
        dependent_edges_from_shard_1.push(CrossShardDependentEdges::new(
            5,
            CrossShardEdges::new(
                ShardedTxnIndex::new(12, 1, round_id),
                txn_5_storgae_location.clone(),
            ),
        ));

        // cross shard dependent edges from shard 2
        let dependent_edges_shard_2 = vec![
            // Txn 21 is dependent on Txn 4
            CrossShardDependentEdges::new(
                4,
                CrossShardEdges::new(
                    ShardedTxnIndex::new(21, 2, round_id),
                    txn_4_storgae_location.clone(),
                ),
            ),
            // Txn 22 is dependent on Txn 5
            CrossShardDependentEdges::new(
                5,
                CrossShardEdges::new(
                    ShardedTxnIndex::new(22, 2, round_id),
                    txn_5_storgae_location.clone(),
                ),
            ),
        ];

        let cross_shard_client = Arc::new(MockCrossShardClient {
            rw_set_results: vec![],
            write_set_with_index_results: vec![],
            num_accepted_txns_results: vec![],
            dependent_edges_results: vec![dependent_edges_from_shard_1, dependent_edges_shard_2],
        });

        let mut sub_blocks = SubBlocksForShard::empty(shard_id);
        let sub_block = SubBlock::new(
            start_index,
            transactions_with_deps
                .iter()
                .map(|txn_with_deps| {
                    TransactionWithDependencies::new(
                        txn_with_deps.txn.clone(),
                        txn_with_deps.cross_shard_dependencies.clone(),
                    )
                })
                .collect_vec(),
        );
        sub_blocks.add_sub_block(sub_block);

        let mut dependent_edge_creator = DependentEdgeCreator::new(
            shard_id,
            cross_shard_client,
            sub_blocks,
            num_shards,
            round_id,
        );

        dependent_edge_creator.create_dependent_edges(&[], 0);

        let sub_blocks_with_dependent_edges = dependent_edge_creator.into_frozen_sub_blocks();
        assert_eq!(sub_blocks_with_dependent_edges.num_sub_blocks(), 1);
        let sub_block = sub_blocks_with_dependent_edges.get_sub_block(0).unwrap();
        assert_eq!(sub_block.num_txns(), 10);

        let dependent_storage_locs = sub_block.transactions_with_deps()[4]
            .cross_shard_dependencies
            .get_dependent_edge_for(ShardedTxnIndex::new(11, 1, round_id))
            .unwrap();
        assert_eq!(dependent_storage_locs, &txn_4_storgae_location);

        let dependent_storage_locs = sub_block.transactions_with_deps()[5]
            .cross_shard_dependencies
            .get_dependent_edge_for(ShardedTxnIndex::new(12, 1, round_id))
            .unwrap();
        assert_eq!(dependent_storage_locs, &txn_5_storgae_location);

        let dependent_storage_locs = sub_block.transactions_with_deps()[4]
            .cross_shard_dependencies
            .get_dependent_edge_for(ShardedTxnIndex::new(21, 2, round_id))
            .unwrap();
        assert_eq!(dependent_storage_locs, &txn_4_storgae_location);

        let dependent_storage_locs = sub_block.transactions_with_deps()[5]
            .cross_shard_dependencies
            .get_dependent_edge_for(ShardedTxnIndex::new(22, 2, round_id))
            .unwrap();
        assert_eq!(dependent_storage_locs, &txn_5_storgae_location);
    }
}
