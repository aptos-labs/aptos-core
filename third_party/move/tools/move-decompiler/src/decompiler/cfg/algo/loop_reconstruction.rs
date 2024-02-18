// Copyright (c) Verichains
// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use std::collections::{BTreeSet, HashMap, HashSet, VecDeque};

use super::{
    super::datastructs::*,
    scc::{Graph, TarjanScc},
};

pub fn loop_reconstruction<BlockContent: BlockContentTrait>(
    bbs: &mut Vec<BasicBlock<usize, BlockContent>>,
    initial_variables: &HashSet<usize>,
) -> Result<(), anyhow::Error> {
    let mut full_view = HashSet::<usize>::new();
    for i in 0..bbs.len() {
        full_view.insert(i);
    }
    loop_reconstruction_recursive(
        bbs,
        bbs.len(),
        &full_view,
        usize::MAX,
        usize::MAX,
        0,
        initial_variables,
    )
}

const SCC_SUPER_GRAPH_EXIT_NODE: usize = usize::MAX;

fn loop_reconstruction_recursive<BlockContent: BlockContentTrait>(
    bbs: &mut Vec<BasicBlock<usize, BlockContent>>,
    original_len: usize,
    current_view: &HashSet<usize>,
    parent_start_idx: usize,
    parent_exit_idx: usize,
    start_idx: usize,
    variables_seen: &HashSet<usize>,
) -> Result<(), anyhow::Error> {
    let graph = build_graph(bbs, current_view, start_idx);
    if graph.nodes().len() == 0 {
        return Ok(());
    }

    let scc = TarjanScc::new(&graph);

    let SSCSuperGraph {
        graph: scc_super_graph,
        entry_nodes: scc_super_graph_node_entries,
        exit_nodes: scc_super_graph_node_exits,
    } = loop_reconstruction_populate_super_graph(bbs, current_view, start_idx, &scc, original_len)?;

    // validate the scc's
    for (scc_idx, scc_nodes) in scc.sccs() {
        if scc_nodes.len() == 1 {
            let node: usize = *scc_nodes.iter().next().unwrap();
            // if this node has self-loop, consider it as a loop
            if !bbs[node].next.next_blocks().iter().any(|x| **x == node) {
                continue;
            }
        }
        let entries_count = if let Some(entries) = scc_super_graph_node_entries.get(&scc_idx) {
            entries.len()
        } else {
            0
        };
        if entries_count > 1 {
            return Err(anyhow::anyhow!("Found SCC with multiple entries"));
        }
        if entries_count == 0 {
            return Err(anyhow::anyhow!(
                "Found non-entry SCC without entry (dead block)"
            ));
        }
    }

    // each scc is a loop, reconstruct them
    let empty_hashset = HashSet::<usize>::new();
    let mut processed_scc_indexes = HashSet::<usize>::new();
    let mut variables_seen = variables_seen.clone();

    for (scc_idx, scc_nodes) in scc.sccs() {
        if processed_scc_indexes.contains(&scc_idx) {
            continue;
        }

        let mut local_variables = HashSet::<usize>::new();
        for &idx in scc_nodes.iter() {
            bbs[idx].has_assignment_variables.iter().for_each(|&x| {
                if !variables_seen.contains(&x) {
                    local_variables.insert(x);
                }
            });
        }

        let previous_variables_seen = variables_seen.clone();
        for &idx in scc_nodes.iter() {
            variables_seen.extend(bbs[idx].referenced_variables_iter().cloned());
        }

        if scc_nodes.len() == 1 {
            let node: usize = *scc_nodes.iter().next().unwrap();
            // if this is a self-loop, we need to process further as a loop, otherwise we can skip it
            if !bbs[node].next.next_blocks().iter().any(|x| **x == node) {
                continue;
            }
        }

        let mut scc_nodes = scc_nodes.clone();
        let scc_entries = scc_super_graph_node_entries.get(&scc_idx).unwrap();
        let scc_entry = *scc_entries.iter().next().unwrap();

        let scc_exits = scc_super_graph_node_exits
            .get(&scc_idx)
            .unwrap_or(&empty_hashset);

        let must_be_exits = scc_filter_must_be_exits(
            bbs,
            current_view,
            scc_idx,
            &scc_nodes,
            &scc,
            &processed_scc_indexes,
            &scc_super_graph,
            scc_exits,
            parent_start_idx,
            parent_exit_idx,
        )?;

        if must_be_exits.len() > 1 {
            return Err(anyhow::anyhow!(
                "Failed to reconstruct loop, multiple exits"
            ));
        }

        let view = view_without_current_and_processed_sccs(
            current_view,
            &scc,
            scc_idx,
            &processed_scc_indexes,
        );

        let mut scc_exit = usize::MAX;

        if must_be_exits.len() == 1 {
            scc_exit = *must_be_exits.iter().next().unwrap();
        } else {
            let mut possible_exits = scc_exits.clone();

            if possible_exits.len() > 1 {
                if let Terminator::IfElse { else_block, .. } = bbs[scc_entry].next {
                    if possible_exits.contains(&else_block) {
                        scc_exit = else_block;
                    }
                }
                if scc_exit == usize::MAX {
                    // try to ignore the exits that has references to this loop's local variables if possible
                    let new_possible_exits = possible_exits
                        .iter()
                        .filter(|&&x| {
                            !cfg_find_reachable_and_exit_nodes(bbs, x, |x| view.contains(&x))
                                .reachable
                                .iter()
                                .any(|&x| {
                                    bbs[x]
                                        .referenced_variables_iter()
                                        .any(|&v| local_variables.contains(&v))
                                })
                        })
                        .cloned()
                        .collect::<HashSet<_>>();
                    if !new_possible_exits.is_empty() {
                        possible_exits = new_possible_exits;
                    }
                }
                if scc_exit == usize::MAX {
                    // heuristic: pick the exit with the largest offset
                    scc_exit = possible_exits
                        .iter()
                        .fold((0, 0), |(max_offset, current_exit), &i| {
                            if bbs[i].offset > max_offset {
                                (bbs[i].offset, bbs[i].idx)
                            } else {
                                (max_offset, current_exit)
                            }
                        })
                        .1;
                }
            }
            if scc_exit == usize::MAX && possible_exits.len() == 1 {
                scc_exit = *possible_exits.iter().next().unwrap();
            }
        }
        // move all other exits into this loop
        for &i in scc_exits.iter() {
            if i != scc_exit {
                cfg_find_reachable_and_exit_nodes(bbs, i, |x| view.contains(&x))
                    .reachable
                    .iter()
                    .for_each(|&x| {
                        if !scc_nodes.contains(&x) {
                            scc_nodes.push(x);
                        }
                    });
            }
        }
        for x in scc_nodes.iter() {
            if let Some((scc_id, _)) = scc.scc_for_node(*x) {
                processed_scc_indexes.insert(scc_id);
            }
        }

        let mut new_blocks: Vec<BasicBlock<usize, BlockContent>> = Vec::new();
        let mut next_block_idx = bbs.len();

        let mut dummy_break = HashMap::<usize, usize>::new();
        let mut dummy_continue = HashMap::<usize, usize>::new();

        let mut add_dummy_block_if_required = |base: usize, x: usize| {
            let mut x: usize = x;
            x = if x == scc_entry {
                if let Some(&id) = dummy_continue.get(&base) {
                    id
                } else {
                    let id = next_block_idx;
                    next_block_idx += 1;
                    let mut new_block: BasicBlock<usize, BlockContent> = Default::default();
                    new_block.idx = id;
                    new_block.offset = usize::MAX;
                    new_block.topo_priority = Some(0);
                    new_block.topo_after = HashSet::from([base]);
                    new_block.topo_before = HashSet::new();
                    if scc_exit != usize::MAX {
                        new_block.topo_before.insert(scc_exit);
                    }
                    new_block.next = Terminator::Continue { target: scc_entry };

                    new_blocks.push(new_block);
                    dummy_continue.insert(base, id);
                    id
                }
            } else {
                x
            };
            x = if x == scc_exit {
                if let Some(&id) = dummy_break.get(&base) {
                    id
                } else {
                    let id = next_block_idx;
                    next_block_idx += 1;

                    let mut new_block: BasicBlock<usize, BlockContent> = Default::default();
                    new_block.idx = id;
                    new_block.offset = usize::MAX;
                    new_block.topo_priority = Some(0);
                    new_block.topo_after = HashSet::from([base]);
                    // as x==scc_exit, scc_exit can't be usize::MAX
                    new_block.topo_before = HashSet::from([scc_exit]);
                    new_block.next = Terminator::Break { target: scc_exit };
                    new_blocks.push(new_block);

                    dummy_break.insert(base, id);
                    id
                }
            } else {
                x
            };
            x
        };

        for &i in scc_nodes.iter() {
            let b = &mut bbs[i];
            match b.next {
                Terminator::Branch { target } => {
                    if target == scc_entry {
                        b.next = Terminator::Continue { target };
                    };
                    if target == scc_exit {
                        b.next = Terminator::Break { target };
                    };
                }
                Terminator::IfElse {
                    if_block,
                    else_block,
                } => {
                    if b.idx != scc_entry {
                        b.next = Terminator::IfElse {
                            if_block: add_dummy_block_if_required(i, if_block),
                            else_block: add_dummy_block_if_required(i, else_block),
                        };
                    }
                }
                _ => {}
            }
        }

        let mut body_view = HashSet::<usize>::new();
        // new blocks only contain break and continue, all of them jump to body's external nodes,
        // so from the body's point of view, adding them or not doesn't change anything
        for &i in scc_nodes.iter() {
            if i != scc_entry {
                body_view.insert(i);
            }
        }

        // check the entry
        let mut is_valid_conditioned_entry = true;
        if let Terminator::IfElse {
            if_block,
            else_block,
        } = bbs[scc_entry].next
        {
            if !scc_nodes.contains(&if_block) && if_block != scc_exit {
                if cfg!(debug_assertions) {
                    return Err(anyhow::anyhow!(
                        "Failed to reconstruct loop, entry node {:?} is not in SCC {:?}, exit {:?}",
                        if_block,
                        scc_nodes,
                        scc_exit
                    ));
                } else {
                    return Err(anyhow::anyhow!(
                        "Failed to reconstruct loop, entry node is not in SCC"
                    ));
                }
            }
            if else_block != scc_exit {
                is_valid_conditioned_entry = false;
            }

            if !is_valid_conditioned_entry {
                // dummy block rewrite was skipped, we need to do it now
                bbs[scc_entry].next = Terminator::IfElse {
                    if_block: add_dummy_block_if_required(scc_entry, if_block),
                    else_block: add_dummy_block_if_required(scc_entry, else_block),
                };
            }
        } else {
            is_valid_conditioned_entry = false;
        }

        let content = HashSet::from_iter(scc_nodes.iter().cloned());
        if is_valid_conditioned_entry {
            if let Terminator::IfElse {
                if_block,
                else_block,
            } = bbs[scc_entry].next
            {
                bbs[scc_entry].next = Terminator::While {
                    inner_block: if_block,
                    outer_block: else_block,
                    content_blocks: content,
                };
            } else {
                unreachable!();
            }
        } else {
            bbs[scc_entry].unconditional_loop_entry = Some((scc_exit, content));
        }

        bbs.append(&mut new_blocks);

        if body_view.len() > 0 {
            loop_reconstruction_recursive(
                bbs,
                original_len,
                &body_view,
                scc_entry,
                scc_exit,
                scc_entry,
                &previous_variables_seen,
            )?;
        }
    }

    Ok(())
}

