// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Copyright © Aptos Foundation

use crate::{
    graph::{EdgeWeight, NodeIndex, NodeWeight},
    graph_stream::{BatchInfo, GraphStream, StreamNode},
    partitioning::{PartitionId, StreamingGraphPartitioner},
};
use aptos_types::batched_stream::BatchedStream;
use aptos_logger::info;

/// The type used to represent real numbers in this implementation.
/// For simplicity, it is a fixed type and not a generic parameter.
pub type Real = f64;

/// Compile-time assertion that `NodeWeight` can be converted to [`Real`].
pub const NODE_WEIGHT_MUST_BE_CONVERTIBLE_TO_FENNEL_REAL: () = assert_into::<NodeWeight, Real>();

/// Compile-time assertion that `EdgeWeight` can be converted to [`Real`].
pub const EDGE_WEIGHT_MUST_BE_CONVERTIBLE_TO_FENNEL_REAL: () = assert_into::<EdgeWeight, Real>();

const fn assert_into<From: Into<To>, To>() {}

#[derive(Clone, Copy, PartialEq)]
pub enum BalanceConstraintMode {
    /// Fixed maximum per-partition load.
    FixedMaxLoad(NodeWeight),

    /// Enforce the balancing constraint on each prefix of nodes in the stream.
    ///
    /// I.e., recompute the max allowed per-partition load after receiving each graph node.
    /// Presumably, provides the worst quality partitions, but does not require any
    /// global information about the graph.
    Prefix,

    /// Enforce the balancing constraint on each prefix of batches in the stream.
    ///
    /// I.e., recompute the max allowed per-partition load after receiving each batch
    /// of the graph nodes. Presumably, provides better quality partitions than `Prefix`,
    /// by requires `BatchInfo` returned from `next_batch()` to contain the total node
    /// weight of the batch.
    Batched,

    /// Enforce the balancing constraint only on the entire partitioning.
    ///
    /// I.e., compute the max allowed per-partition load once, for the whole graph.
    /// Presumably, provides the best quality partitions, but requires the total node
    /// weight of the graph to be known before any batches are received, which may
    /// be not realistic for some applications.
    Global,
}

#[derive(Clone, Copy, PartialEq)]
pub enum AlphaComputationMode {
    /// Fixed alpha.
    Fixed(f64),

    /// Compute alpha based on the previous partitioned prefix.
    ///
    /// I.e., recompute alpha after receiving each graph node.
    /// Presumably, provides the worst quality partitions, but does not require any
    /// global information about the graph.
    Prefix,

    /// Compute alpha based on the previously partitioned prefix and the current batch.
    ///
    /// I.e., recompute alpha after receiving each batch of the graph nodes.
    /// Presumably, provides better quality partitions than `Prefix`,
    /// by requires `BatchInfo` returned from `next_batch()` to contain the total node
    /// and edge weight of the batch.
    Batched,

    /// Compute alpha once, for the whole graph.
    ///
    /// I.e., compute alpha once, for the whole graph.
    /// Presumably, provides the best quality partitions, but requires the total node
    /// and edge weight of the graph to be known before any batches are received, which may
    /// be not realistic for some applications.
    Global,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Failed to satisfy the balancing constraint")]
    BalancingConstraint,

    #[error("Cannot compute alpha. Total edge weight is unknown")]
    AlphaUnknownTotalEdgeWeight,

    #[error("Cannot compute alpha. Total node weight is unknown")]
    AlphaUnknownTotalNodeWeight,

    #[error("Cannot compute alpha. Total batch node weight is unknown")]
    AlphaUnknownBatchNodeWeight,

    #[error("Cannot compute alpha. Total batch edge weight is unknown")]
    AlphaUnknownBatchEdgeWeight,

    #[error("Cannot compute max load. Total node weight is unknown")]
    MaxLoadUnknownNodeWeight,

    #[error("Cannot compute max load. Total batch node weight is unknown")]
    MaxLoadUnknownBatchNodeWeight,

    #[error("Error in the input to the partitioner: {0}")]
    InputError(anyhow::Error),
}

type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Copy)]
pub struct FennelGraphPartitioner {
    /// The number of partitions.
    pub n_partitions: usize,

    /// The maximum allowed load imbalance.
    /// The load of each partition must be at most `(1 + balance_constraint) * (n / k)`,
    /// where n is the number of nodes in the graph and k is the number of partitions.
    ///
    /// Must be at least 0.
    /// Default value: 0.1.
    pub balance_constraint: f64,

