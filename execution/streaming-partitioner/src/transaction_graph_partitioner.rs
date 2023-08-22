// Copyright © Aptos Foundation

use std::collections::HashMap;

use aptos_graphs::graph::Node;
use aptos_graphs::graph_stream::{BatchInfo, StreamBatchInfo, StreamNode};
use aptos_graphs::{GraphStream, NodeIndex};
use aptos_graphs::partitioning::{PartitionId, StreamingGraphPartitioner};
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
    node_weight_function: NF,
    edge_weight_function: EF,
    n_partitions: usize,
}

// `T` is the type of a transaction.
// `S` is the type of the input stream of transactions.
// `NF` maps transactions to node weights.
// `EF` maps pairs of transactions with their serialization indices to edge weights.
// `P` is the type of the streaming graph partitioner.
impl<T, S, NF, EF, P> StreamingTransactionPartitioner<S>
    for TransactionGraphPartitioner<P, NF, EF>
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
        fn((StreamNode<TransactionGraphStream<S, NF, EF>>, PartitionId)) -> PartitionedTransaction<T>,
    >;

    fn partition_transactions(&mut self, transactions: S) -> Result<Self::ResultStream, Self::Error> {
        let graph_stream = TransactionGraphStream::new(
            transactions,
            self.node_weight_function.clone(),
            self.edge_weight_function.clone(),
        );

        // partitioned_graph_stream: BatchedStream<StreamItem = (SerializationIdx, PartitionId)>
        let partitioned_graph_stream = self
            .graph_partitioner
            .partition_stream(graph_stream, self.n_partitions)?;

        Ok(partitioned_graph_stream
            .map_items(|(node, partition)| {
                PartitionedTransaction {
                    transaction: node.data.transaction,
                    serialization_idx: node.index as SerializationIdx,
                    partition,
                    dependencies: node.data.dependencies,
                }
            })
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
    next_serialization_idx: SerializationIdx,
    node_weight_function: NF,
    edge_weight_function: EF,
    last_write: HashMap<<S::StreamItem as PTransaction>::Key, SerializationIdx>,
}

impl<T, S, NF, EF> TransactionGraphStream<S, NF, EF>
where
    T: PTransaction,
    T::Key: Clone + Eq + std::hash::Hash,
    S: BatchedStream<StreamItem = T>,
    NF: Clone + Fn(&T) -> NodeWeight,
    EF: Clone + Fn(SerializationIdx, SerializationIdx) -> EdgeWeight,
{
    fn new(transactions: S, node_weight_function: NF, edge_weight_function: EF) -> Self {
        Self {
            transactions,
            next_serialization_idx: 0,
            node_weight_function,
            edge_weight_function,
            last_write: HashMap::new(),
        }
    }

    fn add_transaction(&mut self, tx: T) -> (Node<TxnWithDeps<T>, NodeWeight>, Vec<(NodeIndex, EdgeWeight)>) {
        let idx = self.next_serialization_idx;
        self.next_serialization_idx += 1;

        // Find this transaction's dependencies.
        let deps: Vec<SerializationIdx> = tx.read_set().filter_map(|key| {
            self.last_write.get(&key).copied()
        }).collect();

        // Update the last write for each key in the write set.
        for key in tx.write_set() {
            self.last_write.insert(key.clone(), idx);
        }

        let weight = (self.node_weight_function)(&tx);

        let edges: Vec<(NodeIndex, EdgeWeight)> = deps.iter().map(|&dep| {
            (dep as NodeIndex, (self.edge_weight_function)(idx, dep))
        }).collect();

        (
            Node {
                index: idx as NodeIndex,
                data: TxnWithDeps {
                    transaction: tx,
                    dependencies: deps,
                },
                weight,
            },
            edges,
        )
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

    type NodeEdgesIter<'a> = Vec<(NodeIndex, EdgeWeight)>
    where Self: 'a;

    type Batch<'a> = Vec<(StreamNode<Self>, Self::NodeEdgesIter<'a>)>
    where Self: 'a;

    fn next_batch(&mut self) -> Option<Result<(Self::Batch<'_>, StreamBatchInfo<Self>), Self::Error>> {
        let batch = match self.transactions.next_batch()? {
            Ok(batch) => batch,
            Err(err) => return Some(Err(err)),
        };

        let batch: Vec<_> = batch.into_iter().map(|tx| self.add_transaction(tx)).collect();

        let batch_info = BatchInfo {
            opt_total_batch_node_count: None,
            opt_total_batch_edge_count: None,
            opt_total_batch_node_weight: Some(batch.iter().map(|(node, _)| node.weight).sum()),
            opt_total_batch_edge_weight: Some(batch
                .iter()
                .flat_map(|(_, edges)| edges.iter().map(|&(_, weight)| weight))
                .sum()
            ),
        };

        Some(Ok((batch, batch_info)))
    }

    fn opt_remaining_batch_count(&self) -> Option<usize> {
        self.transactions.opt_batch_count()
    }

    fn opt_remaining_node_count(&self) -> Option<usize> {
        self.transactions.opt_items_count()
    }

    fn opt_total_node_count(&self) -> Option<usize> {
        None
    }

    fn opt_total_edge_count(&self) -> Option<usize> {
        None
    }
}

/// A transaction with its dependencies.
pub struct TxnWithDeps<T: PTransaction> {
    transaction: T,
    dependencies: Vec<SerializationIdx>,
}
