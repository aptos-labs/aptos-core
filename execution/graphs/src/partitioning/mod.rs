// Copyright © Aptos Foundation

pub mod fennel;
pub mod metis;

use crate::graph::NodeIndex;
use crate::graph_stream::{GraphStream, StreamNode};
use crate::simple_graph::SimpleUndirectedGraph;
use aptos_types::batched_stream;
use aptos_types::batched_stream::BatchedStream;
use std::iter::Sum;

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
    fn partition_stream(&self, graph_stream: S, n_partitions: usize) -> Self::ResultStream;
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

/// Converts a `GraphPartitioner` to a `StreamingGraphPartitioner` by reading the whole
/// graph from the stream, reconstructing it in memory and then partitioning it.
/// This is inefficient and is useful mostly for comparing the quality of true
/// streaming algorithms against non-streaming baselines.
pub struct WholeGraphStreamingPartitioner<P> {
    graph_partitioner: P,
}

impl<P> WholeGraphStreamingPartitioner<P> {
    pub fn new(graph_partitioner: P) -> Self {
        Self { graph_partitioner }
    }
}

impl<S, P> StreamingGraphPartitioner<S> for WholeGraphStreamingPartitioner<P>
where
    S: GraphStream,
    S::NodeWeight: Copy + Default + Sum,
    S::EdgeWeight: Copy + Default + Sum,
    P: GraphPartitioner<SimpleUndirectedGraph<S::NodeData, S::NodeWeight, S::EdgeWeight>>,
{
    type Error = P::Error;

    // Outputs a single batch with the whole partitioning result.
    type ResultStream = batched_stream::Once<Vec<(StreamNode<S>, PartitionId)>, Self::Error>;

    fn partition_stream(&self, graph_stream: S, n_partitions: usize) -> Self::ResultStream {
        let graph = graph_stream.collect();
        let partitioning_result = self.graph_partitioner.partition(&graph, n_partitions);
        let nodes = graph.into_nodes();
        batched_stream::once(
            // `collect()` is used purely for simplicity here as the performance
            // of this method is not really important.
            partitioning_result.map(|partitioning| nodes.zip(partitioning).collect()),
        )
    }
}
