// Copyright © Aptos Foundation

use std::iter::Sum;

// In stable Rust, there are no good ways to implement "number" traits.
// Hence, NodeIndex is a fixed type alias and not a generic parameter or an associated type.
pub type NodeIndex = u32;

/// A simple trait for an undirected graph.
pub trait Graph {
    /// An iterator over the neighbours of a node in the graph.
    type NodeEdgesIter<'a>: Iterator<Item = NodeIndex>
    where
        Self: 'a;

    /// Returns the number of nodes in the graph.
    fn node_count(&self) -> usize;

    /// Returns the number of edges in the graph.
    ///
    /// Depending on the implementation, may take non-constant time.
    fn edge_count(&self) -> usize;

    /// Returns the degree of a node.
    fn degree(&self, node: NodeIndex) -> usize;

    /// Returns an iterator over the neighbors of a node.
    ///
    /// The graph is assumed to be undirected, meaning that if node `v` is present in `u.edges()`
    /// k times, then `u` must be present in `v.edges()` exactly k times as well.
    fn edges(&self, node: NodeIndex) -> Self::NodeEdgesIter<'_>;

    /// A convenience function that returns the range of node indices.
    /// Must be equivalent to 0..self.node_count().
    fn nodes(&self) -> std::ops::Range<NodeIndex> {
        0..self.node_count() as NodeIndex
    }
}

// A trait for an undirected graph with weighted nodes.
pub trait WeightedNodes: Graph {
    /// The weight of a node.
    type NodeWeight: Sum<Self::NodeWeight>;

    /// An iterator over the nodes of the graph with their weights.
    type WeightedNodesIter<'a>: Iterator<Item = (NodeIndex, Self::NodeWeight)>
    where
        Self: 'a;

    /// Returns the weight of a node.
    fn node_weight(&self, node: NodeIndex) -> Self::NodeWeight;

    /// Returns the total weight of all nodes in the graph.
    /// Depending on the implementation, may take non-constant time.
    ///
    /// The default implementation iterates over all nodes in the graph and sums up their weights.
    fn total_node_weight(&self) -> Self::NodeWeight {
        self.weighted_nodes().map(|(_, weight)| weight).sum()
    }

    /// Returns an iterator over the nodes of the graph with their weights.
    ///
    /// Must output the same nodes in the same order as `self.nodes()`, but with their weights.
    fn weighted_nodes(&self) -> Self::WeightedNodesIter<'_>;
}

// A trait for an undirected graph with weighted edges.
pub trait WeightedEdges: Graph {
    /// The weight of an edge.
    type EdgeWeight: Sum<Self::EdgeWeight>;

    /// An iterator over the neighbors of a node with their edge weights.
    type WeightedNodeEdgesIter<'a>: Iterator<Item = (NodeIndex, Self::EdgeWeight)>
    where
        Self: 'a;

    /// Returns the total weight of all edges in the graph.
    /// Depending on the implementation, may take non-constant time.
    ///
    /// The default implementation iterates over all nodes in the graph and sums up their
    /// edge weights, counting each edge only once, when it goes from a node with the higher
    /// index to a node with the lower index.
    fn total_edge_weight(&self) -> Self::EdgeWeight {
        self.nodes()
            .flat_map(|u| {
                self.weighted_edges(u)
                    .filter(move |&(v, _)| v <= u)
                    .map(|(_, weight)| weight)
            })
            .sum()
    }

    /// Returns an iterator over the neighbors of a node with their edge weights.
    ///
    /// Must return the same edges in the same order as `self.edges(node)`, but with their weights.
    /// Depending on the implementation, may be less efficient than `self.edges(node)`.
    fn weighted_edges(&self, node: NodeIndex) -> Self::WeightedNodeEdgesIter<'_>;
}

/// A trait for an undirected graph with weighted nodes and edges.
pub trait WeightedGraph: WeightedNodes + WeightedEdges {}

impl<G> WeightedGraph for G where G: WeightedNodes + WeightedEdges {}

/// Simple wrapper that makes a weighted undirected graph out of any graph
/// by assigning weight "1" to all nodes and edges.
pub struct TriviallyWeightedGraph<G> {
    graph: G,
}

impl<G> TriviallyWeightedGraph<G> {
    pub fn new(graph: G) -> Self {
        Self { graph }
    }
}

impl<G> Graph for TriviallyWeightedGraph<G>
where
    G: Graph,
{
    type NodeEdgesIter<'a> = G::NodeEdgesIter<'a>
    where
        G: 'a;

    fn node_count(&self) -> usize {
        self.graph.node_count()
    }

    fn edge_count(&self) -> usize {
        self.graph.edge_count()
    }

    fn degree(&self, node: NodeIndex) -> usize {
        self.graph.degree(node)
    }

    fn edges(&self, node: NodeIndex) -> Self::NodeEdgesIter<'_> {
        self.graph.edges(node)
    }
}

impl<G> WeightedNodes for TriviallyWeightedGraph<G>
where
    G: Graph,
{
    type NodeWeight = NodeIndex;

    type WeightedNodesIter<'a> =
        std::iter::Map<std::ops::Range<NodeIndex>, fn(NodeIndex) -> (NodeIndex, Self::NodeWeight)>
    where
        Self: 'a;

    fn node_weight(&self, _node: NodeIndex) -> Self::NodeWeight {
        1
    }

    fn total_node_weight(&self) -> Self::NodeWeight {
        self.node_count() as Self::NodeWeight
    }

    fn weighted_nodes(&self) -> Self::WeightedNodesIter<'_> {
        self.graph.nodes().map(|node| (node, 1 as Self::NodeWeight))
    }
}

impl<G> WeightedEdges for TriviallyWeightedGraph<G>
where
    G: Graph,
{
    type EdgeWeight = usize;
    type WeightedNodeEdgesIter<'a> =
        std::iter::Map<G::NodeEdgesIter<'a>, fn(NodeIndex) -> (NodeIndex, usize)>
        where
            G: 'a;

    fn total_edge_weight(&self) -> Self::EdgeWeight {
        self.edge_count()
    }

    fn weighted_edges(&self, node: NodeIndex) -> Self::WeightedNodeEdgesIter<'_> {
        self.graph.edges(node).map(|neighbour| (neighbour, 1))
    }
}
