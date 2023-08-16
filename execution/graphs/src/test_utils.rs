// Copyright © Aptos Foundation

use crate::simple_graph::SimpleUndirectedGraph;

/// Creates a simple graph with four nodes that is useful for testing partitioning algorithms.
pub fn simple_four_nodes_two_partitions_graph() -> SimpleUndirectedGraph<u16, u16> {
    // This graph in ASCII:
    //    (95) n1 --15-- n3 (10)
    //         |        /
    //        10      15
    //         |    /
    //         |  /
    //   (100) n0 --100-- n2 (10)
    let mut graph = SimpleUndirectedGraph::new();

    let n0 = graph.add_node(100);
    let n1 = graph.add_node(95);
    let n2 = graph.add_node(10);
    let n3 = graph.add_node(10);

    graph.add_edge(n0, n1, 10);
    graph.add_edge(n0, n2, 100);
    graph.add_edge(n0, n3, 15);
    graph.add_edge(n1, n3, 15);

    graph
}
