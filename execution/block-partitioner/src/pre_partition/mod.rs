// Copyright © Aptos Foundation

use crate::v2::types::PrePartitionedTxnIdx;
use aptos_types::transaction::analyzed_transaction::AnalyzedTransaction;

pub trait PrePartitioner: Send {
    /// The initial partitioning phase for `ShardedBlockPartitioner`/`PartitionerV2` to divide a block into `num_shards` sub-blocks.
    /// See `PartitionerV2::partition()` for more details.
    fn pre_partition(
        &self,
        transactions: &[AnalyzedTransaction],
        num_shards: usize,
    ) -> Vec<Vec<PrePartitionedTxnIdx>>;
}

pub mod uniform_partitioner;

pub fn start_txn_idxs(
    pre_partitioned: &Vec<Vec<PrePartitionedTxnIdx>>,
) -> Vec<PrePartitionedTxnIdx> {
    let num_shards = pre_partitioned.len();
    let mut ret: Vec<PrePartitionedTxnIdx> = vec![0; num_shards];
    for shard_id in 1..num_shards {
        ret[shard_id] = ret[shard_id - 1] + pre_partitioned[shard_id - 1].len();
    }
    ret
}

#[test]
fn test_start_txn_idxs() {
    let pre_partitioned = vec![vec![0, 1], vec![2, 3, 4], vec![5, 6, 7, 8]];
    assert_eq!(vec![0, 2, 5], start_txn_idxs(&pre_partitioned));
}
