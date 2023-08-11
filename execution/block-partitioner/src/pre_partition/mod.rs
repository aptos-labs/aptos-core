// Copyright © Aptos Foundation

use crate::v2::types::OriginalTxnIdx;
use aptos_types::{
    block_executor::partitioner::ShardId, transaction::analyzed_transaction::AnalyzedTransaction,
};

pub trait PrePartitioner: Send {
    fn pre_partition(
        &self,
        transactions: &[AnalyzedTransaction],
        num_shards: usize,
    ) -> Vec<Vec<OriginalTxnIdx>>;
}

pub mod uniform;

pub fn start_txn_idxs(pre_partitioned: &Vec<Vec<OriginalTxnIdx>>) -> Vec<OriginalTxnIdx> {
    let num_shards = pre_partitioned.len();
    let mut ret: Vec<OriginalTxnIdx> = vec![0; num_shards];
    for shard_id in 1..num_shards {
        ret[shard_id] = ret[shard_id - 1] + pre_partitioned[shard_id - 1].len();
    }
    ret
}
