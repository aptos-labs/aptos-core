// Copyright © Aptos Foundation

use crate::graph::NodeIndex;
use crate::graph_stream::GraphStream;
use crate::partitioning::{PartitionId, StreamingGraphPartitioner};
use aptos_types::batched_stream::BatchedStream;
use itertools::Itertools;

#[derive(Clone, Copy, PartialEq, Eq, Ord, PartialOrd, Hash)]
pub enum BalanceConstraintMode {
    /// Enforce the balancing constraint on each prefix of the stream.
    Prefix,
    /// Enforce the balancing constraint only on the entire partitioning.
    Global,
}

#[derive(Clone, Copy, PartialEq, Eq, Ord, PartialOrd, Hash)]
pub enum AlphaComputationMode {
    /// Compute alpha for each prefix of the stream.
    Prefix,
    /// Compute alpha once, for the whole graph.
    Global,
}

#[derive(Debug, thiserror::Error, Clone, Copy)]
pub enum Error {
    #[error("Failed to satisfy the balancing constraint")]
    BalancingConstraint,

    #[error("Cannot compute alpha. Total edge weight is unknown")]
    AlphaUnknownEdgeWeight,

    #[error("Cannot compute alpha. Total node weight is unknown")]
    AlphaUnknownNodeWeight,

    #[error("Cannot compute max load. Total node weight is unknown")]
    MaxLoadUnknownNodeWeight,
}

type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Copy)]
pub struct FennelGraphPartitioner {
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

impl Default for FennelGraphPartitioner {
    fn default() -> Self {
        Self {
            balance_constraint: 0.1,
            gamma: 1.5,
            balance_constraint_mode: BalanceConstraintMode::Prefix,
            alpha_computation_mode: AlphaComputationMode::Prefix,
        }
    }
}

impl<NW, EW> StreamingGraphPartitioner<NW, EW> for FennelGraphPartitioner
where
    NW: Into<f64>,
    EW: Into<f64>,
{
    type Error = Error;
    type ResultStream<S> = FennelStream<S>
    where
        S: GraphStream<NodeWeight = NW, EdgeWeight = EW>;

    fn partition_stream<S>(&self, graph_stream: S, n_partitions: usize) -> FennelStream<S>
    where
        S: GraphStream<NodeWeight = NW, EdgeWeight = EW>,
    {
        FennelStream::new(graph_stream, self.clone(), n_partitions)
    }
}

impl FennelGraphPartitioner {
    fn max_load(&self, total_node_weight: impl Into<f64>, n_partitions: usize) -> f64 {
        let total_node_weight = total_node_weight.into();
        total_node_weight / (n_partitions as f64) * (1. + self.balance_constraint)
    }

    fn alpha(
        &self,
        total_edge_weight: impl Into<f64>,
        total_node_weight: impl Into<f64>,
        n_partitions: usize,
    ) -> f64 {
        // See: page 4 of the Fennel paper.
        // This formula is generalized for weighted graphs.
        let k = n_partitions as f64;
        let total_edge_weight = total_edge_weight.into();
        let total_node_weight = total_node_weight.into();
        total_edge_weight * k.powf(self.gamma - 1.) / total_node_weight.powf(self.gamma)
    }
}

pub struct FennelStream<S> {
    /// The `GraphStream` to be partitioned.
    ///
    /// `None` indicates that the partitioning has failed and shouldn't be continued.
    graph_stream: Option<S>,

    /// Last error, if any.
    err: Option<Error>,

    /// The partitioning parameters.
    params: FennelGraphPartitioner,

    /// The number of partitions.
    n_partitions: usize,

    /// The load of each partition.
    load: Vec<f64>,

    /// The partition of each node, if it has been assigned one.
    partition: Vec<Option<PartitionId>>,

    /// The total weight of the partitioned nodes.
    partitioned_node_weight: f64,

    /// The total weight of the edges connecting the partitioned nodes.
    /// An edge is *not* counted if it connects a partitioned node to a non-partitioned one.
    partitioned_edge_weight: f64,

    /// The alpha parameter, if set globally.
    /// See: `AlphaComputationMode`.
    alpha: Option<f64>,

