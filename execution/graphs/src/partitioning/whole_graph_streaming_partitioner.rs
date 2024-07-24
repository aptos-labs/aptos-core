// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Copyright © Aptos Foundation

use crate::{
    graph_stream::StreamNode,
    partitioning::{GraphPartitioner, PartitionId, StreamingGraphPartitioner},
    GraphStream, SimpleUndirectedGraph,
};
use aptos_types::batched_stream;

/// Converts a `GraphPartitioner` to a `StreamingGraphPartitioner` by reading the whole
/// graph from the stream, reconstructing it in memory and then partitioning it.
/// This is inefficient and is useful mostly for comparing the quality of true
/// streaming algorithms against non-streaming baselines.
#[derive(Copy, Clone, Debug)]
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
    P: GraphPartitioner<SimpleUndirectedGraph<S::NodeData>>,
{
    // NB: Using `anyhow::Error` for simplicity here.
    type Error = Error<S::Error, P::Error>;
    // Outputs a single batch with the whole partitioning result.
    type ResultStream = batched_stream::Once<Vec<(StreamNode<S>, PartitionId)>, Self::Error>;

    fn partition_stream(&self, graph_stream: S) -> Result<Self::ResultStream, Self::Error> {
        let graph = graph_stream.collect().map_err(Error::GraphStreamError)?;

        let partitioning = self
            .graph_partitioner
            .partition(&graph)
            .map_err(Error::GraphPartitionerError)?;

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
