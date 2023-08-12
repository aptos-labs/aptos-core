// Copyright © Aptos Foundation

use crate::graph::{NodeIndex, WeightedUndirectedGraph};
use rand::seq::SliceRandom;

/// A trait for batched streams for undirected graphs with weighted nodes and edges.
pub trait WeightedUndirectedGraphStream: Sized {
    /// The weight of a node.
    type NodeWeight;

    /// The weight of an edge.
    type EdgeWeight;

    /// An iterator over the neighbours of a node in the graph.
    type NeighboursIter: Iterator<Item = (NodeIndex, Self::EdgeWeight)>;

    /// An iterator over the nodes in a batch.
    type Batch: IntoIterator<Item = (NodeIndex, Self::NodeWeight, Self::NeighboursIter)>;

    /// Returns the next batch of nodes in the stream.
    /// There should be no edges to nodes in future batches.
    fn next_batch(&mut self) -> Option<Self::Batch>;

    /// Returns the total number of batches remaining in the stream, if available.
    fn opt_remaining_batch_count(&self) -> Option<usize> {
        None
    }

    /// Returns the total number of nodes in all remaining batches of the stream combined,
    /// if available.
    fn opt_remaining_node_count(&self) -> Option<usize> {
        None
    }

    /// Returns the total number of nodes in the whole graph, including already processed batches,
    /// if available.
    fn opt_total_node_count(&self) -> Option<usize> {
        None
    }

    /// Returns the total number of edges in the whole graph, including already processed batches,
    /// if available.
    fn opt_total_edge_count(&self) -> Option<usize> {
        None
    }

    /// Returns the total weight of all nodes in the whole graph,
    /// including already processed batches, if available.
    fn opt_total_node_weight(&self) -> Option<Self::NodeWeight> {
        None
    }

    /// Returns the total weight of all edges in the whole graph,
    /// including already processed batches, if available.
    fn opt_total_edge_weight(&self) -> Option<Self::EdgeWeight> {
        None
    }
}

/// A trait for a generic graph streamer.
pub trait GraphStreamer<G> {
    type Stream<'graph>: WeightedUndirectedGraphStream
    where
        Self: 'graph,
        G: 'graph;

    fn stream<'graph>(&self, graph: &'graph G) -> Self::Stream<'graph>;
}

/// Streams graphs in batches of fixed size, in order from `0` to `node_count() - 1`.
pub struct InputOrderGraphStreamer {
    batch_size: usize,
}

impl InputOrderGraphStreamer {
    pub fn new(batch_size: usize) -> Self {
        Self { batch_size }
    }
}

impl<G: WeightedUndirectedGraph> GraphStreamer<G> for InputOrderGraphStreamer {
    type Stream<'graph> = InputOrderGraphStream<'graph, G>
    where
        G: 'graph;

    fn stream<'graph>(&self, graph: &'graph G) -> Self::Stream<'graph> {
        InputOrderGraphStream::new(graph, self.batch_size)
    }
}

/// Streams graphs in batches of fixed size, in random order.
pub struct RandomOrderGraphStreamer {
    // TODO: add support for custom RNG / seed.

    batch_size: usize,
}

impl RandomOrderGraphStreamer {
    /// Creates a new `RandomOrderGraphStreamer` with the given batch size.
    pub fn new(batch_size: usize) -> Self {
        Self { batch_size }
    }
}

impl<G: WeightedUndirectedGraph> GraphStreamer<G> for RandomOrderGraphStreamer {
    type Stream<'graph> = RandomOrderGraphStream<'graph, G>
    where
        G: 'graph;

    fn stream<'graph>(&self, graph: &'graph G) -> Self::Stream<'graph> {
        RandomOrderGraphStream::new(graph, self.batch_size)
    }
}

/// Streams a graph in batches of fixed size, in order from `0` to `node_count() - 1`.
pub struct InputOrderGraphStream<'graph, G> {
    graph: &'graph G,
    batch_size: usize,
    current_node: NodeIndex,
}

impl<'graph, G: WeightedUndirectedGraph> InputOrderGraphStream<'graph, G> {
    pub fn new(graph: &'graph G, batch_size: usize) -> Self {
        Self {
            graph,
            batch_size,
            current_node: 0,
        }
    }
}

