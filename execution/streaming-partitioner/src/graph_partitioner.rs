// Copyright © Aptos Foundation

/// A trait for graph partitioners.
pub trait GraphPartitioner<G> {

    /// Assigns each node in the graph to a partition.
    /// Outputs the mapping from node indices to partitions as a vector.
    /// Node i is assigned to partition output[i].
    fn partition(&self, graph: &G, n_partitions: usize) -> Vec<usize>;
}
