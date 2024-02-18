// Copyright (c) Verichains
// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use std::collections::{BTreeSet, HashMap, HashSet};

use super::{super::datastructs::*, scc::*};

fn topo_sort_stable_usize(
    edges: &Vec<Vec<usize>>,
    constraint_edges: &Vec<Vec<usize>>,
    priority: &Vec<usize>,
) -> Result<Vec<usize>, anyhow::Error> {
    let n = edges.len();

    let mut constraint_edges = constraint_edges.clone();
    for (idx, edge) in constraint_edges.iter_mut().enumerate() {
        edge.sort();
        edge.dedup();
        edge.retain(|&x| x != idx);
    }

    let mut edges = edges.clone();
    // normalize the edges: remove duplicate edges
    for (idx, edge) in edges.iter_mut().enumerate() {
        edge.sort();
        edge.dedup();
        edge.retain(|&x| x != idx);
    }

    // only keep reachable vertices from 0
    let reachable_vertices = {
        let mut visited = vec![false; n];
        let mut stack = Vec::<usize>::new();
        stack.push(0);
        visited[0] = true;
        while let Some(idx) = stack.pop() {
            for &next_idx in edges[idx].iter() {
                if !visited[next_idx] {
                    visited[next_idx] = true;
                    stack.push(next_idx);
                }
            }
        }
        visited
            .iter()
            .enumerate()
            .filter(|(_, &v)| v)
            .map(|(idx, _)| idx)
            .collect::<Vec<usize>>()
    };

    let mut redge = Vec::<HashSet<usize>>::new();
    redge.resize(n, Default::default());
    for (idx, edge) in edges.iter().enumerate() {
        for &next_idx in edge.iter() {
            redge[next_idx].insert(idx);
        }
    }

    let mut constraint_redge = Vec::<HashSet<usize>>::new();
    constraint_redge.resize(n, Default::default());
    for (idx, edge) in constraint_edges.iter().enumerate() {
        for &next_idx in edge.iter() {
            constraint_redge[next_idx].insert(idx);
        }
    }

    let mut result = Vec::<usize>::new();

    let mut queue = BTreeSet::<(usize, usize)>::new();

    let mut queued = vec![false; edges.len()];
    for &v in &reachable_vertices {
        if redge[v].is_empty() {
            queue.insert((priority[v], v));
            queued[v] = true;
        }
    }

    let mut remain = BTreeSet::from_iter(reachable_vertices.iter().map(|&u| (priority[u], u)));

    let check = |queue: &mut BTreeSet<(usize, usize)>,
                 queued: &mut Vec<bool>,
                 redge: &Vec<HashSet<usize>>,
                 constraint_redge: &Vec<HashSet<usize>>,
                 v: usize| {
        if !queued[v] && redge[v].is_empty() && constraint_redge[v].is_empty() {
            queue.insert((priority[v], v));
            queued[v] = true;
        }
    };

    loop {
        while let Some((_, v)) = queue.iter().next().cloned() {
            remain.remove(&(priority[v], v));
            queue.remove(&(priority[v], v));
            result.push(v);
            for &next_idx in edges[v].iter() {
                redge[next_idx].remove(&v);
                check(&mut queue, &mut queued, &redge, &constraint_redge, next_idx);
            }
            for &next_idx in constraint_edges[v].iter() {
                constraint_redge[next_idx].remove(&v);
                check(&mut queue, &mut queued, &redge, &constraint_redge, next_idx);
            }
        }

        if remain.is_empty() {
            break;
        }

        // there is at least one cycle here, pick the smallest vertex with no constraint
        let mut remain_iter = remain.iter();
        let mut next = None;
        while let Some(&(_, v)) = remain_iter.next() {
            if constraint_redge[v].is_empty() {
                next = Some(v);
                break;
            }
        }

        if let Some(v) = next {
            queue.insert((priority[v], v));
            queued[v] = true;
        } else {
            return Err(anyhow::anyhow!("cycle detected in constraint graph"));
        }
    }

    Ok(result)
}

