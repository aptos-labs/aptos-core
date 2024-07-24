// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Copyright © Aptos Foundation

use crate::{
    graph::{EdgeWeight, NodeWeight},
    simple_graph::SimpleUndirectedGraph,
};

/// Creates a simple graph with four nodes that is useful for testing partitioning algorithms.
pub fn simple_four_nodes_two_partitions_graph() -> SimpleUndirectedGraph<&'static str> {
    // This graph in ASCII:
    //    (95) n1 --15-- n3 (10)
    //         |        /
    //        10      15
    //         |    /
    //         |  /
    //   (100) n0 --100-- n2 (10)
    let mut graph = SimpleUndirectedGraph::new();

    let n0 = graph.add_node("node0", 100 as NodeWeight);
    let n1 = graph.add_node("node1", 95 as NodeWeight);
    let n2 = graph.add_node("node2", 10 as NodeWeight);
    let n3 = graph.add_node("node3", 10 as NodeWeight);

    graph.add_edge(n0, n1, 10 as EdgeWeight);
    graph.add_edge(n0, n2, 100 as EdgeWeight);
    graph.add_edge(n0, n3, 15 as EdgeWeight);
    graph.add_edge(n1, n3, 15 as EdgeWeight);

    graph
}
