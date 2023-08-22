// Copyright © Aptos Foundation

use aptos_types::transaction::analyzed_transaction::AnalyzedTransaction;
use crate::v2::types::TxnIdx1;

pub trait PrePartitioner: Send {
    /// The initial partitioning phase for `ShardedBlockPartitioner`/`PartitionerV2` to divide a block into `num_shards` sub-blocks.
    /// See `PartitionerV2::partition()` for more details.
    fn pre_partition(
        &self,
        transactions: &[AnalyzedTransaction],
        num_shards: usize,
    ) -> Vec<Vec<TxnIdx1>>;
}

pub mod uniform_partitioner;
