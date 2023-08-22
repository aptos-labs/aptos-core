// Copyright © Aptos Foundation

pub mod fennel;
pub mod metis;
pub mod whole_graph_streaming_partitioner;

use crate::graph::NodeIndex;
use crate::graph_stream::{GraphStream, StreamNode};
use aptos_types::batched_stream::BatchedStream;

pub use whole_graph_streaming_partitioner::WholeGraphStreamingPartitioner;

// In stable Rust, there are no good ways to implement "number" traits.
// Hence, PartitionId is a fixed type alias and not a generic parameter or an associated type.
pub type PartitionId = NodeIndex;

/// A trait for streaming graph partitioners.
pub trait StreamingGraphPartitioner<S: GraphStream> {
    /// The error type returned by the partitioner.
    type Error;

    type ResultStream: BatchedStream<StreamItem = (StreamNode<S>, PartitionId), Error = Self::Error>;

    /// Assigns each node in the graph to a partition.
    /// Outputs a batched stream of node indices with their assigned partitions.
    fn partition_stream(
        &self,
        graph_stream: S,
        n_partitions: usize,
    ) -> Result<Self::ResultStream, Self::Error>;
}

/// A trait for graph partitioners.
pub trait GraphPartitioner<G> {
    /// The error type returned by the partitioner.
    type Error;

    /// Assigns each node in the graph to a partition.
    /// Outputs the mapping from node indices to partitions as a vector.
    /// Node i is assigned to partition output[i].
    fn partition(&self, graph: &G, n_partitions: usize) -> Result<Vec<PartitionId>, Self::Error>;
}
