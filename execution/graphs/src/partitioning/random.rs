// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Copyright © Aptos Foundation

use crate::{
    graph_stream::StreamNode,
    partitioning::{PartitionId, StreamingGraphPartitioner},
    GraphStream,
};
use aptos_types::batched_stream::BatchedStream;
use rand::Rng;

/// Graph partitioner that assigns transactions to pseudo-random partitions, based on their IDs.
/// Useful as a baseline for comparison against more sophisticated partitioners.
#[derive(Copy, Clone, Debug)]
pub struct RandomPartitioner {
    n_partitions: usize,
}

impl RandomPartitioner {
    pub fn new(n_partitions: usize) -> Self {
        Self { n_partitions }
    }
}

impl<S> StreamingGraphPartitioner<S> for RandomPartitioner
where
    S: GraphStream,
{
    type Error = S::Error;
    type ResultStream = RandomPartitionerStream<S>;

    fn partition_stream(&self, graph_stream: S) -> Result<Self::ResultStream, Self::Error> {
        Ok(RandomPartitionerStream::new(
            graph_stream,
            self.n_partitions,
        ))
    }
}

pub struct RandomPartitionerStream<S> {
    graph_stream: S,
    n_partitions: usize,
}

impl<S> RandomPartitionerStream<S> {
    pub fn new(graph_stream: S, n_partitions: usize) -> Self {
        Self {
            graph_stream,
            n_partitions,
        }
    }
}

impl<S> BatchedStream for RandomPartitionerStream<S>
where
    S: GraphStream,
{
    type Batch = Vec<Self::StreamItem>;
    type Error = S::Error;
    type StreamItem = (StreamNode<S>, PartitionId);

    fn next_batch(&mut self) -> Option<Result<Self::Batch, Self::Error>> {
        self.graph_stream.next_batch().map(|batch_or_err| {
            batch_or_err.map(|(batch, _info)| {
                batch
                    .into_iter()
                    .map(|(node, _edges): (StreamNode<S>, _)| {
                        let partition = rand::thread_rng().gen_range(0, self.n_partitions);
                        (node, partition as PartitionId)
                    })
                    .collect()
            })
        })
    }

    fn opt_items_count(&self) -> Option<usize> {
        self.graph_stream.opt_remaining_node_count()
    }

    fn opt_batch_count(&self) -> Option<usize> {
        self.graph_stream.opt_remaining_batch_count()
    }
}