    /// The max load parameter, if set globally.
    /// See: `BalanceConstraintMode`.
    max_load: Option<f64>,
}

impl<S> FennelStream<S>
where
    S: GraphStream,
    S::NodeWeight: Into<f64>,
    S::EdgeWeight: Into<f64>,
{
    fn new(graph_stream: S, params: FennelGraphPartitioner, n_partitions: usize) -> Self {
        assert!(params.gamma >= 1.);
        assert!(params.balance_constraint > 0.);

        let opt_total_node_count = graph_stream.opt_total_node_count();

        let build_res = |graph_stream, init_err, alpha, max_load| Self {
            graph_stream,
            err: init_err,
            params,
            n_partitions,
            load: vec![0.; n_partitions],
            partition: opt_total_node_count
                .map(|node_count| vec![None; node_count])
                .unwrap_or(Vec::new()),
            partitioned_node_weight: 0.,
            partitioned_edge_weight: 0.,
            alpha,
            max_load,
        };

        let alpha = if params.alpha_computation_mode == AlphaComputationMode::Global {
            let Some(total_edge_weight) = graph_stream.opt_total_edge_weight() else {
                return build_res(None, Some(Error::AlphaUnknownEdgeWeight), None, None);
            };

            let Some(total_node_weight) = graph_stream.opt_total_node_weight() else {
                return build_res(None, Some(Error::AlphaUnknownNodeWeight), None, None);
            };

            Some(params.alpha(total_edge_weight, total_node_weight, n_partitions))
        } else {
            None
        };

        let max_load = if params.balance_constraint_mode == BalanceConstraintMode::Global {
            let Some(total_node_weight) = graph_stream.opt_total_node_weight() else {
                return build_res(None, Some(Error::MaxLoadUnknownNodeWeight), None, None);
            };

            Some(params.max_load(total_node_weight, n_partitions))
        } else {
            None
        };

        build_res(Some(graph_stream), None, alpha, max_load)
    }

    /// Returns the partition of the given node, if it has been assigned one.
    fn partition_of(&self, node: NodeIndex) -> Option<PartitionId> {
        self.partition
            .get(node as usize)
            .and_then(|&opt_partition| opt_partition)
    }

    #[allow(clippy::needless_lifetimes)] // Suppressed a false positive warning.
    /// Partitions the given batch of nodes.
    fn partition<'a>(&mut self, batch: S::Batch<'a>) -> Result<Vec<(NodeIndex, PartitionId)>> {
        batch
            .into_iter()
            .map(|(node, node_weight, node_edges)| {
                // Convert the node weight to f64.
                let node_weight = node_weight.into();

                // It is important to update it in the beginning as we may later use it
                // to compute `max_load` and `alpha`.
                self.partitioned_node_weight += node_weight;

                let mut edges_to = vec![0.; self.n_partitions];
                for (v, w) in node_edges {
                    if let Some(partition) = self.partition_of(v) {
                        let w = w.into();
                        edges_to[partition as usize] += w;

                        // We only account for each edge weight once, when we consider the later
                        // of its ends in the stream.
                        self.partitioned_edge_weight += w;
                    }
                }

                // Compute the max allowed per-partition load, if it hasn't been computed globally,
                // for the whole graph (see: `BalanceConstraintMode`).
                let max_load = self.max_load.unwrap_or_else(|| {
                    self.params
                        .max_load(self.partitioned_node_weight, self.n_partitions)
                });

                // Compute the alpha parameter, if it hasn't been computed globally,
                // for the whole graph (see: `AlphaComputationMode`).
                let alpha = self.alpha.unwrap_or_else(|| {
                    self.params.alpha(
                        self.partitioned_edge_weight,
                        self.partitioned_node_weight,
                        self.n_partitions,
                    )
                });

                // Chooses the best partition by maximizing the score function.
                // If `apply_balance_constraint` is false, the balancing constraint is ignored.
                let choose = |apply_balance_constraint| {
                    let filter_predicate = |&(_, load): &(_, f64)| {
                        if apply_balance_constraint {
                            load < max_load
                        } else {
                            true
                        }
                    };

                    (edges_to.iter().copied())
                        .zip(self.load.iter().copied())
                        .filter(filter_predicate)
                        .map(|(delta_e, old_load)| {
                            let gamma = self.params.gamma;
                            delta_e - alpha * ((old_load + 1.).powf(gamma) - old_load.powf(gamma))
                        })
                        .position_max_by(|x, y| x.partial_cmp(y).unwrap())
                };

                // First, try to choose with the balancing constraint.
                // If it fails, this means that the balancing constraint is too strict
                // and should be ignored.
                // In this case, choose with the balancing constraint turned off, which
                // is guaranteed to succeed.
                let choice = choose(true).unwrap_or_else(|| choose(false).unwrap());

                self.partition[node as usize] = Some(choice as PartitionId);
                self.load[choice] += node_weight;

                Ok((node, choice as PartitionId))
            })
            .collect()
    }
}

impl<S> BatchedStream for FennelStream<S>
where
    S: GraphStream,
    S::NodeWeight: Into<f64>,
    S::EdgeWeight: Into<f64>,
{
    type StreamItem = (NodeIndex, PartitionId);
    type Batch = Vec<Self::StreamItem>;
    type Error = Error;

    fn next_batch(&mut self) -> Option<Result<Vec<(NodeIndex, PartitionId)>>> {
        // If there was an error during initialization, return it.
        if let Some(err) = &self.err {
            return Some(Err(err.clone()));
        }

        // Temporarily take ownership of `graph_stream` to avoid borrowing `self` mutably.
        let mut graph_stream = self.graph_stream.take()?;
        // Partition the next batch.
        let res = graph_stream.next_batch().map(|batch| self.partition(batch));
        // Return the ownership of `graph_stream` back to `self`.
        self.graph_stream = Some(graph_stream);

        res
    }

    fn opt_items_count(&self) -> Option<usize> {
        self.graph_stream
            .as_ref()
            .and_then(|graph_stream| graph_stream.opt_remaining_node_count())
    }

    fn opt_batch_count(&self) -> Option<usize> {
        self.graph_stream
            .as_ref()
            .and_then(|graph_stream| graph_stream.opt_remaining_batch_count())
    }
}
