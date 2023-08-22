// Copyright © Aptos Foundation

use crate::graph_stream::StreamNode;
use crate::partitioning::{GraphPartitioner, PartitionId, StreamingGraphPartitioner};
use crate::{GraphStream, SimpleUndirectedGraph};
use aptos_types::batched_stream;
use std::iter::Sum;

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
    // NB: Using `anyhow::Error` for simplicity here.
    type Error = Error<S::Error, P::Error>;

    // Outputs a single batch with the whole partitioning result.
    type ResultStream = batched_stream::Once<Vec<(StreamNode<S>, PartitionId)>, Self::Error>;

    fn partition_stream(
        &self,
        graph_stream: S,
        n_partitions: usize,
    ) -> Result<Self::ResultStream, Self::Error> {
        let graph = graph_stream
            .collect()
            .map_err(|err: S::Error| Error::GraphStreamError(err))?;

        let partitioning = self
            .graph_partitioner
            .partition(&graph, n_partitions)
            .map_err(|err| Error::GraphPartitionerError(err))?;

        let nodes = graph.into_nodes();
        Ok(batched_stream::once(
            // `collect()` is used purely for simplicity here as the performance
            // of this method is not really important.
            Ok(nodes.zip(partitioning).collect()),
        ))
    }
}

/// An error type for `WholeGraphStreamingPartitioner`.
#[derive(Debug, thiserror::Error)]
pub enum Error<SE, PE> {
    GraphStreamError(SE),
    GraphPartitionerError(PE),
}
