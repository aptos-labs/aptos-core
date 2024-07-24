// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Copyright © Aptos Foundation

use std::cmp::max;
use crate::{PartitionedTransaction, SerializationIdx, StreamingTransactionPartitioner};
use aptos_graphs::{
    graph::{EdgeWeight, Node, NodeWeight},
    graph_stream::{BatchInfo, StreamNode},
    partitioning::{PartitionId, StreamingGraphPartitioner},
    GraphStream, NodeIndex,
};
use aptos_transaction_orderer::common::PTransaction;
use aptos_types::batched_stream::{BatchedStream, MapItems};
use itertools::Itertools;
use rand::seq::SliceRandom;
use std::collections::HashMap;

/// Partitions transactions using a streaming graph partitioner on a graph
/// where the nodes are transactions and the edges are dependencies with
/// the weight of an edge depending on how far the transactions are in the
/// serialization order.
pub struct TransactionGraphPartitioner<P, NF, EF> {
    graph_partitioner: P,

    /// The parameters of the transaction graph partitioner.
    pub params: Params<NF, EF>,
}

impl<P, NF, EF> TransactionGraphPartitioner<P, NF, EF> {
    pub fn new(graph_partitioner: P, params: Params<NF, EF>) -> Self {
        Self {
            graph_partitioner,
            params,
        }
    }
}

/// The parameters of the transaction graph partitioner.
#[derive(Clone, Debug)]
pub struct Params<NF, EF> {
    /// The function that maps transactions to node weights.
    pub node_weight_function: NF,

    /// The function that maps pairs of serialization indices of transactions to edge weights.
    pub edge_weight_function: EF,

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
        let graph_stream = TransactionGraphStream::new(transactions, self.params.clone());