fn scc_filter_must_be_exits<BlockContent: BlockContentTrait>(
    bbs: &[BasicBlock<usize, BlockContent>],
    current_view: &HashSet<usize>,
    scc_idx: usize,
    _scc_nodes: &[usize],
    scc: &TarjanScc,
    processed_scc_indexes: &HashSet<usize>,
    _scc_super_graph: &Graph,
    scc_exits: &HashSet<usize>,
    _parent_start_idx: usize,
    _parent_exit_idx: usize,
) -> Result<HashSet<usize>, anyhow::Error> {
    let mut must_be_exits = HashSet::<usize>::new();
    if scc_exits.len() == 0 {
        return Ok(must_be_exits);
    }

    if !check_in_different_sccs(&scc, &scc_exits) {
        return Err(anyhow::anyhow!(
            "Failed to reconstruct loop, multiple exits {:?}",
            scc_exits
        ));
    }

    let view =
        view_without_current_and_processed_sccs(current_view, scc, scc_idx, processed_scc_indexes);

    let exits_copy = scc_exits.clone();
    let mut remain = scc_exits.clone();
    scc_exits.iter().for_each(|&exit| {
        let ReachableAndExitNodes { exits, reachable } =
            cfg_find_reachable_and_exit_nodes(bbs, exit, |x| view.contains(&x));
        for &other in exits_copy.iter() {
            if other != exit && reachable.contains(&other) {
                remain.remove(&exit);
                break;
            }
        }

        if !exits.is_empty() {
            must_be_exits.insert(exit);
        }
    });

    let limited_by_remain = must_be_exits
        .intersection(&remain)
        .cloned()
        .collect::<HashSet<_>>();

    if limited_by_remain.len() > 0 {
        must_be_exits = limited_by_remain;
    }

    Ok(must_be_exits)
}

