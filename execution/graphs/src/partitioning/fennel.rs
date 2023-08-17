// Copyright © Aptos Foundation

use crate::graph::NodeIndex;
use crate::graph_stream::{GraphStream, StreamBatchInfo};
use crate::partitioning::{PartitionId, StreamingGraphPartitioner};
use aptos_types::batched_stream::BatchedStream;
use itertools::Itertools;

#[derive(Clone, Copy, PartialEq)]
pub enum BalanceConstraintMode {
    /// Fixed maximum per-partition load.
    FixedMaxLoad(f64),

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

#[derive(Debug, thiserror::Error, Clone, Copy, PartialEq, Eq, Ord, PartialOrd, Hash)]
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
    NW: Into<f64> + Copy,
    EW: Into<f64> + Copy,
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
        total_node_weight: impl Into<f64>,
        total_edge_weight: impl Into<f64>,
        n_partitions: usize,
    ) -> f64 {
        // See: page 4 of the Fennel paper.
        // This formula is generalized for weighted graphs.
        let k = n_partitions as f64;
        let total_node_weight = total_node_weight.into();
        let total_edge_weight = total_edge_weight.into();
        total_edge_weight * k.powf(self.gamma - 1.) / total_node_weight.powf(self.gamma)
    }
}

pub struct FennelStream<S> {
    /// The `GraphStream` to be partitioned.
    ///
    /// `Err` indicates that the partitioning has failed and shouldn't be continued.
    /// The inner `Option` is used to temporarily take ownership of `graph_stream`
    /// to avoid borrowing `self` mutably (see the `next_batch` implementation).
    graph_stream: Result<Option<S>>,

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
    S::NodeWeight: Into<f64> + Copy,
    S::EdgeWeight: Into<f64> + Copy,
{
    fn new(graph_stream: S, params: FennelGraphPartitioner, n_partitions: usize) -> Self {
        assert!(params.gamma >= 1.);
        assert!(params.balance_constraint > 0.);

        let opt_total_node_count = graph_stream.opt_total_node_count();

        let build_res = |graph_stream, alpha, max_load| Self {
            graph_stream,
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

        let alpha = match params.alpha_computation_mode {
            AlphaComputationMode::Global => {
                let Some(total_edge_weight) = graph_stream.opt_total_edge_weight() else {
                    return build_res(Err(Error::AlphaUnknownTotalEdgeWeight), None, None);
                };

                let Some(total_node_weight) = graph_stream.opt_total_node_weight() else {
                    return build_res(Err(Error::AlphaUnknownTotalNodeWeight), None, None);
                };

                Some(params.alpha(total_node_weight, total_edge_weight, n_partitions))
            },
            AlphaComputationMode::Fixed(alpha) => Some(alpha),
            _ => None,
        };

        let max_load = match params.balance_constraint_mode {
            BalanceConstraintMode::Global => {
                let Some(total_node_weight) = graph_stream.opt_total_node_weight() else {
                    return build_res(Err(Error::MaxLoadUnknownNodeWeight), None, None);
                };

                Some(params.max_load(total_node_weight, n_partitions))
            },
            BalanceConstraintMode::FixedMaxLoad(max_load) => Some(max_load),
            _ => None,
        };

        build_res(Ok(Some(graph_stream)), alpha, max_load)
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
        batch_info: StreamBatchInfo<S>,
    ) -> Result<Vec<(NodeIndex, PartitionId)>> {
        let mut alpha = self.alpha;
        let mut max_load = self.max_load;

        if self.params.alpha_computation_mode == AlphaComputationMode::Batched {
            let Some(batch_node_weight) = batch_info.opt_total_batch_node_weight else {
                return Err(Error::AlphaUnknownBatchNodeWeight);
            };
            let total_node_weight = self.partitioned_node_weight + batch_node_weight.into();

            let Some(batch_edge_weight) = batch_info.opt_total_batch_edge_weight else {
                return Err(Error::AlphaUnknownBatchEdgeWeight);
            };
            let total_edge_weight = self.partitioned_edge_weight + batch_edge_weight.into();

            alpha = Some(self.params.alpha(
                total_node_weight,
                total_edge_weight,
                self.n_partitions,
            ));
        }

        if self.params.balance_constraint_mode == BalanceConstraintMode::Batched {
            let Some(batch_node_weight) = batch_info.opt_total_batch_node_weight else {
                return Err(Error::MaxLoadUnknownBatchNodeWeight);
            };
            let total_node_weight = self.partitioned_node_weight + batch_node_weight.into();

            max_load = Some(self.params.max_load(total_node_weight, self.n_partitions));
        }

        batch
            .into_iter()
            .map(|(node, node_weight, node_edges)| {
                // Convert the node weight to f64.
                let node_weight = node_weight.into();

                // Allocate more space if necessary.
                if node as usize >= self.partition.len() {
                    let new_len = (self.partition.len() * 2).max(node as usize + 1).max(16);
                    self.partition.resize(new_len, None);
                }

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

                // Compute the max allowed per-partition load, if it hasn't been computed yet
                // (see: `BalanceConstraintMode`).
                let max_load = max_load.unwrap_or_else(|| {
                    self.params
                        .max_load(self.partitioned_node_weight, self.n_partitions)
                });

                // Compute the alpha parameter if it hasn't been computed yet.
                // (see: `AlphaComputationMode`).
                let alpha = alpha.unwrap_or_else(|| {
                    self.params.alpha(
                        self.partitioned_node_weight,
                        self.partitioned_edge_weight,
                        self.n_partitions,
                    )
                });

                // Chooses the best partition by maximizing the score function.
                // If `apply_balance_constraint` is false, the balancing constraint is ignored.
                let choose = |apply_balance_constraint| {
                    let filter_predicate = |&(_, load): &(_, f64)| {
                        if apply_balance_constraint {
                            load + node_weight < max_load
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
    S::NodeWeight: Into<f64> + Copy,
    S::EdgeWeight: Into<f64> + Copy,
{
    type StreamItem = (NodeIndex, PartitionId);
    type Batch = Vec<Self::StreamItem>;
    type Error = Error;

    fn next_batch(&mut self) -> Option<Result<Vec<(NodeIndex, PartitionId)>>> {
        let mut graph_stream = match &mut self.graph_stream {
            Ok(graph_stream) => {
                // Temporarily take ownership of `graph_stream` to avoid borrowing `self` mutably.
                graph_stream.take().unwrap()
            },
            Err(err) => return Some(Err(err.clone())),
        };

        // Get the next batch and partition it.
        let (batch, batch_info) = graph_stream.next_batch()?;
        match self.partition_batch(batch, batch_info) {
            Ok(batch) => {
                // Return the ownership of `graph_stream` back to `self`.
                self.graph_stream = Ok(Some(graph_stream));
                Some(Ok(batch))
            },
            Err(err) => {
                self.graph_stream = Err(err);
                Some(Err(err.clone()))
            },
        }
    }

    fn opt_items_count(&self) -> Option<usize> {
        let Ok(graph_stream) = &self.graph_stream else {
            return None;
        };
        graph_stream.as_ref().unwrap().opt_remaining_node_count()
    }

    fn opt_batch_count(&self) -> Option<usize> {
        let Ok(graph_stream) = &self.graph_stream else {
            return None;
        };
        graph_stream.as_ref().unwrap().opt_remaining_node_count()
    }
}

#[cfg(test)]
mod tests {
    use crate::graph_stream::{GraphStreamer, InputOrderGraphStreamer};
    use crate::partitioning::fennel::FennelGraphPartitioner;
    use crate::partitioning::StreamingGraphPartitioner;
    use crate::test_utils::simple_four_nodes_two_partitions_graph;
    use aptos_types::batched_stream::BatchedStream;

    #[test]
    fn simple_four_nodes_two_partitions_test() {
        let graph = simple_four_nodes_two_partitions_graph();

        let graph_streamer = InputOrderGraphStreamer::new(2);

        let mut partitioner = FennelGraphPartitioner::default();
        partitioner.balance_constraint = 0.2;

        let partition_stream = partitioner.partition_stream(graph_streamer.stream(&graph), 2);

        let mut partition_iter = partition_stream.into_items_iter();

        // The first node may be sent to any partition, depending on the implementation.
        let (first_item, first_partition) = partition_iter.next().unwrap().unwrap();
        assert_eq!(first_item, 0);

        // The second node must be sent to the other partition to satisfy the balancing constraint.
        assert_eq!(partition_iter.next(), Some(Ok((1, 1 - first_partition))));

        // The third node must be sent to the same partition as the first one
        // due to a heavy edge between them.
        assert_eq!(partition_iter.next(), Some(Ok((2, first_partition))));

        // Finally, the fourth node must be sent to the same partition as the second node
        // as it has equal weight edges to both partitions, but the second one is less loaded.
        assert_eq!(partition_iter.next(), Some(Ok((3, 1 - first_partition))));
        assert_eq!(partition_iter.next(), None);
    }
}
