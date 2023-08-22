// Copyright © Aptos Foundation

use std::collections::HashMap;
use rand::seq::SliceRandom;

use aptos_graphs::graph::Node;
use aptos_graphs::graph_stream::{BatchInfo, StreamBatchInfo, StreamNode};
use aptos_graphs::partitioning::{PartitionId, StreamingGraphPartitioner};
use aptos_graphs::{GraphStream, NodeIndex};
use aptos_transaction_orderer::common::PTransaction;
use aptos_types::batched_stream::{BatchedStream, MapItems};

use crate::{PartitionedTransaction, SerializationIdx, StreamingTransactionPartitioner};

/// The weight of a node in the transaction graph.
///
/// For simplicity, it is a fixed type and not a generic parameter.
type NodeWeight = f64;

/// The weight of an edge in the transaction graph.
///
/// For simplicity, it is a fixed type and not a generic parameter.
type EdgeWeight = f64;

/// Partitions transactions using a streaming graph partitioner on a graph
/// where the nodes are transactions and the edges are dependencies with
/// the weight of an edge depending on how far the transactions are in the
/// serialization order.
pub struct TransactionGraphPartitioner<P, NF, EF> {
    graph_partitioner: P,

    /// The parameters of the transaction graph partitioner.
    pub params: Params<NF, EF>,
}

/// The parameters of the transaction graph partitioner.
#[derive(Clone, Debug)]
pub struct Params<NF, EF> {
    /// The function that maps transactions to node weights.
    pub node_weight_function: NF,

    /// The function that maps pairs of serialization indices of transactions to edge weights.
    pub edge_weight_function: EF,

    /// The number of partitions.
    pub n_partitions: usize,

    /// Whether to shuffle batches before passing them to the graph partitioner.
    pub shuffle_batches: bool,
}

// `T` is the type of a transaction.
// `S` is the type of the input stream of transactions.
// `NF` maps transactions to node weights.
// `EF` maps pairs of transactions with their serialization indices to edge weights.
// `P` is the type of the streaming graph partitioner.
impl<T, S, NF, EF, P> StreamingTransactionPartitioner<S> for TransactionGraphPartitioner<P, NF, EF>
where
    T: PTransaction,
    T::Key: Clone + Eq + std::hash::Hash,
    S: BatchedStream<StreamItem = T>,
    NF: Clone + Fn(&T) -> NodeWeight,
    EF: Clone + Fn(SerializationIdx, SerializationIdx) -> EdgeWeight,
    P: Clone + StreamingGraphPartitioner<TransactionGraphStream<S, NF, EF>>,
{
    type Error = P::Error;

    type ResultStream = MapItems<
        P::ResultStream,
        fn(
            (StreamNode<TransactionGraphStream<S, NF, EF>>, PartitionId),
        ) -> PartitionedTransaction<T>,
    >;

    fn partition_transactions(
        &mut self,
        transactions: S,
    ) -> Result<Self::ResultStream, Self::Error> {
        let graph_stream = TransactionGraphStream::new(
            transactions,
            self.params.clone(),
        );

        // partitioned_graph_stream: BatchedStream<StreamItem = (SerializationIdx, PartitionId)>
        let partitioned_graph_stream = self
            .graph_partitioner
            .partition_stream(graph_stream, self.params.n_partitions)?;

        Ok(
            partitioned_graph_stream.map_items(|(node, partition)| PartitionedTransaction {
                transaction: node.data.transaction,
                serialization_idx: node.index as SerializationIdx,
                partition,
                dependencies: node.data.dependencies,
            }),
        )
    }
}

/// Takes a stream of transactions and turns it into a stream of the dependency graph.
pub struct TransactionGraphStream<S, NF, EF>
where
    S: BatchedStream,
    S::StreamItem: PTransaction,
{
    transactions: S,
    params: Params<NF, EF>,
    next_serialization_idx: SerializationIdx,
    last_write: HashMap<<S::StreamItem as PTransaction>::Key, SerializationIdx>,
    partitioned_node_weight: NodeWeight,
    partitioned_edge_weight: EdgeWeight,
    edges: Vec<Vec<(NodeIndex, EdgeWeight)>>,
    rng: rand::rngs::ThreadRng,
}

