// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Copyright © Aptos Foundation

// For simplicity, `NodeIndex`, `NodeWeight`, and `EdgeWeight` are fixed types
// and not generic parameters or associated types.

/// The index of a node in a graph.
pub type NodeIndex = u32;

/// The weight of a node in a graph.
///
/// For now, `i32` is used for compatibility with the Metis library.
pub type NodeWeight = i32;

/// The weight of an edge in a graph.
///
/// For now, `i32` is used for compatibility with the Metis library.
pub type EdgeWeight = i32;

/// A node in a graph.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Node<Data> {
    pub index: NodeIndex,
    pub data: Data,
    pub weight: NodeWeight,
}

/// Convenience type alias for a node in a graph.
pub type GraphNode<G> = Node<<G as WeightedGraph>::NodeData>;

/// A reference to a node in a graph.
pub struct NodeRef<'a, Data> {
    pub index: NodeIndex,
    pub data: &'a Data,
    pub weight: NodeWeight,
}

/// Convenience type alias for a reference to a node in a graph.
pub type GraphNodeRef<'a, G> = NodeRef<'a, <G as WeightedGraph>::NodeData>;

/// A simple trait for a weighted undirected graph with arbitrary data associated with nodes.
pub trait WeightedGraph {
    /// The data associated with a node.
    type NodeData;

    /// An iterator over the nodes of the graph.
    type NodesIter<'a>: Iterator<Item = GraphNodeRef<'a, Self>>
    where
        Self: 'a;

    /// An iterator over the neighbours of a node in the graph.
    type NodeEdgesIter<'a>: Iterator<Item = NodeIndex>
    where
        Self: 'a;

    /// An iterator over the neighbors of a node with their edge weights.
    type WeightedNodeEdgesIter<'a>: Iterator<Item = (NodeIndex, EdgeWeight)>
    where
        Self: 'a;

    /// Returns the number of nodes in the graph.
    fn node_count(&self) -> usize;

    /// Returns the number of edges in the graph.
    ///
    /// Depending on the implementation, may take non-constant time.
    fn edge_count(&self) -> usize;

    /// Returns the node by its index.
    fn get_node(&self, idx: NodeIndex) -> GraphNodeRef<'_, Self>;

    /// Returns the weight of a node.
    fn node_weight(&self, idx: NodeIndex) -> NodeWeight {
        self.get_node(idx).weight
    }

    /// Returns the data associated with a node.
    fn node_data(&self, idx: NodeIndex) -> &Self::NodeData {
        self.get_node(idx).data
    }

    /// Returns the degree of a node.
    fn degree(&self, node: NodeIndex) -> usize;

    /// Returns an iterator over the neighbors of a node.
    ///
    /// The graph is assumed to be undirected, meaning that if node `v` is present in `u.edges()`
    /// k times, then `u` must be present in `v.edges()` exactly k times as well.
    fn edges(&self, node: NodeIndex) -> Self::NodeEdgesIter<'_>;

    /// Returns an iterator over the nodes of the graph with their weights.
    ///
    /// Must output the same nodes in the same order as `self.nodes()`, but with their weights.
    fn weighted_nodes(&self) -> Self::NodesIter<'_>;

    /// Returns an iterator over the neighbors of a node with their edge weights.
    ///
    /// Must return the same edges in the same order as `self.edges(node)`, but with their weights.
    /// Depending on the implementation, may be less efficient than `self.edges(node)`.
    fn weighted_edges(&self, node: NodeIndex) -> Self::WeightedNodeEdgesIter<'_>;

    /// A convenience function that returns the range of node indices.
    /// Must be equivalent to 0..self.node_count().
    fn nodes(&self) -> std::ops::Range<NodeIndex> {
        0..self.node_count() as NodeIndex
    }

    /// Returns the total weight of all nodes in the graph.
    /// Depending on the implementation, may take non-constant time.
    ///
    /// The default implementation iterates over all nodes in the graph and sums up their weights.
    fn total_node_weight(&self) -> NodeWeight {
        self.weighted_nodes().map(|node| node.weight).sum()
    }

    /// Returns the total weight of all edges in the graph.
    /// Depending on the implementation, may take non-constant time.
    ///
    /// The default implementation iterates over all nodes in the graph and sums up their
    /// edge weights, counting each edge only once, when it goes from a node with the higher
    /// index to a node with the lower index.
    fn total_edge_weight(&self) -> EdgeWeight {
        self.nodes()
            .flat_map(|u| {
                self.weighted_edges(u)
                    .filter(move |&(v, _)| v <= u)
                    .map(|(_, weight)| weight)
            })
            .sum()
    }
}