    /// The exponent of the load balancing score function.
    ///
    /// Must be at least 1.
    /// Default value: 1.5.
    pub gamma: f64,

    /// The mode in which the balancing constraint is enforced.
    ///
    /// See: `BalanceConstraintMode`.
    /// Default value: `BalanceConstraintMode::Global`.
    pub balance_constraint_mode: BalanceConstraintMode,

    /// The mode in which parameter alpha is computer.
    /// See page 4 of the Fennel whitepaper for details.
    ///
    /// See: `AlphaComputationMode`.
    /// Default value: `AlphaComputationMode::Global`.
    pub alpha_computation_mode: AlphaComputationMode,
}

impl FennelGraphPartitioner {
    pub fn new(n_partitions: usize) -> Self {
        let balance_constraint = std::env::var("FENNEL_BALANCE_CONSTRAINT").map(|v|v.parse::<f64>().unwrap_or(0.1)).unwrap_or(0.1);
        let gamma = 1.5;
        info!("Creating FennelGraphPartitioner with balance_constraint={}, gamma={}", balance_constraint, gamma);
        Self {
            n_partitions,
            balance_constraint,
            gamma,
            balance_constraint_mode: BalanceConstraintMode::Prefix,
            alpha_computation_mode: AlphaComputationMode::Prefix,
        }
    }
}

impl<S> StreamingGraphPartitioner<S> for FennelGraphPartitioner
where
    S: GraphStream,
    S::Error: Into<anyhow::Error>,
{
    type Error = Error;
    type ResultStream = FennelStream<S>;

    fn partition_stream(&self, graph_stream: S) -> Result<FennelStream<S>> {
        FennelStream::new(graph_stream, *self)
    }
}

impl FennelGraphPartitioner {
    fn max_load(&self, total_node_weight: NodeWeight, n_partitions: usize) -> NodeWeight {
        let total_node_weight: f64 = total_node_weight.into();
        let n_partitions = n_partitions as f64;
        (total_node_weight / n_partitions * (1. + self.balance_constraint)) as NodeWeight
    }

    fn alpha(
        &self,
        total_node_weight: NodeWeight,
        total_edge_weight: EdgeWeight,
        n_partitions: usize,
    ) -> f64 {
        // See: page 4 of the Fennel paper.
        // This formula is generalized for weighted graphs.
        let k = n_partitions as f64;
        let total_node_weight: f64 = total_node_weight.into();
        let total_edge_weight: f64 = total_edge_weight.into();
        total_edge_weight * k.powf(self.gamma - 1.) / total_node_weight.powf(self.gamma)
    }
}

pub struct FennelStream<S> {
    /// The `GraphStream` to be partitioned, or [`None`] if the stream has
    /// ended or partitioning cannot be continued due to an error.
    graph_stream: Option<S>,

    /// The partitioning parameters.
    params: FennelGraphPartitioner,

    /// The load of each partition.
    load: Vec<NodeWeight>,

    /// The partition of each node, if it has been assigned one.
    partition: Vec<Option<PartitionId>>,

    /// The total weight of the partitioned nodes.
    partitioned_node_weight: NodeWeight,

    /// The total weight of the edges connecting the partitioned nodes.
    /// An edge is *not* counted if it connects a partitioned node to a non-partitioned one.
    partitioned_edge_weight: EdgeWeight,

    /// The alpha parameter, if set globally.
    /// See: `AlphaComputationMode`.
    alpha: Option<f64>,

    /// The max load parameter, if set globally.
    /// See: `BalanceConstraintMode`.
    max_load: Option<NodeWeight>,
}