impl<T, S, NF, EF> TransactionGraphStream<S, NF, EF>
where
    T: PTransaction,
    T::Key: Clone + Eq + std::hash::Hash,
    S: BatchedStream<StreamItem = T>,
    NF: Clone + Fn(&T) -> NodeWeight,
    EF: Clone + Fn(SerializationIdx, SerializationIdx) -> EdgeWeight,
{
    fn new(transactions: S, params: Params<NF, EF>) -> Self {
        Self {
            transactions,
            params,
            next_serialization_idx: 0,
            last_write: HashMap::new(),
            partitioned_node_weight: 0.,
            partitioned_edge_weight: 0.,
            edges: Vec::new(),
            rng: rand::thread_rng(),  // TODO: allow using a custom RNG.
        }
    }

    fn add_transaction(&mut self, tx: T) -> StreamNode<Self> {
        let idx = self.next_serialization_idx;
        assert_eq!(idx as usize, self.edges.len());
        self.next_serialization_idx += 1;
        self.edges.push(Vec::new());

        // Find this transaction's dependencies.
        let deps: Vec<SerializationIdx> = tx
            .read_set()
            .filter_map(|key| self.last_write.get(&key).copied())
            .collect();

        let mut new_edges_weight = 0.;

        // Add edges to the dependency graph based on the dependencies.
        for &dep in deps.iter() {
            let edge_weight = (self.params.edge_weight_function)(idx, dep);
            new_edges_weight += edge_weight;
            self.edges[idx as usize].push((dep as NodeIndex, edge_weight));
            self.edges[dep as usize].push((idx as NodeIndex, edge_weight));
        }

        // Update the last write for each key in the write set.
        for key in tx.write_set() {
            self.last_write.insert(key.clone(), idx);
        }

        // Compute the node weight of this transaction.
        let node_weight = (self.params.node_weight_function)(&tx);

        // Update the partitioned node and edge weights.
        self.partitioned_node_weight += node_weight;
        self.partitioned_edge_weight += new_edges_weight;

        Node {
            index: idx as NodeIndex,
            data: TxnWithDeps {
                transaction: tx,
                dependencies: deps,
            },
            weight: node_weight,
        }
    }
}

impl<T, S, NF, EF> GraphStream for TransactionGraphStream<S, NF, EF>
where
    T: PTransaction,
    T::Key: Clone + Eq + std::hash::Hash,
    S: BatchedStream<StreamItem = T>,
    NF: Clone + Fn(&T) -> NodeWeight,
    EF: Clone + Fn(SerializationIdx, SerializationIdx) -> EdgeWeight,
{
    type NodeData = TxnWithDeps<T>;
    type NodeWeight = NodeWeight;
    type EdgeWeight = EdgeWeight;
    type Error = S::Error;

    type NodeEdges<'a> = std::iter::Copied<std::slice::Iter<'a, (NodeIndex, EdgeWeight)>>
    where Self: 'a;

    type Batch<'a> = Vec<(StreamNode<Self>, Self::NodeEdges<'a>)>
    where Self: 'a;

    fn next_batch(
        &mut self,
    ) -> Option<Result<(Self::Batch<'_>, StreamBatchInfo<Self>), Self::Error>> {
        let batch = match self.transactions.next_batch()? {
            Ok(batch) => batch,
            Err(err) => return Some(Err(err)),
        }
        .into_iter();

        let partitioned_node_weight = self.partitioned_node_weight;
        let partitioned_edge_weight = self.partitioned_edge_weight;

        // The `collect` is necessary to ensure that all edges are added to the graph before
        // any node is returned.
        let batch_nodes: Vec<StreamNode<Self>> = batch
            .into_iter()
            .map(|tx| self.add_transaction(tx))
            .collect();

        let batch_node_weight = self.partitioned_node_weight - partitioned_node_weight;
        let batch_edge_weight = self.partitioned_edge_weight - partitioned_edge_weight;

        let mut batch: Vec<_> = batch_nodes
            .into_iter()
            .map(|node: StreamNode<Self>| {
                let index = node.index;
                (node, self.edges[index as usize].iter().copied())
            })
            .collect();

        if self.params.shuffle_batches {
            batch.shuffle(&mut self.rng);
        }

        let batch_info = BatchInfo {
            opt_total_batch_node_count: None,
            opt_total_batch_edge_count: None,
            opt_total_batch_node_weight: Some(batch_node_weight),
            opt_total_batch_edge_weight: Some(batch_edge_weight),
        };

        Some(Ok((batch, batch_info)))
    }

    fn opt_remaining_batch_count(&self) -> Option<usize> {
        self.transactions.opt_batch_count()
    }

    fn opt_remaining_node_count(&self) -> Option<usize> {
        self.transactions.opt_items_count()
    }
}

/// A transaction with its dependencies.
pub struct TxnWithDeps<T: PTransaction> {
    transaction: T,
    dependencies: Vec<SerializationIdx>,
}
