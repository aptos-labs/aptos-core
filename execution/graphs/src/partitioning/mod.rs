// Copyright © Aptos Foundation

pub mod metis;
pub mod fennel;

use crate::graph::{NodeIndex, WeightedGraph};
use crate::graph_stream::{GraphStreamer, GraphStream};
use aptos_types::batched_stream::BatchedStream;

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
pub trait StreamingGraphPartitioner<NW, EW> {
    type Error;

    type ResultStream<S>: BatchedStream<StreamItem = (NodeIndex, PartitionId), Error = Self::Error>
    where
        S: GraphStream<NodeWeight = NW, EdgeWeight = EW>;

    /// Assigns each node in the graph to a partition.
    /// Outputs a batched stream of node indices with their assigned partitions.
    fn partition_stream<S>(
        &self,
        graph_stream: S,
        n_partitions: usize,
    ) -> Self::ResultStream<S>
    where
        S: GraphStream<NodeWeight = NW, EdgeWeight = EW>;
}

pub struct OrderedGraphPartitioner<P, S> {
    streaming_graph_partitioner: P,
    graph_streamer: S,
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