impl<S> FennelStream<S>
where
    S: GraphStream,
{
    fn new(graph_stream: S, params: FennelGraphPartitioner) -> Result<Self> {
        assert!(params.gamma >= 1.);
        assert!(params.balance_constraint > 0.);
        assert!(params.n_partitions > 0);

        let alpha = match params.alpha_computation_mode {
            AlphaComputationMode::Global => {
                let Some(total_edge_weight) = graph_stream.opt_total_edge_weight() else {
                    return Err(Error::AlphaUnknownTotalEdgeWeight);
                };

                let Some(total_node_weight) = graph_stream.opt_total_node_weight() else {
                    return Err(Error::AlphaUnknownTotalNodeWeight);
                };

                Some(params.alpha(total_node_weight, total_edge_weight, params.n_partitions))
            },
            AlphaComputationMode::Fixed(alpha) => Some(alpha),
            _ => None,
        };

        let max_load = match params.balance_constraint_mode {
            BalanceConstraintMode::Global => {
                let Some(total_node_weight) = graph_stream.opt_total_node_weight() else {
                    return Err(Error::MaxLoadUnknownNodeWeight);
                };

                Some(params.max_load(total_node_weight, params.n_partitions))
            },
            BalanceConstraintMode::FixedMaxLoad(max_load) => Some(max_load),
            _ => None,
        };

        let opt_total_node_count = graph_stream.opt_total_node_count();

        Ok(Self {
            graph_stream: Some(graph_stream),
            load: vec![0 as NodeWeight; params.n_partitions],
            params,
            partition: opt_total_node_count
                .map(|node_count| vec![None; node_count])
                .unwrap_or(Vec::new()),
            partitioned_node_weight: 0 as NodeWeight,
            partitioned_edge_weight: 0 as EdgeWeight,
            alpha,
            max_load,
        })
    }

    /// Returns the partition of the given node, if it has been assigned one.
    fn partition_of(&self, node: NodeIndex) -> Option<PartitionId> {
        self.partition
            .get(node as usize)
            .and_then(|&opt_partition| opt_partition)
    }

    #[allow(clippy::needless_lifetimes)] // Suppressed a false positive warning.
    /// Partitions the given batch of nodes.
    fn partition_batch<'a>(
        &mut self,
        batch: S::Batch<'a>,
        batch_info: BatchInfo,
    ) -> Result<Vec<(StreamNode<S>, PartitionId)>> {
        let mut alpha = self.alpha;
        let mut max_load = self.max_load;

        if self.params.alpha_computation_mode == AlphaComputationMode::Batched {
            let Some(batch_node_weight) = batch_info.opt_total_batch_node_weight else {
                return Err(Error::AlphaUnknownBatchNodeWeight);
            };
            let total_node_weight = self.partitioned_node_weight + batch_node_weight;

            let Some(batch_edge_weight) = batch_info.opt_total_batch_edge_weight else {
                return Err(Error::AlphaUnknownBatchEdgeWeight);
            };
            let total_edge_weight = self.partitioned_edge_weight + batch_edge_weight;

            alpha = Some(self.params.alpha(
                total_node_weight,
                total_edge_weight,
                self.params.n_partitions,
            ));
        }

        if self.params.balance_constraint_mode == BalanceConstraintMode::Batched {
            let Some(batch_node_weight) = batch_info.opt_total_batch_node_weight else {
                return Err(Error::MaxLoadUnknownBatchNodeWeight);
            };
            let total_node_weight = self.partitioned_node_weight + batch_node_weight;

            max_load = Some(
                self.params
                    .max_load(total_node_weight, self.params.n_partitions),
            );
        }

        batch
            .into_iter()
            .map(|(node, edges): (StreamNode<S>, _)| {
                // Allocate more space if necessary.
                if node.index as usize >= self.partition.len() {
                    // NB: Depending on the implementation, `resize` may lead to quadratic
                    // time complexity. However, Rust's implementation does not suffer from
                    // this problem.
                    self.partition.resize(node.index as usize + 1, None);
                }

                // It is important to update it in the beginning as we may later use it
                // to compute `max_load` and `alpha`.
                self.partitioned_node_weight += node.weight;

                let mut edges_to = vec![0 as EdgeWeight; self.params.n_partitions];
                for (v, w) in edges {
                    if let Some(partition) = self.partition_of(v) {
                        edges_to[partition as usize] += w;

                        // We only account for each edge weight once, when we consider the later
                        // of its ends in the stream.
                        self.partitioned_edge_weight += w;
                    }
                }

                // Compute the max allowed per-partition load, if it hasn't been computed yet
                // (see: `BalanceConstraintMode`).
                let max_load = max_load.unwrap_or_else(|| {
                    self.params
                        .max_load(self.partitioned_node_weight, self.params.n_partitions)
                });

                // Compute the alpha parameter if it hasn't been computed yet.
                // (see: `AlphaComputationMode`).
                let alpha = alpha.unwrap_or_else(|| {
                    self.params.alpha(
                        self.partitioned_node_weight,
                        self.partitioned_edge_weight,
                        self.params.n_partitions,
                    )
                });

                let score = |delta_e: EdgeWeight, load: NodeWeight| {
                    let delta_e: f64 = delta_e.into();
                    let old_load: f64 = load.into();
                    let new_load: f64 = (load + node.weight).into();
                    let gamma = self.params.gamma;

                    delta_e - alpha * (new_load.powf(gamma) - old_load.powf(gamma))
                };

                // Chooses the best partition by maximizing the score function.
                // If `apply_balance_constraint` is false, the balancing constraint is ignored.
                let choose = |apply_balance_constraint| {
                    (edges_to.iter().copied())
                        .zip(self.load.iter().copied())
                        .enumerate()
                        .filter(|&(_partition, (_delta_e, load))| {
                            if apply_balance_constraint {
                                load + node.weight < max_load
                            } else {
                                true
                            }
                        })
                        .map(|(partition, (delta_e, load))| (partition, score(delta_e, load)))
                        .max_by(|(_, score1), (_, score2)| score1.partial_cmp(score2).unwrap())
                        .map(|(partition, _)| partition)
                };

                // First, try to choose with the balancing constraint.
                // If it fails, this means that the balancing constraint is too strict
                // and should be ignored.
                // In this case, choose with the balancing constraint turned off, which
                // is guaranteed to succeed.
                let choice = choose(true).unwrap_or_else(|| choose(false).unwrap());

                self.partition[node.index as usize] = Some(choice as PartitionId);
                self.load[choice] += node.weight;

                Ok((node, choice as PartitionId))
            })
            .collect()
    }
}

