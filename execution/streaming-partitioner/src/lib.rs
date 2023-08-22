// Copyright © Aptos Foundation

pub mod transaction_graph_partitioner;

use aptos_graphs::partitioning::PartitionId;
use aptos_transaction_orderer::common::PTransaction;
use aptos_types::batched_stream::BatchedStream;

/// Indicates the position of the transaction in the serialization order of the block.
pub type SerializationIdx = u64;

/// A transaction with its dependencies, serialization index, and partition.
pub struct PartitionedTransaction<T> {
    pub transaction: T,
    pub serialization_idx: SerializationIdx,
    pub partition: PartitionId,
    pub dependencies: Vec<SerializationIdx>,
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

    fn partition_transactions(&mut self, transactions: S) -> Result<Self::ResultStream, Self::Error>;
}
