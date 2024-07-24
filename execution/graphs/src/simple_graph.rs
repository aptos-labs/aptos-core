// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Copyright © Aptos Foundation

use crate::{
    graph::{EdgeWeight, GraphNode, GraphNodeRef, Node, NodeIndex, NodeRef, NodeWeight},
    graph_stream::{FromGraphStream, GraphStream},
    WeightedGraph,
};

/// A weighted undirected graph represented in a simple format, where for each node
/// we store its weight and a list of its neighbours with the corresponding edge weights.
#[derive(Clone, Debug)]
pub struct SimpleUndirectedGraph<Data> {
    node_weights: Vec<NodeWeight>,
    node_data: Vec<Data>,
    edges: Vec<Vec<(NodeIndex, EdgeWeight)>>,
}

impl<Data> Default for SimpleUndirectedGraph<Data> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Data> SimpleUndirectedGraph<Data> {
    /// Creates a new empty graph.
    pub fn new() -> Self {
        Self {
            node_weights: Vec::new(),
            node_data: Vec::new(),
            edges: Vec::new(),
        }
    }

    /// Adds a node to the graph.
    pub fn add_node(&mut self, data: Data, weight: NodeWeight) -> NodeIndex {
        let idx = self.node_weights.len() as NodeIndex;
        self.node_weights.push(weight);
        self.node_data.push(data);
        self.edges.push(Vec::new());
        idx
    }

    /// Adds an undirected edge to the graph.
    pub fn add_edge(&mut self, source: NodeIndex, target: NodeIndex, weight: EdgeWeight) {
        self.edges[source as usize].push((target, weight));
        self.edges[target as usize].push((source, weight));
    }

    /// Returns an iterator over the nodes of the graph.
    pub fn into_nodes(self) -> impl Iterator<Item = GraphNode<Self>> {
        self.nodes()
            .zip(self.node_data)
            .zip(self.node_weights)
            .map(|((index, data), weight)| Node {
                index,
                data,
                weight,
            })
    }
}

impl<S, Data> FromGraphStream<S> for SimpleUndirectedGraph<Data>
where
    S: GraphStream<NodeData = Data>,
{
    type Error = S::Error;

    /// Reconstructs an undirected graph from a `GraphStream`.
    fn from_graph_stream(mut graph_stream: S) -> Result<Self, S::Error> {
        let mut node_weights: Vec<NodeWeight> = Vec::new();
        let mut node_data: Vec<Option<Data>> = Vec::new();
        let mut graph_edges: Vec<Vec<_>> = Vec::new();

        while let Some(batch_res) = graph_stream.next_batch() {
            let (batch, _batch_info) = batch_res?;

            for (node, edges) in batch {
                if node.index as usize >= node_weights.len() {
                    node_weights.resize(node.index as usize + 1, 0 as NodeWeight);
                    node_data.resize_with(node.index as usize + 1, || None);
                    graph_edges.resize(node.index as usize + 1, Vec::new());
                }

                node_weights[node.index as usize] = node.weight;
                node_data[node.index as usize] = Some(node.data);
                for (target, edge_weight) in edges {
                    // We only add edges with target >= node to avoid adding the same edge twice.
                    if target <= node.index {
                        graph_edges[node.index as usize].push((target, edge_weight));
                        graph_edges[target as usize].push((node.index, edge_weight));
                    }
                }
            }
        }

        Ok(Self {
            node_weights,
            node_data: node_data
                .into_iter()
                .map(|opt_data| opt_data.expect("Missing node in a graph stream"))
                .collect(),
            edges: graph_edges,
        })
    }
}

impl<Data> WeightedGraph for SimpleUndirectedGraph<Data> {
    type NodeData = Data;
    type NodeEdgesIter<'a> = std::iter::Map<
        std::slice::Iter<'a, (NodeIndex, EdgeWeight)>,
        fn(&'a (NodeIndex, EdgeWeight)) -> NodeIndex,
    >
    where Self: 'a;
    type NodesIter<'a> = NodesIter<'a, Data>
    where Self: 'a;
    type WeightedNodeEdgesIter<'a> = std::iter::Copied<std::slice::Iter<'a, (NodeIndex, EdgeWeight)>>
    where Self: 'a;

    fn node_count(&self) -> usize {
        self.node_weights.len()
    }

    fn edge_count(&self) -> usize {
        self.edges.iter().map(|neighbours| neighbours.len()).sum()
    }

    fn get_node(&self, idx: NodeIndex) -> GraphNodeRef<'_, Self> {
        NodeRef {
            index: idx,
            data: &self.node_data[idx as usize],
            weight: self.node_weights[idx as usize],
        }
    }

    fn degree(&self, node: NodeIndex) -> usize {
        self.edges[node as usize].len()
    }

    fn edges(&self, node: NodeIndex) -> Self::NodeEdgesIter<'_> {
        self.edges[node as usize].iter().map(|&(v, _)| v)
    }

    fn weighted_nodes(&self) -> Self::NodesIter<'_> {
        let nodes = self.nodes();
        NodesIter {
            graph: self,
            node_indices: nodes,
        }
    }

    fn weighted_edges(&self, node: NodeIndex) -> Self::WeightedNodeEdgesIter<'_> {
        self.edges[node as usize].iter().copied()
    }
}

pub struct NodesIter<'a, Data> {
    graph: &'a SimpleUndirectedGraph<Data>,
    node_indices: std::ops::Range<NodeIndex>,
}

impl<'a, Data> Iterator for NodesIter<'a, Data> {
    type Item = NodeRef<'a, Data>;

    fn next(&mut self) -> Option<Self::Item> {
        self.node_indices.next().map(|idx| NodeRef {
            index: idx,
            data: &self.graph.node_data[idx as usize],
            weight: self.graph.node_weights[idx as usize],
        })
    }
}
