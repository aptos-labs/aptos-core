// Copyright © Aptos Foundation

use std::cmp::{max, min};
use std::collections::HashSet;
use std::sync::Arc;
use rand;
use rand::distributions::WeightedIndex;
use rand::prelude::{Distribution, ThreadRng};

struct UndirectedEdge {
    node_id_0: usize,
    node_id_1: usize,
    weight: u64,
}

struct UndirectedGraph {
    num_nodes: usize,
    edges: Vec<UndirectedEdge>,
    eids_by_nid: Vec<Vec<usize>>,
}

impl UndirectedGraph {
    fn eids_by_nid(num_nodes: usize, edges: &Vec<UndirectedEdge>) -> Vec<Vec<usize>>{
        let mut ret = vec![vec![]; num_nodes];
        for (eid, edge) in edges.iter().enumerate() {
            ret.get_mut(edge.node_id_0).unwrap().push(eid);
            ret.get_mut(edge.node_id_1).unwrap().push(eid);
        }
        ret
    }

    fn new(num_nodes: usize, edges: Vec<(usize, usize, u64)>) -> Self {
        let edges = edges.into_iter().map(|(a, b, c)| UndirectedEdge{
            node_id_0: min(a,b),
            node_id_1: max(a,b),
            weight: c,
        }).collect();
        let eids_by_nid = Self::eids_by_nid(num_nodes, &edges);
        Self {
            num_nodes,
            edges,
            eids_by_nid,
        }
    }

    fn rand_sharding(rng: &mut ThreadRng, account_weights: &Vec<u64>, block_size: usize) -> Self {
        let dist = WeightedIndex::new(account_weights).unwrap();
        let num_nodes = account_weights.len() + block_size;

        let mut edges = Vec::new();
        for tid in 0..block_size {
            let sender_idx  = dist.sample(rng);
            let recipient_idx = dist.sample(rng);
            edges.push(UndirectedEdge {
                node_id_0: tid,
                node_id_1: block_size + sender_idx,
                weight: 1,
            });
            edges.push(UndirectedEdge {
                node_id_0: tid,
                node_id_1: block_size + recipient_idx,
                weight: 1,
            });
        }

        let eids_by_nid = Self::eids_by_nid(num_nodes, &edges);
        Self {
            num_nodes,
            edges,
            eids_by_nid,
        }

    }

    fn cut_volume(&self, partitioning: &Vec<usize>) -> u64 {
        assert_eq!(self.num_nodes, partitioning.len());
        let mut ret = 0;
        for edge in self.edges.iter() {
            if partitioning[edge.node_id_0] != partitioning[edge.node_id_1] {
                ret += edge.weight;
            }
        }
        ret
    }
}

#[test]
fn test_cut_volume() {
    let graph = UndirectedGraph::new(4, vec![(1,2,100), (2,3,100), (0,3,100)]);
    assert_eq!(300, graph.cut_volume(&vec![0,1,2,3]));
    assert_eq!(200, graph.cut_volume(&vec![0,0,1,1]));
    assert_eq!(100, graph.cut_volume(&vec![1,0,1,1]));
    assert_eq!(100, graph.cut_volume(&vec![1,0,0,1]));
}

fn ppt_min_cut(graph: &UndirectedGraph) -> Vec<usize> {
    vec![]
}

fn all_binary_partitioning(n: usize) -> Vec<Vec<usize>> {
    if n <= 1 {
        vec![]
    } else {
        let sub_pars = all_binary_partitioning(n - 1);
        let mut ret = vec![];
        for mut sub_par in sub_pars {
            let mut par1 = sub_par.clone();
            sub_par.push(0);
            par1.push(1);
            ret.push(sub_par);
            ret.push(par1);
        }
        ret.push(vec![vec![0; n-1], vec![1]].concat());
        ret
    }
}

#[test]
fn test_all_binary_partitioning() {
    for par in all_binary_partitioning(4) {
        println!("par={:?}", par);
    }
}


fn bf_min_cut(graph: &UndirectedGraph) -> Vec<usize> {
    let mut best = u64::MAX;
    let mut sol = vec![];
    for partitioning in all_binary_partitioning(graph.num_nodes) {
        let cur = graph.cut_volume(&partitioning);
        if cur < best {
            best = cur;
            sol = partitioning;
        }
    }
    sol
}

#[test]
fn test3() {
    let mut rng = rand::thread_rng();
    let num_accounts: usize = 4;
    let account_weights: Vec<u64> = vec![1; num_accounts];
    let block_size: usize = 15;
    let graph = UndirectedGraph::rand_sharding(&mut rng, &account_weights, block_size);
    // let ppt_min_cut_output = ppt_min_cut(&graph);
    // println!("ppt_min_cut={}", graph.cut_volume(&ppt_min_cut_output));
    let bf_min_cut_output = bf_min_cut(&graph);
    println!("bf_min_cut={}", graph.cut_volume(&bf_min_cut_output));
}

#[test]
fn test4() {
    let graph = UndirectedGraph::new(5, vec![(0,1,200), (0,2,200),(0,3,100,),(1,2,300),(1,3,200),(1,4,100),(2,3,200),(2,4,100),(3,4,100)]);
    // let ppt_min_cut_output = ppt_min_cut(&graph);
    // println!("ppt_min_cut={}", graph.cut_volume(&ppt_min_cut_output));
    let bf_min_cut_output = bf_min_cut(&graph);
    println!("bf_min_cut={}", graph.cut_volume(&bf_min_cut_output));
    println!("bf_min_cut_output={:?}", &bf_min_cut_output);
}