impl<S> BatchedStream for FennelStream<S>
where
    S: GraphStream,
    S::Error: Into<anyhow::Error>,
{
    type Batch = Vec<Self::StreamItem>;
    type Error = Error;
    type StreamItem = (StreamNode<S>, PartitionId);

    fn next_batch(&mut self) -> Option<Result<Vec<(StreamNode<S>, PartitionId)>>> {
        // Take ownership of `self.graph_stream` to avoid borrowing it mutably.
        let mut graph_stream = self.graph_stream.take()?;

        // In case of `None` or an error, `self.graph_stream` remains `None`,
        // indicating the end of the stream.
        let (batch, batch_info) = match graph_stream.next_batch()? {
            Ok(ret) => ret,
            Err(err) => return Some(Err(Error::InputError(err.into()))),
        };

        match self.partition_batch(batch, batch_info) {
            Ok(res) => {
                // Return the ownership of `graph_stream` back to `self`.
                self.graph_stream = Some(graph_stream);
                Some(Ok(res))
            },
            Err(err) => {
                // `self.graph_stream` stays `None`, indicating that partitioning
                // cannot be continued.
                Some(Err(err))
            },
        }
    }

    fn opt_items_count(&self) -> Option<usize> {
        let Some(graph_stream) = &self.graph_stream else {
            // `None` indicates the end of the stream.
            return Some(0);
        };
        graph_stream.opt_remaining_node_count()
    }

    fn opt_batch_count(&self) -> Option<usize> {
        let Some(graph_stream) = &self.graph_stream else {
            // `None` indicates the end of the stream.
            return None;
        };
        graph_stream.opt_remaining_node_count()
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        graph_stream::input_order_stream,
        partitioning::{fennel::FennelGraphPartitioner, StreamingGraphPartitioner},
        test_utils::simple_four_nodes_two_partitions_graph,
    };
    use aptos_types::batched_stream::BatchedStream;

    #[test]
    fn simple_four_nodes_two_partitions_test() {
        let graph = simple_four_nodes_two_partitions_graph();

        let graph_stream = input_order_stream(&graph, 1);

        let mut partitioner = FennelGraphPartitioner::new(2);
        partitioner.balance_constraint = 0.2;

        let partition_stream = partitioner.partition_stream(graph_stream).unwrap();

        let mut partition_iter = partition_stream.into_items_iter();

        // The first node may be sent to any partition, depending on the implementation.
        let (node, first_partition) = partition_iter.next().unwrap().unwrap();
        assert_eq!(node.index, 0);

        // The second node must be sent to the other partition to satisfy the balancing constraint.
        let (node, partition) = partition_iter.next().unwrap().unwrap();
        assert_eq!(node.index, 1);
        assert_eq!(partition, 1 - first_partition);

        // The third node must be sent to the same partition as the first one
        // due to a heavy edge between them.
        let (node, partition) = partition_iter.next().unwrap().unwrap();
        assert_eq!(node.index, 2);
        assert_eq!(partition, first_partition);

        // Finally, the fourth node must be sent to the same partition as the second node
        // as it has equal weight edges to both partitions, but the second one is less loaded.
        let (node, partition) = partition_iter.next().unwrap().unwrap();
        assert_eq!(node.index, 3);
        assert_eq!(partition, 1 - first_partition);

        assert!(partition_iter.next().is_none());
    }
}
