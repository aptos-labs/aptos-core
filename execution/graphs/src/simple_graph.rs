// Copyright © Aptos Foundation

use std::iter::Sum;
use crate::graph::{Graph, NodeIndex, WeightedEdges, WeightedNodes};

/// An undirected graph represented in a simple format, where for each node
/// we store its weight and a list of its neighbours with the corresponding edge weights.
///
/// Used as an example implementation of the `WeightedGraph` trait and
/// for testing of the graph algorithms.
#[derive(Debug, Clone, Hash)]
pub struct SimpleUndirectedGraph<NW, EW> {
    node_weights: Vec<NW>,
    edges: Vec<Vec<(NodeIndex, EW)>>,
}

impl<NW: Copy, EW: Copy> SimpleUndirectedGraph<NW, EW> {
    /// Creates a new empty graph.
    pub fn new() -> Self {
        Self {
            node_weights: Vec::new(),
            edges: Vec::new(),
        }
    }

    /// Adds a node to the graph.
    pub fn add_node(&mut self, weight: NW) -> NodeIndex {
        let node = self.node_weights.len() as NodeIndex;
        self.node_weights.push(weight);
        self.edges.push(Vec::new());
        node
    }

    /// Adds an undirected edge to the graph.
    pub fn add_edge(&mut self, source: NodeIndex, target: NodeIndex, weight: EW) {
        self.edges[source as usize].push((target, weight));
        self.edges[target as usize].push((source, weight));
    }
}

impl<NW, EW> Graph for SimpleUndirectedGraph<NW, EW> {
    type NodeEdgesIter<'a> = std::iter::Map<
        std::slice::Iter<'a, (NodeIndex, EW)>,
        fn(&'a (NodeIndex, EW)) -> NodeIndex,
    >
    where Self: 'a;

    fn node_count(&self) -> usize {
        self.node_weights.len()
    }

    fn edge_count(&self) -> usize {
        self.edges.iter().map(|neighbours| neighbours.len()).sum()
    }

    fn degree(&self, node: NodeIndex) -> usize {
        self.edges[node as usize].len()
    }

    fn edges(&self, node: NodeIndex) -> Self::NodeEdgesIter<'_> {
        self.edges[node as usize].iter().map(|&(v, _)| v)
    }
}

impl<NW, EW> WeightedNodes for SimpleUndirectedGraph<NW, EW>
where
    NW: Sum<NW> + Copy,
{
    type NodeWeight = NW;

    type WeightedNodesIter<'a> = std::iter::Zip<
        std::ops::Range<NodeIndex>,
        std::iter::Copied<std::slice::Iter<'a, NW>>,
    >
    where Self: 'a;

    fn node_weight(&self, node: NodeIndex) -> Self::NodeWeight {
        self.node_weights[node as usize]
    }

    fn weighted_nodes(&self) -> Self::WeightedNodesIter<'_> {
        self.nodes().zip(self.node_weights.iter().copied())
    }
}

impl<NW, EW> WeightedEdges for SimpleUndirectedGraph<NW, EW>
where
    EW: Sum<EW> + Copy,
{
    type EdgeWeight = EW;

    type WeightedNodeEdgesIter<'a> = std::iter::Copied<std::slice::Iter<'a, (NodeIndex, EW)>>
    where Self: 'a;

    fn weighted_edges(&self, node: NodeIndex) -> Self::WeightedNodeEdgesIter<'_> {
        self.edges[node as usize].iter().copied()
    }
}