fn recursive_add_relation_by_scc(
    edges: &mut Vec<Vec<usize>>,
    graph: &Graph,
    subset: &HashSet<usize>,
    entry: usize,
) {
    if subset.len() <= 1 {
        return;
    }

    let mut subset_graph = Graph::new();
    for &u in graph.nodes().iter().filter(|x| subset.contains(x)) {
        subset_graph.ensure_node(u);
        for &v in graph.edges(u).filter(|x| subset.contains(x)) {
            subset_graph.add_edge(u, v);
        }
    }
    let mut tarjan = TarjanScc::new(&subset_graph);

    if tarjan.sccs().count() == 1 {
        // try again without entry
        subset_graph.remove_edges_to(entry);
        tarjan = TarjanScc::new(&subset_graph);
    }

    if tarjan.sccs().count() == 1 {
        return;
    }

    let mut scc_supergraph_edges = HashSet::<(usize, usize)>::new();
    let mut scc_entry = HashMap::<usize, Vec<usize>>::new();

    scc_entry
        .entry(tarjan.scc_for_node(entry).unwrap().0)
        .or_insert(Vec::new())
        .push(entry);
    for &u in graph.nodes().iter().filter(|x| subset.contains(x)) {
        let scc_idx = tarjan.scc_for_node(u).unwrap().0;
        subset_graph.ensure_node(u);
        for &v in graph.edges(u).filter(|x| subset.contains(x)) {
            let next_scc_idx = tarjan.scc_for_node(v).unwrap().0;
            if scc_idx != next_scc_idx {
                scc_entry.entry(next_scc_idx).or_insert(Vec::new()).push(v);
                scc_supergraph_edges.insert((scc_idx, next_scc_idx));
            }
        }
    }

    let sccs = tarjan.sccs().collect::<HashMap<_, _>>();
    for (fr, to) in scc_supergraph_edges.iter() {
        let fr_nodes = sccs.get(fr).unwrap();
        let to_nodes = sccs.get(to).unwrap();
        for &fr_node in fr_nodes.iter() {
            for &to_node in to_nodes.iter() {
                edges[fr_node].push(to_node);
            }
        }
    }

    for (idx, scc) in tarjan.sccs() {
        if scc.len() > 1 {
            let mut entries = scc_entry.get(&idx).unwrap().clone();
            entries.sort();
            recursive_add_relation_by_scc(
                edges,
                &subset_graph,
                &HashSet::from_iter(scc.iter().cloned()),
                entries[0],
            );
        }
    }
}

