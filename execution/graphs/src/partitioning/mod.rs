// Copyright © Aptos Foundation

pub mod metis;

use crate::graph::{NodeIndex, WeightedUndirectedGraph};
use crate::graph_stream::{GraphStreamer, WeightedUndirectedGraphStream};
use aptos_types::batched_stream::BatchedStream;

// In stable Rust, there are no good ways to implement "number" traits.
// Hence, PartitionId is a fixed type alias and not a generic parameter or an associated type.
pub type PartitionId = NodeIndex;

/// A trait for graph partitioners.
pub trait GraphPartitioner<G> {
    /// Assigns each node in the graph to a partition.
    /// Outputs the mapping from node indices to partitions as a vector.
    /// Node i is assigned to partition output[i].
    fn partition(&self, graph: &G, n_partitions: usize) -> anyhow::Result<Vec<PartitionId>>;
}

/// A trait for streaming graph partitioners.
pub trait StreamingGraphPartitioner<NW, EW> {
    type ResultStream<'a, S>: BatchedStream<StreamItem = anyhow::Result<(NodeIndex, PartitionId)>>
    where
        Self: 'a,
        S: 'a;

    /// Assigns each node in the graph to a partition.
    /// Outputs a batched stream of node indices with their assigned partitions.
    fn partition_stream<'s, S>(
        &self,
        graph_stream: &'s mut S,
        n_partitions: usize,
    ) -> Self::ResultStream<'s, S>
    where
        S: WeightedUndirectedGraphStream<NodeWeight = NW, EdgeWeight = EW>;
}

pub struct OrderedGraphPartitioner<P, O> {
    streaming_graph_partitioner: P,
    graph_streamer: O,
}

impl<G, P, O> GraphPartitioner<G> for OrderedGraphPartitioner<P, O>
where
    G: WeightedUndirectedGraph,
    O: GraphStreamer<G>,
    P: StreamingGraphPartitioner<G::NodeWeight, G::EdgeWeight>,
{
    fn partition(&self, graph: &G, n_partitions: usize) -> anyhow::Result<Vec<PartitionId>> {
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
