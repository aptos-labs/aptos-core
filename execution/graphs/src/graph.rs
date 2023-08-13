// Copyright © Aptos Foundation

// In stable Rust, there are no good ways to implement "number" traits.
// Hence, NodeIndex is a fixed type alias and not a generic parameter or an associated type.
pub type NodeIndex = u32;

/// A simple trait for an undirected graph.
pub trait UndirectedGraph {
    /// An iterator over the neighbours of a node in the graph.
    type NeighboursIter<'a>: Iterator<Item = NodeIndex>
    where
        Self: 'a;

    /// Returns the number of nodes in the graph.
    fn node_count(&self) -> usize;

    /// Returns the number of edges in the graph.
    fn edge_count(&self) -> usize;

    /// Returns the degree of a node.
    fn degree(&self, node: NodeIndex) -> usize;

    /// Returns an iterator over the neighbors of a node.
    fn edges(&self, node: NodeIndex) -> Self::NeighboursIter<'_>;

    /// A convenience function that returns the range of node indices.
    /// Must be equivalent to 0..self.node_count().
    fn nodes(&self) -> std::ops::Range<NodeIndex> {
        0..self.node_count() as NodeIndex
    }
}

// A trait for an undirected graph with weighted nodes.
pub trait WeightedNodes: UndirectedGraph {
    /// The weight of a node.
    type NodeWeight;

    /// An iterator over the nodes of the graph with their weights.
    type WeightedNodesIter<'a>: Iterator<Item = (NodeIndex, Self::NodeWeight)>
    where
        Self: 'a;

    /// Returns the weight of a node.
    fn node_weight(&self, node: NodeIndex) -> Self::NodeWeight;

    /// Returns the total weight of all nodes in the graph.
    /// Depending on the implementation, may take non-constant time.
    fn total_node_weight(&self) -> Self::NodeWeight;

    /// Returns an iterator over the nodes of the graph with their weights.
    fn weighted_nodes(&self) -> Self::WeightedNodesIter<'_>;
}

// A trait for an undirected graph with weighted edges.
pub trait WeightedEdges: UndirectedGraph {
    /// The weight of an edge.
    type EdgeWeight;

    /// An iterator over the neighbors of a node with their edge weights.
    type WeightedNeighboursIter<'a>: Iterator<Item = (NodeIndex, Self::EdgeWeight)>
    where
        Self: 'a;

    fn total_edge_weight(&self) -> Self::EdgeWeight;

    fn weighted_edges(&self, node: NodeIndex) -> Self::WeightedNeighboursIter<'_>;
}

/// A trait for an undirected graph with weighted nodes and edges.
pub trait WeightedUndirectedGraph: WeightedNodes + WeightedEdges {}

impl<G> WeightedUndirectedGraph for G where G: WeightedNodes + WeightedEdges {}

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

impl<G> UndirectedGraph for TriviallyWeightedGraph<G>
where
    G: UndirectedGraph,
{
    type NeighboursIter<'a> = G::NeighboursIter<'a>
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

    fn edges(&self, node: NodeIndex) -> Self::NeighboursIter<'_> {
        self.graph.edges(node)
    }
}

impl<G> WeightedNodes for TriviallyWeightedGraph<G>
where
    G: UndirectedGraph,
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
    G: UndirectedGraph,
{
    type EdgeWeight = usize;
    type WeightedNeighboursIter<'a> =
        std::iter::Map<G::NeighboursIter<'a>, fn(NodeIndex) -> (NodeIndex, usize)>
        where
            G: 'a;

    fn total_edge_weight(&self) -> Self::EdgeWeight {
        self.edge_count()
    }

    fn weighted_edges(&self, node: NodeIndex) -> Self::WeightedNeighboursIter<'_> {
        self.graph.edges(node).map(|neighbour| (neighbour, 1))
    }
}
