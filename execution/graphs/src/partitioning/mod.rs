// Copyright © Aptos Foundation

pub mod fennel;
pub mod metis;

use crate::graph::{Graph, NodeIndex, WeightedGraph};
use crate::graph_stream::{GraphStream, GraphStreamer};
use crate::simple_graph::SimpleUndirectedGraph;
use aptos_types::batched_stream;
use aptos_types::batched_stream::BatchedStream;

// In stable Rust, there are no good ways to implement "number" traits.
// Hence, PartitionId is a fixed type alias and not a generic parameter or an associated type.
pub type PartitionId = NodeIndex;

/// A trait for streaming graph partitioners.
pub trait StreamingGraphPartitioner<NW, EW> {
    type Error;

    type ResultStream<S>: BatchedStream<StreamItem = (NodeIndex, PartitionId), Error = Self::Error>
    where
        S: GraphStream<NodeWeight = NW, EdgeWeight = EW>;

    /// Assigns each node in the graph to a partition.
    /// Outputs a batched stream of node indices with their assigned partitions.
    fn partition_stream<S>(&self, graph_stream: S, n_partitions: usize) -> Self::ResultStream<S>
    where
        S: GraphStream<NodeWeight = NW, EdgeWeight = EW>;
}

/// A trait for graph partitioners.
pub trait GraphPartitioner<G> {
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

impl<NW, EW, P> StreamingGraphPartitioner<NW, EW> for WholeGraphStreamingPartitioner<P>
where
    NW: Copy + Default,
    EW: Copy + Default,
    P: GraphPartitioner<SimpleUndirectedGraph<NW, EW>>,
{
    type Error = P::Error;
    // Outputs a single batch with the whole partitioning result.
    type ResultStream<S> = batched_stream::Once<Vec<(NodeIndex, PartitionId)>, Self::Error>
    where
        S: GraphStream<NodeWeight = NW, EdgeWeight = EW>;

    fn partition_stream<S>(&self, graph_stream: S, n_partitions: usize) -> Self::ResultStream<S>
    where
        S: GraphStream<NodeWeight = NW, EdgeWeight = EW>,
    {
        let graph: SimpleUndirectedGraph<_, _> = graph_stream.collect();
        let partitioning_result = self.graph_partitioner.partition(&graph, n_partitions);
        batched_stream::once(
            // `collect()` is used purely for simplicity here as the performance
            // of this method is not really important.
            partitioning_result.map(|partitioning| graph.nodes().zip(partitioning).collect()),
        )
    }
}

/// Converts a `StreamingGraphPartitioner` to a `GraphPartitioner` by streaming the graph
/// using the provided `GraphStreamer`.
pub struct OrderedGraphPartitioner<P, S> {
    streaming_graph_partitioner: P,
    graph_streamer: S,
}

impl<P, S> OrderedGraphPartitioner<P, S> {
    pub fn new(streaming_graph_partitioner: P, graph_streamer: S) -> Self {
        Self {
            streaming_graph_partitioner,
            graph_streamer,
        }
    }
}

impl<G, P, S> GraphPartitioner<G> for OrderedGraphPartitioner<P, S>
where
    G: WeightedGraph,
    S: GraphStreamer<G>,
    P: StreamingGraphPartitioner<G::NodeWeight, G::EdgeWeight>,
{
    type Error = P::Error;

    fn partition(&self, graph: &G, n_partitions: usize) -> Result<Vec<PartitionId>, Self::Error> {
        let mut graph_stream = self.graph_streamer.stream(graph);
        let partition_stream = self
            .streaming_graph_partitioner
            .partition_stream(&mut graph_stream, n_partitions);

        let mut partitioning = vec![0; graph.node_count()];

        for res in partition_stream.into_items_iter() {
            match res {
                Ok((node, partition)) => {
                    partitioning[node as usize] = partition;
                },
                Err(err) => {
                    return Err(err);
                },
            }
        }

        Ok(partitioning)
    }
}