fn view_without_current_and_processed_sccs<'a>(
    current_view: &'a HashSet<usize>,
    scc: &TarjanScc,
    scc_idx: usize,
    processed_scc_indexes: &HashSet<usize>,
) -> HashSet<&'a usize> {
    current_view
        .into_iter()
        .filter(|x| {
            if let Some((scc_id, _)) = scc.scc_for_node(**x) {
                scc_id != scc_idx && !processed_scc_indexes.contains(&scc_id)
            } else {
                true
            }
        })
        .collect::<HashSet<_>>()
}

struct SSCSuperGraph {
    graph: Graph,
    entry_nodes: HashMap<usize, HashSet<usize>>,
    exit_nodes: HashMap<usize, HashSet<usize>>,
}

fn loop_reconstruction_populate_super_graph<BlockContent: BlockContentTrait>(
    bbs: &[BasicBlock<usize, BlockContent>],
    current_view: &HashSet<usize>,
    start_idx: usize,
    scc: &TarjanScc,
    max_idx: usize,
) -> Result<SSCSuperGraph, anyhow::Error> {
    let mut scc_super_graph = Graph::new();
    let mut scc_super_graph_node_entries = HashMap::<usize, HashSet<usize>>::new();
    let mut scc_super_graph_node_exits = HashMap::<usize, HashSet<usize>>::new();
    for u in 0..bbs.len() {
        if !current_view.contains(&u) {
            continue;
        }
        if let Some((scc_id, _)) = scc.scc_for_node(u) {
            for &v in bbs[u].next.next_blocks() {
                if v >= max_idx {
                    continue;
                }
                // v is reachable so it's safe to unwrap
                let v_scc_id = if let Some((v_scc_id, _)) = scc.scc_for_node(v) {
                    v_scc_id
                } else {
                    // v is not visible
                    // make a fake scc for this node, so we can add an edge to the exit node
                    SCC_SUPER_GRAPH_EXIT_NODE
                };
                if scc_id != v_scc_id {
                    scc_super_graph.add_edge(scc_id, v_scc_id);
                    scc_super_graph_node_entries
                        .entry(v_scc_id)
                        .or_insert(HashSet::new())
                        .insert(v);
                    scc_super_graph_node_exits
                        .entry(scc_id)
                        .or_insert(HashSet::new())
                        .insert(v);
                }
            }
        }
    }

    if current_view.contains(&start_idx) {
        let root_scc_id = scc.scc_for_node(start_idx).unwrap().0;
        scc_super_graph_node_entries
            .entry(root_scc_id)
            .or_insert(HashSet::new())
            .insert(start_idx);
    } else {
        for possible_root in find_possible_root(bbs, start_idx, current_view)? {
            let root_scc_id = scc.scc_for_node(possible_root).unwrap().0;
            scc_super_graph_node_entries
                .entry(root_scc_id)
                .or_insert(HashSet::new())
                .insert(possible_root);
        }
    }

    Ok(SSCSuperGraph {
        graph: scc_super_graph,
        entry_nodes: scc_super_graph_node_entries,
        exit_nodes: scc_super_graph_node_exits,
    })
}

