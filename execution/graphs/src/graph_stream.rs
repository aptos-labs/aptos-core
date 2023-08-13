// Copyright © Aptos Foundation

use aptos_types::with::{MapWithOp, MapWithRef};
use crate::graph::{NodeIndex, WeightedUndirectedGraph};
use rand::seq::SliceRandom;
use aptos_types::batched_stream::{BatchedStream, BatchIterator, ItemsIterator};


/// A trait for batched streams for undirected graphs with weighted nodes and edges.
pub trait WeightedUndirectedGraphStream: Sized {
    /// The weight of a node.
    type NodeWeight;

    /// The weight of an edge.
    type EdgeWeight;

    /// An iterator over the neighbours of a node in the graph.
    type NeighboursIter<'a>: Iterator<Item = (NodeIndex, Self::EdgeWeight)>
    where
        Self: 'a;

    /// An iterator over the nodes in a batch.
    type BatchIter<'a>: Iterator<Item = (NodeIndex, Self::NodeWeight, Self::NeighboursIter<'a>)>
    where
        Self: 'a;

    /// Applies a function to the next batch of items in the stream.
    fn process_batch<'a, F, R>(&'a mut self, f: F) -> R
        where
            F: FnOnce(Option<Self::BatchIter<'a>>) -> R;

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
pub trait GraphStreamer<G: WeightedUndirectedGraph> {
    type Stream<'graph>: WeightedUndirectedGraphStream<NodeWeight = G::NodeWeight, EdgeWeight = G::EdgeWeight>
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
        Self: 'graph,
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

    type NeighboursIter<'a> = G::WeightedNeighboursIter<'a>
    where
        Self: 'a;

    type BatchIter<'a> = MapWithRef<
        'a,
        std::ops::Range<NodeIndex>,
        Self,
        fn(&Self, NodeIndex) -> (NodeIndex, Self::NodeWeight, Self::NeighboursIter<'a>)
    >
    where
        Self: 'a,
        G: 'a;

    fn process_batch<'a, F, R>(&'a mut self, f: F) -> R where F: FnOnce(Option<Self::BatchIter<'a>>) -> R {
        if self.current_node == self.graph.node_count() as NodeIndex {
            return f(None);
        }

        let batch_start = self.current_node;
        self.current_node = (self.current_node + self.batch_size as NodeIndex)
            .min(self.graph.node_count() as NodeIndex);

        f(Some(
            (batch_start..self.current_node)
                .map_with_ref(self, |self_, node| {
                    let node_weight = self_.graph.node_weight(node);
                    let neighbours = self_.graph.weighted_edges(node);
                    (node, node_weight, neighbours)
                }))
        )
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

    type NeighboursIter<'a> = G::WeightedNeighboursIter<'a>
    where
        Self: 'a;

    type BatchIter<'a> = MapWithRef<
        'a,
        std::iter::Copied<std::slice::Iter<'a, NodeIndex>>,
        Self,
        fn(&Self, NodeIndex) -> (NodeIndex, Self::NodeWeight, Self::NeighboursIter<'a>)
    >
    where
        Self: 'a;

    fn process_batch<'a, F, R>(&'a mut self, f: F) -> R where F: FnOnce(Option<Self::BatchIter<'a>>) -> R {
        if self.current_node == self.order.len() as NodeIndex {
            return f(None);
        }

        let batch_start = self.current_node;
        self.current_node =
            (self.current_node + self.batch_size as NodeIndex).min(self.order.len() as NodeIndex);

        f(Some(
            (&self.order[batch_start as usize..self.current_node as usize])
                .into_iter()
                .copied()
                .map_with_ref(self, |self_, node| {
                    let node_weight = self_.graph.node_weight(node);
                    let neighbours = self_.graph.weighted_edges(node);
                    (node, node_weight, neighbours)
                })
        ))
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
