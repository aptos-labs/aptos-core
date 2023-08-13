// Copyright © Aptos Foundation

use crate::graph::{NodeIndex, WeightedUndirectedGraph};
use crate::graph_stream::WeightedUndirectedGraphStream;
use crate::partitioning::{PartitionId, StreamingGraphPartitioner};
use nonmax::NonMaxUsize;
use rand::seq::SliceRandom;
use aptos_types::batched_stream::{BatchedStream, BatchIterator, ItemsIterator};

#[derive(Clone, Copy)]
pub enum BalanceConstraintMode {
    /// Enforce the balancing constraint on each prefix of the stream.
    Prefix,
    /// Enforce the balancing constraint only on the entire partitioning.
    Global,
}

#[derive(Clone, Copy)]
pub enum AlphaComputationMode {
    /// Compute alpha for each prefix of the stream.
    Prefix,
    /// Compute alpha once, for the whole graph.
    Global,
}

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
    type ResultStream<'s, S> = FennelStream<'s, S>
    where
        Self: 's,
        S: 's;

    fn partition_stream<'s, S>(
        &self,
        graph_stream: &'s mut S,
        n_partitions: usize,
    ) -> FennelStream<'s, S>
    where
        S: WeightedUndirectedGraphStream<NodeWeight = NW, EdgeWeight = EW>,
    {
        FennelStream {
            graph_stream,
            params: self.clone(),
            n_partitions,
        }
    }
}

pub struct FennelStream<'s, S> {
    graph_stream: &'s mut S,
    params: FennelGraphPartitioner,
    n_partitions: usize,
    load: Vec<usize>,
    partition: Vec<Option<NonMaxUsize>>,
    partitioned_node_count: usize,
}

impl<'s, S> BatchedStream for FennelStream<'s, S>
where
    S: WeightedUndirectedGraphStream,
    S::NodeWeight: Into<f64>,
    S::EdgeWeight: Into<f64>
{
    type StreamItem = (NodeIndex, PartitionId);
    type BatchIter = std::vec::IntoIter<Self::StreamItem>;

    fn next_batch(&mut self) -> Option<Self::BatchIter> {
        self.graph_stream.next_batch().map(|batch| {
            self.partitioned_node_count += 1;

            let mut edges_to = vec![0.; k];
            for (v, w) in graph.edges(node) {
                if let Some(partition) = partition[v] {
                    edges_to[partition.get()] += w.into();
                }
            }

            let max_load = match self.balance_constraint_mode {
                BalanceConstraintMode::Prefix => self.max_load(partitioned_nodes, n_partitions),
                BalanceConstraintMode::Global => self.max_load(graph.node_count(), n_partitions),
            };

            let choice = edges_to
                .into_iter()
                .zip(load.iter().copied())
                .filter(|(_, load)| *load < max_load)
                .map(|(delta_e, old_load)| {
                    let old_load = old_load as f64;
                    delta_e - alpha * ((old_load + 1.).powf(gamma) - old_load.powf(gamma))
                })
                .position_max_by(|x, y| x.partial_cmp(y).unwrap());

            // TODO: match choice

            partition[node] = Some(NonMaxUsize::new(choice).unwrap());
            load[choice] += 1;
        });

        todo!()
    }

    fn opt_len(&self) -> Option<usize> {
        self.graph_stream.opt_remaining_node_count()
    }

    fn opt_batch_count(&self) -> Option<usize> {
        self.graph_stream.opt_remaining_batch_count()
    }
}

impl<'s, S> FennelStream<'s, S>
where
    S: WeightedUndirectedGraphStream,
    S::NodeWeight: Into<f64>,
    S::EdgeWeight: Into<f64>,
{
    fn max_load(&self, total_node_weight: f64, n_partitions: usize) -> usize {
        let eps = self.params.balance_constraint;
        (total_node_weight / (n_partitions as f64) * (1. + eps)).ceil() as usize
    }

    fn alpha(gamma: f64, total_edge_weight: f64, total_node_weight: f64, n_partitions: usize) -> f64 {
        // See: page 4 of the Fennel paper.
        // This formula is generalized for weighted graphs.
        let k = n_partitions as f64;
        total_edge_weight * k.powf(gamma - 1.) / total_node_weight.powf(gamma)
    }
}