        // partitioned_graph_stream: BatchedStream<StreamItem = (SerializationIdx, PartitionId)>
        let partitioned_graph_stream = self.graph_partitioner.partition_stream(graph_stream)?;

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
            partitioned_node_weight: 0 as NodeWeight,
            partitioned_edge_weight: 0 as EdgeWeight,
            edges: Vec::new(),
            rng: rand::thread_rng(), // TODO: allow using a custom RNG.
        }
    }

    fn add_transaction(&mut self, tx: T) -> StreamNode<Self> {
        let idx = self.next_serialization_idx;
        assert_eq!(idx as usize, self.edges.len());
        self.next_serialization_idx += 1;
        self.edges.push(Vec::new());

        // Find this transaction's dependencies.
        let deps: HashMap<SerializationIdx, Vec<T::Key>> = tx
            .read_set()
            .filter_map(|key| Some((*self.last_write.get(key)?, key.clone())))
            .into_group_map();

        let mut new_edges_weight = 0 as EdgeWeight;

        // Add edges to the dependency graph based on the dependencies.
        let basic_network_latency_penalty: EdgeWeight = 10;
        let cross_round_discount: EdgeWeight = 10;
        let per_key_penalty: EdgeWeight = 1;
        let idx_round = idx / 48;
        for (&dep, keys) in deps.iter() {
            let dep_round = dep / 48;
            let edge_weight = per_key_penalty * (keys.len() as EdgeWeight) + max(0, basic_network_latency_penalty - cross_round_discount * ((idx_round - dep_round) as EdgeWeight));
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
    type Batch<'a> = Vec<(StreamNode<Self>, Self::NodeEdges<'a>)>
    where Self: 'a;
    type Error = S::Error;
    type NodeData = TxnWithDeps<T>;
    type NodeEdges<'a> = std::iter::Copied<std::slice::Iter<'a, (NodeIndex, EdgeWeight)>>
    where Self: 'a;

    fn next_batch(&mut self) -> Option<Result<(Self::Batch<'_>, BatchInfo), Self::Error>> {
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
    dependencies: HashMap<SerializationIdx, Vec<T::Key>>,
}

#[cfg(test)]
mod tests {
    use crate::{
        transaction_graph_partitioner::{EdgeWeight, NodeWeight},
        PartitionedTransaction, SerializationIdx, StreamingTransactionPartitioner,
    };
    use aptos_graphs::partitioning::{
        fennel::{AlphaComputationMode, BalanceConstraintMode, FennelGraphPartitioner},
        random::RandomPartitioner,
        PartitionId,
    };
    use aptos_transaction_orderer::common::PTransaction;
    use aptos_types::batched_stream::{
        BatchedStream, IntoNoErrorBatchedStream, NoErrorBatchedStream,
    };
    use rand::{rngs::StdRng, Rng, SeedableRng};
    use std::fmt::Debug;

    #[test]
    fn test_fennel_11_transactions_over_4_batches() {
        let input_stream = input_11_transactions_over_4_batches();

        // Fennel is likely to perform poorly on such a small input.
        let mut fennel = FennelGraphPartitioner::new(2);
        fennel.balance_constraint_mode = BalanceConstraintMode::Batched;
        fennel.alpha_computation_mode = AlphaComputationMode::Batched;

        let mut partitioner = super::TransactionGraphPartitioner {
            graph_partitioner: fennel,
            params: super::Params {
                node_weight_function,
                edge_weight_function,
                shuffle_batches: false,
            },
        };

        let res = partitioner.partition_transactions(input_stream).unwrap();

        print_output("Fennel partitioning (11 txns)", 2, 11, res, true);
    }

    // Run with `--features metis-partitioner` to enable Metis tests.
    #[test]
    #[cfg(feature = "metis-partitioner")]
    fn test_metis_11_transactions_over_4_batches() {
        use aptos_graphs::partitioning::{metis::*, WholeGraphStreamingPartitioner};

        let input_stream = input_11_transactions_over_4_batches();

        let metis = MetisGraphPartitioner::new(2);
        let streaming_partitioner = WholeGraphStreamingPartitioner::new(metis);

        let mut partitioner = super::TransactionGraphPartitioner {
            graph_partitioner: streaming_partitioner,
            params: super::Params {
                node_weight_function,
                edge_weight_function,
                shuffle_batches: false,
            },
        };

        let res = partitioner.partition_transactions(input_stream).unwrap();

        print_output("Metis partitioning (11 txns)", 2, 11, res, true);
    }

    #[test]
    fn test_pseudorandom_11_transactions_over_4_batches() {
        let input_stream = input_11_transactions_over_4_batches();

        let streaming_partitioner = RandomPartitioner::new(2);

        let mut partitioner = super::TransactionGraphPartitioner {
            graph_partitioner: streaming_partitioner,
            params: super::Params {
                node_weight_function,
                edge_weight_function,
                shuffle_batches: false,
            },
        };

        let res = partitioner.partition_transactions(input_stream).unwrap();

        print_output("Random partitioning (11 txns)", 2, 11, res, true);
    }

    #[test]
    fn test_fennel_100k_transactions_over_100_batches_into_60_partitions() {
        let input_stream = input_random_p2p_transactions(100, 1000, 1000);

        let mut fennel = FennelGraphPartitioner::new(60);
        fennel.balance_constraint_mode = BalanceConstraintMode::Batched;
        fennel.alpha_computation_mode = AlphaComputationMode::Batched;

        let mut partitioner = super::TransactionGraphPartitioner {
            graph_partitioner: fennel,
            params: super::Params {
                node_weight_function,
                edge_weight_function,
                shuffle_batches: false,
            },
        };

        let res = partitioner.partition_transactions(input_stream).unwrap();

        print_output("Fennel partitioning (100k)", 60, 100_000, res, false);
    }

    #[test]
    #[cfg(feature = "metis-partitioner")]
    fn test_metis_100k_transactions_over_100_batches_into_60_partitions() {
        use aptos_graphs::partitioning::{metis::*, WholeGraphStreamingPartitioner};

        let input_stream = input_random_p2p_transactions(100, 1000, 1000);

        let metis = MetisGraphPartitioner::new(60);
        let streaming_partitioner = WholeGraphStreamingPartitioner::new(metis);

        let mut partitioner = super::TransactionGraphPartitioner {
            graph_partitioner: streaming_partitioner,
            params: super::Params {
                node_weight_function,
                edge_weight_function,
                shuffle_batches: false,
            },
        };

        let res = partitioner.partition_transactions(input_stream).unwrap();

        print_output("Metis partitioning (100k)", 60, 100_000, res, false);
    }

    #[test]
    fn test_pseudorandom_100k_transactions_over_100_batches_into_60_partitions() {
        let input_stream = input_random_p2p_transactions(100, 1000, 1000);

        let streaming_partitioner = RandomPartitioner::new(60);

        let mut partitioner = super::TransactionGraphPartitioner {
            graph_partitioner: streaming_partitioner,
            params: super::Params {
                node_weight_function,
                edge_weight_function,
                shuffle_batches: false,
            },
        };

        let res = partitioner.partition_transactions(input_stream).unwrap();

        print_output("Random partitioning (100k)", 60, 100_000, res, false);
    }

    fn node_weight_function<K>(tx: &MockPTransaction<K>) -> NodeWeight {
        tx.estimated_gas as NodeWeight
    }

    fn edge_weight_function(idx1: SerializationIdx, idx2: SerializationIdx) -> EdgeWeight {
        ((1. / (1. + idx1 as f64 - idx2 as f64)) * 1000000.) as EdgeWeight
    }

    fn input_11_transactions_over_4_batches(
    ) -> impl NoErrorBatchedStream<StreamItem = MockPTransaction<String>> {
        let [w, x, y, z] = [
            String::from("w"),
            String::from("x"),
            String::from("y"),
            String::from("z"),
        ];

        // transactions with their read and write sets.
        // read and write sets are more-or-less arbitrary,
        // but the write set always contains all keys from the read set.
        // Key `w` is read by almost all transactions and is written by just 1 transaction.
        vec![
            vec![
                MockPTransaction::new(1, vec![w.clone(), x.clone()], vec![x.clone()]),
                MockPTransaction::new(2, vec![w.clone(), x.clone(), y.clone()], vec![y.clone()]),
                MockPTransaction::new(1, vec![y.clone(), z.clone()], vec![z.clone()]),
                MockPTransaction::new(1, vec![w.clone(), y.clone()], vec![y.clone()]),
            ],
            vec![
                MockPTransaction::new(3, vec![w.clone()], vec![w.clone()]),
                MockPTransaction::new(1, vec![w.clone(), x.clone()], vec![x.clone()]),
            ],
            vec![
                MockPTransaction::new(2, vec![w.clone(), x.clone()], vec![x.clone()]),
                MockPTransaction::new(1, vec![y.clone(), z.clone()], vec![z.clone()]),
            ],
            vec![
                MockPTransaction::new(1, vec![y.clone(), z.clone()], vec![y.clone()]),
                MockPTransaction::new(1, vec![w.clone(), y.clone()], vec![y.clone()]),
                MockPTransaction::new(2, vec![w.clone(), x.clone(), y.clone()], vec![x.clone()]),
            ],
        ]
        .into_no_error_batched_stream()
    }

    fn input_random_p2p_transactions(
        n_batches: usize,
        txns_per_batch: usize,
        n_accounts: usize,
    ) -> impl NoErrorBatchedStream<StreamItem = MockPTransaction<u32>> {
        // create an RNG with a fixed seed for reproducibility.
        let mut rng = StdRng::seed_from_u64(42);

        (0..n_batches)
            .map(move |_| {
                (0..txns_per_batch)
                    .map(|_| random_p2p_transaction(n_accounts, &mut rng))
                    .collect::<Vec<_>>()
            })
            .into_no_error_batched_stream()
    }

    // Use `cargo test -p aptos-streaming-partitioner -- --nocapture --test-threads 1`
    // to see the output. Otherwise, the output is hidden by the test harness.
    fn print_output<S, K>(
        name: &str,
        n_partitions: usize,
        n_txns: usize,
        res: S,
        print_partitioning: bool,
    ) where
        S: BatchedStream<StreamItem = PartitionedTransaction<MockPTransaction<K>>>,
        S::Error: Debug,
        K: Clone + Debug,
    {
        println!();
        println!("-------------------------------------");
        println!("--- {}", name);
        println!("-------------------------------------");
        let mut txns_by_partition = vec![vec![]; n_partitions];
        let mut partition_by_txn = vec![0; n_txns];

        // Read the output.
        for batch in res.unwrap_batches().into_no_error_batch_iter() {
            for tx in batch {
                partition_by_txn[tx.serialization_idx as usize] = tx.partition;
                txns_by_partition[tx.partition as usize].push(tx);
            }
        }

        // Sort the transactions by their serialization index.
        for partition in txns_by_partition.iter_mut() {
            partition.sort_by_key(|tx| tx.serialization_idx);
        }

        let mut cut_edges_weight = 0 as EdgeWeight;
        let mut total_edges_weight = 0 as EdgeWeight;

        // Compute the cut edges weight and the total edges weight.
        for (partition_idx, partition) in txns_by_partition.iter().enumerate() {
            for tx in partition {
                for &dep in tx.dependencies.keys() {
                    let edge_weight = edge_weight_function(tx.serialization_idx, dep);
                    total_edges_weight += edge_weight;
                    if partition_by_txn[dep as usize] != partition_idx as PartitionId {
                        cut_edges_weight += edge_weight;
                    }
                }
            }
        }

        if print_partitioning {
            // Print out the partitioning.
            for (partition_idx, partition) in txns_by_partition.iter().enumerate() {
                println!("Partition {}:", partition_idx);
                for tx in partition {
                    println!("  Txn {}: {:?}", tx.serialization_idx, tx);
                }
            }
        }

        let total_node_weight = txns_by_partition
            .iter()
            .flat_map(|partition| {
                partition
                    .iter()
                    .map(|tx| node_weight_function(&tx.transaction))
            })
            .sum::<NodeWeight>();

        let max_partition_weight = txns_by_partition
            .iter()
            .map(|partition| {
                partition
                    .iter()
                    .map(|tx| node_weight_function(&tx.transaction))
                    .sum::<NodeWeight>()
            })
            .max()
            .unwrap();

        // Print the cut edges weight.
        println!(
            "Cut edges weight: {} / {} ({:.2})",
            cut_edges_weight,
            total_edges_weight,
            cut_edges_weight as f64 / total_edges_weight as f64
        );
        println!(
            "Max partition weight: {} / {} ({:.4}, ideal: {:.4}, imbalance: {:.4})",
            max_partition_weight,
            total_node_weight,
            max_partition_weight as f64 / total_node_weight as f64,
            1. / n_partitions as f64,
            (max_partition_weight as f64 / (total_node_weight as f64 / n_partitions as f64)) - 1.
        );
        println!("-------------------------------------");
        println!();
    }

    fn random_p2p_transaction(n_accounts: usize, rng: &mut StdRng) -> MockPTransaction<u32> {
        let estimated_gas = rng.gen_range(1, 100);

        let sender = rng.gen_range(0, n_accounts as u32);
        let receiver = loop {
            let receiver = rng.gen_range(0, n_accounts as u32);
            if receiver != sender {
                break receiver;
            }
        };

        MockPTransaction::new(estimated_gas, vec![sender, receiver], vec![
            sender, receiver,
        ])
    }

    #[derive(Clone, Debug)]
    struct MockPTransaction<K> {
        estimated_gas: u32,
        read_set: Vec<K>,
        write_set: Vec<K>,
    }

    impl<K> MockPTransaction<K> {
        fn new(estimated_gas: u32, read_set: Vec<K>, write_set: Vec<K>) -> Self {
            Self {
                estimated_gas,
                read_set,
                write_set,
            }
        }
    }

    impl<K> PTransaction for MockPTransaction<K> {
        type Key = K;
        type ReadSetIter<'a> = std::slice::Iter<'a, Self::Key>
        where Self: 'a;
        type WriteSetIter<'a> = std::slice::Iter<'a, Self::Key>
        where Self: 'a;

        fn read_set(&self) -> std::slice::Iter<Self::Key> {
            self.read_set.iter()
        }

        fn write_set(&self) -> std::slice::Iter<Self::Key> {
            self.write_set.iter()
        }
    }
}
