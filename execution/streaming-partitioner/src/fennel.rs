// Copyright © Aptos Foundation

use crate::graph::{NodeIndex, WeightedUndirectedGraph};
use crate::graph_partitioner::{GraphPartitioner, PartitionId};
use nonmax::NonMaxUsize;
use rand::seq::SliceRandom;

pub enum StreamOrder {
    Random,
    Input,
    // NB: the paper also considers DFS and BFS orderings, but we don't implement them here.
}

pub enum BalanceConstraintMode {
    /// Enforce the balancing constraint on each prefix of the stream.
    Prefix,
    /// Enforce the balancing constraint only on the entire partitioning.
    Global,
}

pub enum AlphaComputationMode {
    /// Compute alpha for each prefix of the stream.
    Prefix,
    /// Compute alpha once, for the whole graph.
    Global,
}

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

    /// The order in which nodes are streamed to the partitioner.
    ///
    /// Default value: `StreamOrder::Random`.
    pub stream_order: StreamOrder,

    /// The mode in which the balancing constraint is enforced.
    ///
    /// See: `BalanceConstraintMode`.
    /// Default value: `BalanceConstraintMode::Global`.
    pub balance_constraint_mode: BalanceConstraintMode,
}

impl Default for FennelGraphPartitioner {
    fn default() -> Self {
        Self {
            balance_constraint: 0.1,
            gamma: 1.5,
            stream_order: StreamOrder::Random,
            balance_constraint_mode: BalanceConstraintMode::Prefix,
        }
    }
}

impl FennelGraphPartitioner {
    fn max_load(&self, total_node_weight: f64, n_partitions: usize) -> usize {
        (total_node_weight / (n_partitions as f64) * (1. + self.balance_constraint)).ceil() as usize
    }

    fn alpha(gamma: f64, total_edge_weight: f64) -> usize {
        // See: page 4 of the Fennel paper.
        // This formula is generalized for weighted graphs.
        todo!()
    }

    fn partition_impl<G>(
        &self,
        nodes: impl IntoIterator<Item = NodeIndex>,
        graph: &G,
        n_partitions: usize,
    ) -> anyhow::Result<Vec<PartitionId>>
    where
        G: WeightedUndirectedGraph,
        G::NodeWeight: Into<f64>,
        G::EdgeWeight: Into<f64>,
    {
        // See: page 4 of the Fennel paper.
        // However, the original paper considered only unweighted graphs.
        let total_edge_weight: f64 = graph.total_edge_weight().into();
        let alpha = total_edge_weight * (k as f64).powf(gamma - 1.) / (n as f64).powf(gamma);

        let mut load = vec![0; k];
        let mut partition: Vec<Option<NonMaxUsize>> = vec![None; n];

        let mut partitioned_nodes = 0;
        for node in nodes {
            partitioned_nodes += 1;

            let mut edges_to = vec![0.; k];
            for (v, w) in graph.edges(node) {
                if let Some(partition) = partition[v] {
                    edges_to[partition.get()] += w.into();
                }
            }

            let max_load = match self.balance_constraint_mode {
                BalanceConstraintMode::Prefix => {
                    self.max_load(partitioned_nodes, n_partitions)
                },
                BalanceConstraintMode::Global => {
                    self.max_load(graph.node_count(), n_partitions)
                },
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
        }

        assert_eq!(partitioned_nodes, n);
        partition.into_iter().map(|x| x.unwrap().get()).collect()
    }
}

impl<G> GraphPartitioner<G> for FennelGraphPartitioner
where
    G: WeightedUndirectedGraph,
    G::NodeWeight: Into<f64>,
    G::EdgeWeight: Into<f64>,
{
    type Error = anyhow::Error;

    fn partition(&self, graph: &G, n_partitions: usize) -> anyhow::Result<Vec<PartitionId>> {
        assert!(self.gamma > 1.);
        assert!(self.balance_constraint >= 0.);

        match self.stream_order {
            StreamOrder::Random => {
                let mut nodes: Vec<NodeIndex> = graph.nodes().collect();
                let mut rng = rand::thread_rng();
                nodes.shuffle(&mut rng);
                self.partition_impl(nodes, graph, n_partitions)
            },
            StreamOrder::Input => self.partition_impl(graph.nodes(), graph, n_partitions),
        }
    }
}