impl<'graph, G> WeightedUndirectedGraphStream for InputOrderGraphStream<'graph, G>
where
    G: WeightedUndirectedGraph,
{
    type NodeWeight = G::NodeWeight;
    type EdgeWeight = G::EdgeWeight;
    type NeighboursIter = G::WeightedNeighboursIter;
    type Batch = Vec<(NodeIndex, Self::NodeWeight, Self::NeighboursIter)>;

    fn next_batch(&mut self) -> Option<Self::Batch> {
        if self.current_node == self.graph.node_count() as NodeIndex {
            return None;
        }

        let batch_start = self.current_node;
        self.current_node = (self.current_node + self.batch_size as NodeIndex)
            .min(self.graph.node_count() as NodeIndex);

        // TODO: consider getting rid of collecting into a vector
        Some((batch_start..self.current_node)
            .map(|node| {
                let node_weight = self.graph.node_weight(node);
                let neighbours = self.graph.weighted_edges(node);
                (node, node_weight, neighbours)
            }).collect())
    }

    fn opt_remaining_batch_count(&self) -> Option<usize> {
        self.opt_remaining_node_count()
            .map(|count| (count + self.batch_size - 1) / self.batch_size)
    }

    fn opt_remaining_node_count(&self) -> Option<usize> {
        Some(self.graph.node_count() - self.current_node as usize)
    }

    fn opt_total_node_count(&self) -> Option<usize> {
        Some(self.graph.node_count())
    }

    fn opt_total_edge_count(&self) -> Option<usize> {
        Some(self.graph.edge_count())
    }

    fn opt_total_node_weight(&self) -> Option<Self::NodeWeight> {
        Some(self.graph.total_node_weight())
    }

    fn opt_total_edge_weight(&self) -> Option<Self::EdgeWeight> {
        Some(self.graph.total_edge_weight())
    }
}

/// Streams a graph in batches of fixed size, in random order.
pub struct RandomOrderGraphStream<'graph, G> {
    graph: &'graph G,
    batch_size: usize,
    order: Vec<NodeIndex>,
    current_node: NodeIndex,
}

impl<'graph, G: WeightedUndirectedGraph> RandomOrderGraphStream<'graph, G> {
    pub fn new(graph: &'graph G, batch_size: usize) -> Self {
        let mut order: Vec<_> = graph.nodes().collect();
        let mut rng = rand::thread_rng();
        order.shuffle(&mut rng);

        Self {
            graph,
            batch_size,
            order,
            current_node: 0,
        }
    }
}

impl<'graph, G> WeightedUndirectedGraphStream for RandomOrderGraphStream<'graph, G>
where
    G: WeightedUndirectedGraph,
{
    type NodeWeight = G::NodeWeight;
    type EdgeWeight = G::EdgeWeight;
    type NeighboursIter = G::WeightedNeighboursIter;
    type Batch = Vec<(NodeIndex, Self::NodeWeight, Self::NeighboursIter)>;

    fn next_batch(&mut self) -> Option<Self::Batch> {
        if self.current_node == self.order.len() as NodeIndex {
            return None;
        }

        let batch_start = self.current_node;
        self.current_node = (self.current_node + self.batch_size as NodeIndex)
            .min(self.order.len() as NodeIndex);

        // TODO: consider getting rid of collecting into a vector
        Some((&self.order[batch_start as usize..self.current_node as usize])
            .into_iter()
            .map(|&node| {
                let node_weight = self.graph.node_weight(node);
                let neighbours = self.graph.weighted_edges(node);
                (node, node_weight, neighbours)
            }).collect())
    }

    fn opt_remaining_batch_count(&self) -> Option<usize> {
        self.opt_remaining_node_count()
            .map(|count| (count + self.batch_size - 1) / self.batch_size)
    }

    fn opt_remaining_node_count(&self) -> Option<usize> {
        Some(self.graph.node_count() - self.current_node as usize)
    }

    fn opt_total_node_count(&self) -> Option<usize> {
        Some(self.graph.node_count())
    }

    fn opt_total_edge_count(&self) -> Option<usize> {
        Some(self.graph.edge_count())
    }

    fn opt_total_node_weight(&self) -> Option<Self::NodeWeight> {
        Some(self.graph.total_node_weight())
    }

    fn opt_total_edge_weight(&self) -> Option<Self::EdgeWeight> {
        Some(self.graph.total_edge_weight())
    }
}
