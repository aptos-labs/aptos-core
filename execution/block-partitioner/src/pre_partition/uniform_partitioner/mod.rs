// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

#[cfg(test)]
use crate::test_utils::P2PBlockGenerator;
use crate::{
    pre_partition::PrePartitioner,
    v2::{
        state::PartitionState,
        types::{OriginalTxnIdx, PrePartitionedTxnIdx},
    },
};
#[cfg(test)]
use rand::thread_rng;

/// A naive partitioner that evenly divide txns into shards.
/// Example: processing txns 0..11 results in `[[0,1,2,3],[4,5,6,7],[8,9,10]]`.
pub struct UniformPartitioner {}

impl UniformPartitioner {
    fn process(&self, num_txns: usize, num_shards: usize) -> Vec<Vec<PrePartitionedTxnIdx>> {
        let num_chunks = num_shards;
        let num_big_chunks = num_txns % num_chunks;
        let small_chunk_size = num_txns / num_chunks;
        let mut ret = Vec::with_capacity(num_chunks);
        let mut next_chunk_start = 0;
        for chunk_id in 0..num_chunks {
            let extra = if chunk_id < num_big_chunks { 1 } else { 0 };
            let next_chunk_end = next_chunk_start + small_chunk_size + extra;
            let chunk: Vec<usize> = (next_chunk_start..next_chunk_end).collect();
            next_chunk_start = next_chunk_end;
            ret.push(chunk);
        }
        ret
    }
}

impl PrePartitioner for UniformPartitioner {
    fn pre_partition(
        &self,
        state: &PartitionState,
    ) -> (
        Vec<OriginalTxnIdx>,
        Vec<PrePartitionedTxnIdx>,
        Vec<Vec<PrePartitionedTxnIdx>>,
    ) {
        let pre_partitioned = self.process(state.num_txns(), state.num_executor_shards);
        let mut txn_counter = 0;
        let mut start_txn_idxs_by_shard = vec![0; state.num_executor_shards];
        for (shard_id, txns) in pre_partitioned.iter().enumerate() {
            start_txn_idxs_by_shard[shard_id] = txn_counter;
            txn_counter += txns.len();
        }
        let ori_txn_idxs = (0..state.num_txns()).collect();
        (ori_txn_idxs, start_txn_idxs_by_shard, pre_partitioned)
    }
}

#[test]
fn test_uniform_partitioner() {
    let block_gen = P2PBlockGenerator::new(10);
    let mut rng = thread_rng();
    let txns = block_gen.rand_block(&mut rng, 18);
    let partitioner = UniformPartitioner {};
    let actual = partitioner.process(txns.len(), 5);
    assert_eq!(
        vec![4, 4, 4, 3, 3],
        actual.iter().map(|v| v.len()).collect::<Vec<usize>>()
    );
    assert_eq!((0..18).collect::<Vec<usize>>(), actual.concat());

    let actual = partitioner.process(txns.len(), 3);
    assert_eq!(
        vec![6, 6, 6],
        actual.iter().map(|v| v.len()).collect::<Vec<usize>>()
    );
    assert_eq!((0..18).collect::<Vec<usize>>(), actual.concat());
}

pub mod config;
