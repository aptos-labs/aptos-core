// Copyright © Aptos Foundation

use crate::graph::{UndirectedGraph, WeightedUndirectedGraph};
use crate::partitioning::{GraphPartitioner, PartitionId};

/// A weighted undirected graph in the format expected by the Metis library.
struct MetisGraph {
    /// The adjacency lists of all vertices concatenated together.
    /// As each edge connects two vertices, `adjncy.len()` must be equal to `2m`,
    /// where `m` is the number of edges.
    adjncy: Vec<metis::Idx>,

    /// The index of the start of each adjacency list in `adjncy`
    /// with the last element equal to `adjncy.len()`.
    /// `xadj.len()` must be equal to `n + 1`, where `n` is the number of vertices.
    xadj: Vec<metis::Idx>,

    /// The weight of each vertex.
    /// If `None`, all vertices are assumed to have unit weight.
    /// Otherwise, `vwgt.unwrap().len()` must be equal to `n`,
    /// where `n` is the number of vertices.
    vwgt: Option<Vec<metis::Idx>>,

    /// The weight of each edge, in the same order as in `adjncy`.
    /// If `None`, all edges are assumed to have unit weight.
    /// Otherwise, `adjwgt.unwrap().len()` must be equal to `2m`,
    /// where `m` is the number of edges.
    adjwgt: Option<Vec<metis::Idx>>,
}

/// The partitioning algorithm.
/// See tye Metis manual or whitepaper for details:
/// http://glaros.dtc.umn.edu/gkhome/fetch/sw/metis/manual.pdf
pub enum PartitioningType {
    /// Recursively bi-partitions the graph until the desired number of partitions is reached.
    RecursiveBisection,

    /// Directly partitions the graph into the desired number of partitions.
    KWayPartitioning,
}

/// Metis library graph partitioner wrapper.
///
/// Metis should be installed on the system.
/// Please see the description of the `metis` crate for details:
/// https://crates.io/crates/metis
///
/// Useful links:
///  - Metis homepage: http://glaros.dtc.umn.edu/gkhome/metis/metis/overview
///  - Metis manual: http://glaros.dtc.umn.edu/gkhome/fetch/sw/metis/manual.pdf
pub struct MetisGraphPartitioner {
    /// The maximum allowed load imbalance.
    /// The load of each partition must be at most `(1 + balance_constraint) * (n / k)`,
    /// where n is the number of nodes in the graph and k is the number of partitions.
    ///
    /// Must be greater than 0.
    /// Default value: 0.1.
    pub balance_constraint: f64,

    /// The partitioning algorithm.
    /// See the Metis manual or whitepaper for details.
    ///
    /// Default value: `PartitioningType::KWayPartitioning`.
    pub partitioning_type: PartitioningType,
}

impl Default for MetisGraphPartitioner {
    fn default() -> Self {
        Self {
            balance_constraint: 0.1,
            partitioning_type: PartitioningType::KWayPartitioning,
        }
    }
}

impl MetisGraph {
    fn unweighted<G: UndirectedGraph>(graph: &G) -> Self {
        // adjncy is the concatenation of the adjacency lists of all vertices.
        let adjncy: Vec<_> = graph
            .nodes()
            .flat_map(|u| graph.edges(u).map(|v| v as metis::Idx))
            .collect();

        // xadj is the index of the start of each adjacency list in adjncy.
        // The first element must be 0 and the last must be equal to the length of adjncy.
        let degree_prefix_sum = graph
            .nodes()
            .map(|u| graph.edges(u).count() as metis::Idx)
            .scan(0, |state, x| {
                *state += x;
                Some(*state)
            });
        let xadj: Vec<_> = std::iter::once(0).chain(degree_prefix_sum).collect();

        assert_eq!(xadj.len(), graph.node_count() + 1);
        assert_eq!(xadj[0], 0);
        assert_eq!(xadj[xadj.len() - 1], adjncy.len() as metis::Idx);

        MetisGraph {
            adjncy,
            xadj,
            vwgt: None,
            adjwgt: None,
        }
    }

    fn weighted<G: WeightedUndirectedGraph>(graph: &G) -> Self
    where
        G::NodeWeight: Into<metis::Idx>,
        G::EdgeWeight: Into<metis::Idx>,
    {
        let mut res = Self::unweighted(graph);

        res.vwgt = Some(graph.weighted_nodes().map(|(_, w)| w.into()).collect());

        res.adjwgt = Some(
            graph
                .nodes()
                .flat_map(|u| graph.weighted_edges(u).map(|(_, w)| w.into()))
                .collect(),
        );

        res
    }

    fn node_count(&self) -> usize {
        assert!(self.xadj.len() > 0);
        self.xadj.len() - 1
    }
}

impl MetisGraphPartitioner {
    fn partition(
        &self,
        metis_graph: &mut MetisGraph,
        n_partitions: usize,
    ) -> anyhow::Result<Vec<PartitionId>> {
        let node_count = metis_graph.node_count();

        let mut handle = metis::Graph::new(
            1, // number of balancing constraints.
            n_partitions as metis::Idx,
            &mut metis_graph.xadj,
            &mut metis_graph.adjncy,
        );

        if let Some(vwgt) = &mut metis_graph.vwgt {
            handle = handle.set_vwgt(vwgt);
        }

        if let Some(adjwgt) = &mut metis_graph.adjwgt {
            handle = handle.set_adjwgt(adjwgt);
        }

        let mut ubvec = vec![1. + self.balance_constraint as metis::Real];
        handle = handle.set_ubvec(&mut ubvec);

        let mut res = vec![0; node_count];

        match self.partitioning_type {
            PartitioningType::RecursiveBisection => {
                handle.part_recursive(&mut res)?;
            },
            PartitioningType::KWayPartitioning => {
                handle.part_kway(&mut res)?;
            },
        }

        Ok(res.into_iter().map(|x| x as PartitionId).collect())
    }
}

impl<G: WeightedUndirectedGraph> GraphPartitioner<G> for MetisGraphPartitioner
where
    G::NodeWeight: Into<metis::Idx>,
    G::EdgeWeight: Into<metis::Idx>,
{
    type Error = anyhow::Error;

    /// Partitions the graph using the Metis graph partitioner.
    ///
    /// The partitioning may return an error if it fails to satisfy the balancing constraint
    /// or if the Metis library is not properly installed.
    /// See the description of the `metis` crate for details: https://crates.io/crates/metis
    fn partition(&self, graph: &G, n_partitions: usize) -> anyhow::Result<Vec<PartitionId>> {
        let mut metis_graph = MetisGraph::weighted(graph);
        self.partition(&mut metis_graph, n_partitions)
    }
}
