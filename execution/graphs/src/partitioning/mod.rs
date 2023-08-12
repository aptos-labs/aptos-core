// Copyright © Aptos Foundation

pub mod metis;

use aptos_types::batched_stream::BatchedStream;
use crate::graph::{NodeIndex, WeightedUndirectedGraph};
use crate::graph_stream::{GraphStreamer, WeightedUndirectedGraphStream};

// In stable Rust, there are no good ways to implement "number" traits.
// Hence, PartitionId is a fixed type alias and not a generic parameter or an associated type.
pub type PartitionId = NodeIndex;

/// A trait for graph partitioners.
pub trait GraphPartitioner<G> {
    type Error;

    /// Assigns each node in the graph to a partition.
    /// Outputs the mapping from node indices to partitions as a vector.
    /// Node i is assigned to partition output[i].
    fn partition(&self, graph: &G, n_partitions: usize) -> Result<Vec<PartitionId>, Self::Error>;
}

/// A trait for streaming graph partitioners.
pub trait StreamingGraphPartitioner {
    type Error;
    type ResultStream: BatchedStream<StreamItem = Result<(NodeIndex, PartitionId), Self::Error>>;

    /// Assigns each node in the graph to a partition.
    /// Outputs a batched stream of node indices with their assigned partitions.
    fn partition_stream<S>(&self, graph_stream: &mut S, n_partitions: usize) -> Self::ResultStream
    where
        S: WeightedUndirectedGraphStream;
}

pub struct OrderedGraphPartitioner<P, O>
{
    streaming_graph_partitioner: P,
    graph_streamer: O,
}

impl<G, P, O> GraphPartitioner<G> for OrderedGraphPartitioner<P, O>
where
    G: WeightedUndirectedGraph,
    O: GraphStreamer<G>,
    P: StreamingGraphPartitioner,
{
    type Error = P::Error;

    fn partition(&self, graph: &G, n_partitions: usize) -> Result<Vec<PartitionId>, Self::Error> {
        let mut graph_stream = self.graph_streamer.stream(graph);
        let partition_stream = self.streaming_graph_partitioner.partition_stream(&mut graph_stream, n_partitions);

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
