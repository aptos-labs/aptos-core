// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Copyright © Aptos Foundation

use crate::{
    graph,
    graph::{EdgeWeight, NodeIndex, NodeWeight},
    WeightedGraph,
};
use aptos_types::{
    closuretools::{ClosureTools, MapClosure},
    no_error,
    no_error::NoError,
};
use namable_closures::{closure, Closure};
use rand::seq::SliceRandom;

/// Convenience type alias for a node in a graph stream.
pub type StreamNode<S> = graph::Node<<S as GraphStream>::NodeData>;

/// Convenience type alias for a reference to a node in a graph stream.
pub type StreamNodeRef<'a, S> = graph::NodeRef<'a, <S as GraphStream>::NodeData>;

/// A trait for batched streams for undirected graphs with weighted nodes and edges.
pub trait GraphStream: Sized {
    /// The type of the nodes in the graph.
    type NodeData;

    /// The error type that can occur when advancing the stream.
    type Error;

    /// An iterator over the neighbours of a node in the graph.
    type NodeEdges<'a>: IntoIterator<Item = (NodeIndex, EdgeWeight)>
    where
        Self: 'a;

    /// An iterator over the nodes in a batch.
    type Batch<'a>: IntoIterator<Item = (StreamNode<Self>, Self::NodeEdges<'a>)>
    where
        Self: 'a;

    /// Advances the stream and returns the next value.
    ///
    /// Returns [`None`] when stream is finished.
    fn next_batch(&mut self) -> Option<Result<(Self::Batch<'_>, BatchInfo), Self::Error>>;

    /// Borrows a stream, rather than consuming it.
    ///
    /// This is useful to allow applying stream adapters while still retaining
    /// ownership of the original iterator, similarly to [`Iterator::by_ref`].
    fn by_ref(&mut self) -> &mut Self {
        self
    }

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
    fn opt_total_node_weight(&self) -> Option<NodeWeight> {
        None
    }

    /// Returns the total weight of all edges in the whole graph,
    /// including already processed batches, if available.
    fn opt_total_edge_weight(&self) -> Option<EdgeWeight> {
        None
    }

    /// Collects the stream into a graph or any other container implementing `FromGraphStream`.
    fn collect<B>(self) -> Result<B, B::Error>
    where
        B: FromGraphStream<Self>,
    {
        B::from_graph_stream(self)
    }

    /// Returns a graph stream with batches collected into `Vec`.
    fn materialize(self) -> Materialize<Self> {
        Materialize::new(self)
    }

    /// Returns a graph stream with shuffled batches.
    fn shuffle<R>(self, rng: &mut R) -> Shuffle<'_, Self, R>
    where
        R: rand::Rng,
    {
        Shuffle::new(self, rng)
    }
}

/// A struct containing optional information about a batch.
#[derive(Default, Debug, Clone, Copy, PartialEq)]
pub struct BatchInfo {
    pub opt_total_batch_node_count: Option<usize>,
    pub opt_total_batch_edge_count: Option<usize>,
    pub opt_total_batch_node_weight: Option<NodeWeight>,
    pub opt_total_batch_edge_weight: Option<EdgeWeight>,
}

/// A trait for types that can be constructed from a `GraphStream`.
pub trait FromGraphStream<S>: Sized
where
    S: GraphStream,
{
    type Error;

    /// Reconstructs a graph from a `GraphStream`.
    fn from_graph_stream(graph_stream: S) -> Result<Self, Self::Error>;
}

/// A trait for graph streams with known exact node count.
pub trait ExactNodeCountGraphStream: GraphStream {
    fn remaining_node_count(&self) -> usize {
        self.opt_remaining_node_count().unwrap()
    }

    fn total_node_count(&self) -> usize {
        self.opt_total_node_count().unwrap()
    }
}

// A mutable reference to a `GraphStream` is a `GraphStream` itself.
impl<'a, S> GraphStream for &'a mut S
where
    S: GraphStream,
{
    type Batch<'b> = S::Batch<'b>
    where Self: 'b;
    type Error = S::Error;
    type NodeData = S::NodeData;
    type NodeEdges<'b> = S::NodeEdges<'b>
    where Self: 'b;

    fn next_batch(&mut self) -> Option<Result<(Self::Batch<'_>, BatchInfo), Self::Error>> {
        (**self).next_batch()
    }

    fn opt_remaining_batch_count(&self) -> Option<usize> {
        (**self).opt_remaining_batch_count()
    }

    fn opt_remaining_node_count(&self) -> Option<usize> {
        (**self).opt_remaining_node_count()
    }

    fn opt_total_node_count(&self) -> Option<usize> {
        (**self).opt_total_node_count()
    }

    fn opt_total_edge_count(&self) -> Option<usize> {
        (**self).opt_total_edge_count()
    }

    fn opt_total_node_weight(&self) -> Option<NodeWeight> {
        (**self).opt_total_node_weight()
    }

    fn opt_total_edge_weight(&self) -> Option<EdgeWeight> {
        (**self).opt_total_edge_weight()
    }
}

