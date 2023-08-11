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
