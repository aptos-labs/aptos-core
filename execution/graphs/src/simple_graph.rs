// Copyright © Aptos Foundation

use std::iter::Sum;
use crate::graph::{Graph, NodeIndex, WeightedEdges, WeightedNodes};
use crate::graph_stream::{FromGraphStream, GraphStream};

/// A weighted undirected graph represented in a simple format, where for each node
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

    /// Resizes the graph to the given number of nodes.
    /// If the graph is already larger than the given number of nodes, does nothing.
    /// If the graph is smaller than the given number of nodes,
    /// adds new nodes with the given default weight.
    pub fn upsize(&mut self, new_size: usize, default_weight: NW) -> NodeIndex {
        let old_size = self.node_weights.len();
        if new_size > old_size {
            self.node_weights.resize(new_size, default_weight);
            self.edges.resize(new_size, Vec::new());
        }
        old_size as NodeIndex
    }
}

impl<S: GraphStream> FromGraphStream<S> for SimpleUndirectedGraph<S::NodeWeight, S::EdgeWeight>
where
    S::NodeWeight: Copy + Default,
    S::EdgeWeight: Copy + Default,
{
    /// Reconstructs an undirected graph from a `GraphStream`.
    fn from_graph_stream(mut graph_stream: S) -> Self {
        let mut graph = Self {
            node_weights: Vec::new(),
            edges: Vec::new(),
        };

        while let Some((batch, _)) = graph_stream.next_batch() {
            for (node, node_weight, edges) in batch {
                graph.upsize(node as usize + 1, S::NodeWeight::default());
                graph.node_weights[node as usize] = node_weight;
                for (target, edge_weight) in edges {
                    // We only add edges with target >= node to avoid adding the same edge twice.
                    if target <= node {
                        graph.add_edge(node, target, edge_weight);
                    }
                }
            }
        }

        graph
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