/// Streams a graph in batches of fixed size, in order of nodes from `0` to `node_count() - 1`.
pub fn input_order_stream<G>(graph: &G, batch_size: usize) -> InputOrderGraphStream<'_, G>
where
    G: WeightedGraph,
{
    InputOrderGraphStream::new(graph, batch_size)
}

/// Streams graphs in batches of fixed size, in random order.
pub fn random_order_stream<G>(graph: &G, batch_size: usize) -> RandomOrderGraphStream<'_, G>
where
    G: WeightedGraph,
{
    RandomOrderGraphStream::new(graph, batch_size)
}

/// Streams a graph in batches of fixed size, in order from `0` to `node_count() - 1`.
pub struct InputOrderGraphStream<'graph, G> {
    graph: &'graph G,
    batch_size: usize,
    current_node: NodeIndex,
}

impl<'graph, G: WeightedGraph> InputOrderGraphStream<'graph, G> {
    pub fn new(graph: &'graph G, batch_size: usize) -> Self {
        Self {
            graph,
            batch_size,
            current_node: 0,
        }
    }
}

impl<'graph, G: WeightedGraph> GraphStream for InputOrderGraphStream<'graph, G> {
    type Batch<'a> = MapClosure<
        std::ops::Range<NodeIndex>,
        Closure<'a, Self, (NodeIndex,), (StreamNode<Self>, Self::NodeEdges<'a>)>,
    >
    where Self: 'a;
    type Error = NoError;
    type NodeData = &'graph G::NodeData;
    type NodeEdges<'a> = G::WeightedNodeEdgesIter<'a>
    where Self: 'a;

    fn next_batch(&mut self) -> Option<no_error::Result<(Self::Batch<'_>, BatchInfo)>> {
        if self.current_node == self.graph.node_count() as NodeIndex {
            return None;
        }

        let batch_start = self.current_node;
        self.current_node = (self.current_node + self.batch_size as NodeIndex)
            .min(self.graph.node_count() as NodeIndex);

        Some(Ok((
            (batch_start..self.current_node).map_closure(closure!(self_ = self => |idx| {
                let node_ref = self_.graph.get_node(idx);
                let neighbours = self_.graph.weighted_edges(idx);
                (graph::Node {
                    index: idx,
                    data: node_ref.data,
                    weight: node_ref.weight,
                }, neighbours)
            })),
            BatchInfo {
                opt_total_batch_node_count: Some(self.batch_size),
                opt_total_batch_edge_count: None,
                opt_total_batch_node_weight: None,
                opt_total_batch_edge_weight: None,
            },
        )))
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

    fn opt_total_node_weight(&self) -> Option<NodeWeight> {
        Some(self.graph.total_node_weight())
    }

    fn opt_total_edge_weight(&self) -> Option<EdgeWeight> {
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

impl<'graph, G: WeightedGraph> RandomOrderGraphStream<'graph, G> {
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

impl<'graph, G: WeightedGraph> GraphStream for RandomOrderGraphStream<'graph, G> {
    type Batch<'a> = MapClosure<
        std::iter::Copied<std::slice::Iter<'a, NodeIndex>>,
        Closure<'a, Self, (NodeIndex,), (StreamNode<Self>, Self::NodeEdges<'a>)>
    >
    where Self: 'a;
    type Error = NoError;
    type NodeData = &'graph G::NodeData;
    type NodeEdges<'a> = G::WeightedNodeEdgesIter<'a>
    where Self: 'a;

    fn next_batch(&mut self) -> Option<no_error::Result<(Self::Batch<'_>, BatchInfo)>> {
        if self.current_node == self.order.len() as NodeIndex {
            return None;
        }

        let batch_start = self.current_node;
        self.current_node =
            (self.current_node + self.batch_size as NodeIndex).min(self.order.len() as NodeIndex);

        Some(Ok((
            self.order[batch_start as usize..self.current_node as usize]
                .iter()
                .copied()
                .map_closure(closure!(self_ = self => |idx| {
                    let node_ref = self_.graph.get_node(idx);
                    let neighbours = self_.graph.weighted_edges(idx);
                    (graph::Node {
                        index: idx,
                        data: node_ref.data,
                        weight: node_ref.weight,
                    }, neighbours)
                })),
            BatchInfo {
                opt_total_batch_node_count: Some(self.batch_size),
                opt_total_batch_edge_count: None,
                opt_total_batch_node_weight: None,
                opt_total_batch_edge_weight: None,
            },
        )))
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

    fn opt_total_node_weight(&self) -> Option<NodeWeight> {
        Some(self.graph.total_node_weight())
    }

    fn opt_total_edge_weight(&self) -> Option<EdgeWeight> {
        Some(self.graph.total_edge_weight())
    }
}

/// A batched stream with batches collected into `Vec`.
pub struct Materialize<S> {
    inner: S,
}

impl<S> Materialize<S> {
    pub fn new(stream: S) -> Self {
        Self { inner: stream }
    }
}

impl<S> GraphStream for Materialize<S>
where
    S: GraphStream,
{
    type Batch<'a> = Vec<(StreamNode<S>, Self::NodeEdges<'a>)>
    where Self: 'a;
    type Error = S::Error;
    type NodeData = S::NodeData;
    type NodeEdges<'a> = S::NodeEdges<'a>
    where Self: 'a;

    fn next_batch(&mut self) -> Option<Result<(Self::Batch<'_>, BatchInfo), Self::Error>> {
        self.inner
            .next_batch()
            .map(|res| res.map(|(batch, info)| (batch.into_iter().collect(), info)))
    }

    fn opt_remaining_batch_count(&self) -> Option<usize> {
        self.inner.opt_remaining_batch_count()
    }

    fn opt_remaining_node_count(&self) -> Option<usize> {
        self.inner.opt_remaining_node_count()
    }

    fn opt_total_node_count(&self) -> Option<usize> {
        self.inner.opt_total_node_count()
    }

    fn opt_total_edge_count(&self) -> Option<usize> {
        self.inner.opt_total_edge_count()
    }

    fn opt_total_node_weight(&self) -> Option<NodeWeight> {
        self.inner.opt_total_node_weight()
    }

    fn opt_total_edge_weight(&self) -> Option<EdgeWeight> {
        self.inner.opt_total_edge_weight()
    }
}

/// A batched stream with shuffled batches.
pub struct Shuffle<'rng, S, R> {
    inner: Materialize<S>,
    rng: &'rng mut R,
}

impl<'rng, S, R> Shuffle<'rng, S, R>
where
    S: GraphStream,
{
    pub fn new(stream: S, rng: &'rng mut R) -> Self {
        Self {
            inner: stream.materialize(),
            rng,
        }
    }
}

impl<'rng, S, R> GraphStream for Shuffle<'rng, S, R>
where
    S: GraphStream,
    R: rand::Rng,
{
    type Batch<'a> = Vec<(StreamNode<S>, Self::NodeEdges<'a>)>
    where Self: 'a;
    type Error = S::Error;
    type NodeData = S::NodeData;
    type NodeEdges<'a> = S::NodeEdges<'a>
    where Self: 'a;

    fn next_batch(&mut self) -> Option<Result<(Self::Batch<'_>, BatchInfo), Self::Error>> {
        self.inner.next_batch().map(|res| {
            res.map(|(mut batch, info)| {
                batch.shuffle(self.rng);
                (batch, info)
            })
        })
    }

    fn opt_remaining_batch_count(&self) -> Option<usize> {
        self.inner.opt_remaining_batch_count()
    }

    fn opt_remaining_node_count(&self) -> Option<usize> {
        self.inner.opt_remaining_node_count()
    }

    fn opt_total_node_count(&self) -> Option<usize> {
        self.inner.opt_total_node_count()
    }

    fn opt_total_edge_count(&self) -> Option<usize> {
        self.inner.opt_total_edge_count()
    }

    fn opt_total_node_weight(&self) -> Option<NodeWeight> {
        self.inner.opt_total_node_weight()
    }

    fn opt_total_edge_weight(&self) -> Option<EdgeWeight> {
        self.inner.opt_total_edge_weight()
    }
}