pub fn topo_sort<BlockContent: BlockContentTrait>(
    blocks: Vec<BasicBlock<usize, BlockContent>>,
    scc_relation: bool,
) -> Result<Vec<BasicBlock<usize, BlockContent>>, anyhow::Error> {
    let mut edges = Vec::<Vec<usize>>::new();
    edges.resize(blocks.len(), Vec::new());
    let mut constraint_edges = Vec::<Vec<usize>>::new();
    constraint_edges.resize(blocks.len(), Vec::new());
    let mut priority = vec![0; blocks.len()];
    let max_block_offset = blocks
        .iter()
        .reduce(|a, b| if a.offset > b.offset { a } else { b })
        .map(|x| x.offset)
        .unwrap_or(0);

    if scc_relation {
        let mut graph = Graph::new();
        for (idx, block) in blocks.iter().enumerate() {
            graph.ensure_node(idx);
            for &&next_idx in block.next.next_blocks().iter() {
                graph.add_edge(idx, next_idx);
            }
        }

        let mut full_subset = HashSet::<usize>::new();
        for (idx, _) in blocks.iter().enumerate() {
            full_subset.insert(idx);
        }
        recursive_add_relation_by_scc(&mut edges, &graph, &full_subset, 0);
    }

    for (idx, block) in blocks.iter().enumerate() {
        priority[idx] = if let Some(p) = &block.topo_priority {
            *p
        } else if block.offset != usize::MAX {
            block.idx * 100000 + 1
        } else {
            // try to keep the original order
            usize::MAX - max_block_offset - 1 + block.offset
        };
        if let Some((exit, contents)) = &block.unconditional_loop_entry {
            if exit != &usize::MAX {
                for &content in contents.iter() {
                    edges[content].push(*exit);
                }
            }
        }
        match &block.next {
            Terminator::IfElse {
                if_block,
                else_block,
            } => {
                edges[idx].push(*if_block);
                edges[idx].push(*else_block);
            }
            Terminator::Break { target }
            | Terminator::Continue { target }
            | Terminator::Branch { target } => {
                edges[idx].push(*target);
            }
            Terminator::While {
                inner_block,
                outer_block,
                content_blocks,
            } => {
                edges[idx].push(*inner_block);
                edges[idx].push(*outer_block);
                edges[*inner_block].push(*outer_block);
                for &content_block in content_blocks.iter() {
                    edges[content_block].push(*outer_block);
                }
            }
            Terminator::Ret => {}
            Terminator::Abort => {}
            Terminator::Normal => {
                if cfg!(debug_assertions) {
                    return Err(anyhow::anyhow!(
                        "unsupported terminator {:?} in block {}",
                        block.next,
                        idx
                    ));
                } else {
                    return Err(anyhow::anyhow!(
                        "unsupported terminator {} in block {}",
                        block.next,
                        idx
                    ));
                }
            }
        }
        for x in block.topo_before.iter().filter(|x| **x < blocks.len()) {
            constraint_edges[idx].push(*x);
        }
        for x in block.topo_after.iter().filter(|x| **x < blocks.len()) {
            constraint_edges[*x].push(idx);
        }
    }

    let order = topo_sort_stable_usize(&edges, &constraint_edges, &priority)?;
    let rorder = {
        let mut rorder = vec![0; blocks.len()];
        for (idx, &order_idx) in order.iter().enumerate() {
            rorder[order_idx] = idx;
        }
        rorder
    };
    let mut result = Vec::<BasicBlock<usize, BlockContent>>::new();

    for (idx, &order_idx) in order.iter().enumerate() {
        let mut block = blocks[order_idx].clone();
        block.idx = idx;
        block.next = match block.next {
            Terminator::IfElse {
                if_block,
                else_block,
            } => Terminator::IfElse {
                if_block: rorder[if_block],
                else_block: rorder[else_block],
            },
            Terminator::Ret => Terminator::Ret,
            Terminator::Abort => Terminator::Abort,
            Terminator::Normal => Terminator::Normal,
            Terminator::While {
                inner_block,
                outer_block,
                content_blocks,
            } => Terminator::While {
                inner_block: rorder[inner_block],
                outer_block: rorder[outer_block],
                content_blocks: content_blocks
                    .iter()
                    .map(|&x| rorder[x])
                    .collect::<HashSet<usize>>(),
            },
            Terminator::Branch { target } => Terminator::Branch {
                target: rorder[target],
            },
            Terminator::Break { target } => Terminator::Break {
                target: rorder[target],
            },
            Terminator::Continue { target } => Terminator::Continue {
                target: rorder[target],
            },
        };
        block.topo_after = block
            .topo_after
            .iter()
            .map(|&x| rorder[x])
            .collect::<HashSet<usize>>();
        block.topo_before = block
            .topo_before
            .iter()
            .map(|&x| rorder[x])
            .collect::<HashSet<usize>>();
        if let Some((idx, contents)) = block.unconditional_loop_entry {
            let new_idx = if idx != usize::MAX { rorder[idx] } else { idx };
            block.unconditional_loop_entry = Some((
                new_idx,
                contents
                    .iter()
                    .map(|&x| rorder[x])
                    .collect::<HashSet<usize>>(),
            ));
        }
        result.push(block);
    }

    Ok(result)
}
