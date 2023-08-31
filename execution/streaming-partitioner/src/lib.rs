// Copyright © Aptos Foundation

pub mod transaction_graph_partitioner;

use std::collections::HashMap;
use aptos_graphs::partitioning::PartitionId;
use aptos_transaction_orderer::common::PTransaction;
use aptos_types::batched_stream::BatchedStream;

/// Indicates the position of the transaction in the serialization order of the block.
pub type SerializationIdx = u32;

/// A transaction with its dependencies, serialization index, and partition.
#[derive(Clone, Debug)]
pub struct PartitionedTransaction<T: PTransaction> {
    pub transaction: T,
    pub serialization_idx: SerializationIdx,
    pub partition: PartitionId,
    pub dependencies: HashMap<SerializationIdx, Vec<T::Key>>,
}

/// A trait for streaming transaction partitioners.
pub trait StreamingTransactionPartitioner<S>
where
    S: BatchedStream,
    S::StreamItem: PTransaction,
{
    /// The error type returned by the partitioner.
    type Error;

    type ResultStream: BatchedStream<
        StreamItem = PartitionedTransaction<S::StreamItem>,
        Error = Self::Error,
    >;

    fn partition_transactions(
        &mut self,
        transactions: S,
    ) -> Result<Self::ResultStream, Self::Error>;
}