struct ReachableAndExitNodes {
    reachable: HashSet<usize>,
    // terminated: HashSet<usize>,
    exits: HashSet<usize>,
}

fn cfg_find_reachable_and_exit_nodes<BlockContent: BlockContentTrait>(
    bbs: &[BasicBlock<usize, BlockContent>],
    start: usize,
    fn_in_view: impl Fn(usize) -> bool,
) -> ReachableAndExitNodes {
    let mut queue = VecDeque::<usize>::new();
    let mut visited = HashSet::<usize>::new();
    let mut reachable = HashSet::<usize>::new();
    // let mut terminated = HashSet::<usize>::new();
    let mut exits = HashSet::<usize>::new();

    queue.push_back(start);
    visited.insert(start);
    while let Some(idx) = queue.pop_front() {
        if !fn_in_view(idx) {
            exits.insert(idx);
            continue;
        }
        reachable.insert(idx);
        if match &bbs[idx].next {
            Terminator::Normal
            | Terminator::IfElse { .. }
            | Terminator::Branch { .. }
            | Terminator::While { .. }
            | Terminator::Break { .. }
            | Terminator::Continue { .. } => false,
            Terminator::Ret | Terminator::Abort => true,
        } {
            // terminated.insert(idx);
            continue;
        }
        for &&nxt in bbs[idx].next.next_blocks().iter() {
            if visited.insert(nxt) {
                queue.push_back(nxt);
            }
        }
    }
    ReachableAndExitNodes {
        reachable,
        // terminated,
        exits,
    }
}

fn check_in_different_sccs(scc: &TarjanScc, scc_exits: &HashSet<usize>) -> bool {
    let mut scc_ids = HashSet::<usize>::new();
    for &i in scc_exits.iter() {
        if let Some((scc_id, _)) = scc.scc_for_node(i) {
            if !scc_ids.insert(scc_id) {
                return false;
            }
        } else {
            // i is not visible or it's the ssc itself, both case are safe
        }
    }
    true
}

fn find_possible_root<BlockContent: BlockContentTrait>(
    bbs: &[BasicBlock<usize, BlockContent>],
    start_idx: usize,
    current_view: &HashSet<usize>,
) -> Result<HashSet<usize>, anyhow::Error> {
    let mut possible_roots = HashSet::<usize>::new();
    for &v in bbs[start_idx].next.next_blocks() {
        if current_view.contains(&v) {
            possible_roots.insert(v);
        }
    }
    Ok(possible_roots)
}

fn build_graph<BlockContent: BlockContentTrait>(
    blocks: &[BasicBlock<usize, BlockContent>],
    current_view: &HashSet<usize>,
    starting_idx: usize,
) -> Graph {
    let mut graph = Graph::new();
    let mut visited = BTreeSet::<usize>::new();
    let mut queue = VecDeque::<usize>::new();
    queue.push_back(starting_idx);
    visited.insert(starting_idx);
    for u in current_view {
        graph.ensure_node(*u)
    }
    while let Some(idx) = queue.pop_front() {
        for &&nxt in blocks[idx].next.next_blocks().iter() {
            if !current_view.contains(&nxt) {
                continue;
            }
            if current_view.contains(&idx) {
                graph.add_edge(idx, nxt);
            }
            if visited.insert(nxt) {
                queue.push_back(nxt);
            }
        }
    }
    graph
}
