// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::pipeline::livevar_analysis_processor::LiveVarAnnotation;
use move_binary_format::file_format::CodeOffset;
use move_model::{ast::TempIndex, model::FunctionEnv};
use move_stackless_bytecode::{
    function_target::{FunctionData, FunctionTarget},
    function_target_pipeline::{FunctionTargetProcessor, FunctionTargetsHolder},
    stackless_bytecode::{AssignKind, Bytecode, Operation},
    stackless_control_flow_graph::StacklessControlFlowGraph,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    mem,
};

#[derive(Clone, Debug)]
struct OrderInfo {
    dependencies: BTreeSet<CodeOffset>,
    dfs_numberings: Vec<Option<CodeOffset>>,
}

impl OrderInfo {
    pub fn remap(&self, base: CodeOffset) -> Self {
        let dependencies = self
            .dependencies
            .iter()
            .map(|d| *d + base)
            .collect::<BTreeSet<_>>();
        let dfs_numberings = self
            .dfs_numberings
            .iter()
            .map(|n| n.map(|n| n + base))
            .collect::<Vec<_>>();
        Self {
            dependencies,
            dfs_numberings,
        }
    }
}

#[derive(Clone, Debug)]
pub struct OrderingAnnotation(BTreeMap<CodeOffset, OrderInfo>);

/// Mapping from every touch instruction to the ("use" instruction, source offset, multi-use?) they correspond to.
#[derive(Clone, Debug)]
pub struct PrepareUseAnnotation(pub BTreeMap<CodeOffset, (CodeOffset, bool)>);

struct ReorderedBlock {
    block: Vec<Bytecode>,
    ordering: OrderingAnnotation,
    touch_use: PrepareUseAnnotation,
}

pub struct ReorderedFunction {
    code: Vec<Bytecode>,
    ordering: OrderingAnnotation,
    touch_use: PrepareUseAnnotation,
}

#[derive(Debug)]
struct UseDefGraph(pub BTreeMap<CodeOffset, Vec<Option<CodeOffset>>>);

// struct ReorderableBlock {}

// impl ReorderableBlock {
//     pub fn new(code: &[Bytecode], lower: CodeOffset, upper: CodeOffset) -> Self {
//         let block = code[usize::from(lower)..=usize::from(upper)].to_vec();
//         Self {}
//     }
// }

struct InstructionReordering();

impl InstructionReordering {
    pub fn compute_reordered_instructions(target: &FunctionTarget) -> Option<ReorderedFunction> {
        let code = target.get_bytecode();
        if code.iter().any(|instr| instr.is_spec_only()) {
            return None;
        }
        let cfg = StacklessControlFlowGraph::new_forward(code);
        // let live_vars = target
        //     .get_annotations()
        //     .get::<LiveVarAnnotation>()
        //     .expect("live variable annotation is a prerequisite");
        let mut block_ranges = cfg
            .blocks()
            .iter()
            .filter_map(|block_id| cfg.instr_offset_bounds(*block_id))
            .collect::<Vec<_>>();
        // TODO: Can be skipped if `block_ranges` are guaranteed to be already sorted.
        block_ranges.sort_by_key(|k| k.0);
        let mut new_code = vec![];
        let mut ordering_annotation = OrderingAnnotation(BTreeMap::new());
        let mut touch_use_annotation = PrepareUseAnnotation(BTreeMap::new());
        for (lower, upper) in block_ranges {
            let ReorderedBlock {
                block,
                ordering,
                touch_use,
            } = Self::optimize_block_for_stack_machine(code, lower, upper, target);
            let new_lower = new_code.len() as CodeOffset;
            new_code.extend(block);
            for (offset, order_info) in ordering.0.into_iter() {
                ordering_annotation.0.insert(
                    new_lower + offset as CodeOffset,
                    order_info.remap(new_lower),
                );
            }
            for (touch_offset, tuple) in touch_use.0 {
                touch_use_annotation
                    .0
                    .insert(new_lower + touch_offset, (new_lower + tuple.0, tuple.1));
            }
        }
        Some(ReorderedFunction {
            code: new_code,
            ordering: ordering_annotation,
            touch_use: touch_use_annotation,
        })
    }

    fn optimize_block_for_stack_machine(
        code: &[Bytecode],
        lower: CodeOffset,
        upper: CodeOffset,
        target: &FunctionTarget,
    ) -> ReorderedBlock {
        let mut new_block = code[usize::from(lower)..=usize::from(upper)].to_vec();
        // If there are any spec blocks, we do not perform any optimizations, as dependencies
        // in spec blocks are not captured. We could relax this limitation in the future.
        if new_block.len() > 256
            || new_block.iter().any(|instr| {
                instr.is_spec_only()
                    || matches!(instr, Bytecode::SpecBlock(..))
                    || matches!(instr, Bytecode::Call(_, _, _, _, Some(_)))
            })
        {
            return {
                ReorderedBlock {
                    block: new_block, // No reordering or insertion of `Prepare`.
                    ordering: OrderingAnnotation(BTreeMap::new()),
                    touch_use: PrepareUseAnnotation(BTreeMap::new()),
                }
            };
        }
        // Compute the use-def graph for this block.
        let (use_def_graph, mut prepare_use_map) =
            Self::ordered_edge_data_dependence_graph(&mut new_block);
        let dependencies = DependenceConstraints::empty()
            .add_false_dependencies(&new_block)
            .add_true_dependencies(&use_def_graph)
            .add_ref_arg_dependencies(&new_block, target)
            .add_move_dependencies(&new_block)
            .add_relatively_non_reorderable_dependencies(&new_block)
            .make_transitively_closed()
            .get_constraints();

        // None are marked as taken initially. (`taken`: bool vector)
        // Start from bottom.
        // It is marked as taken? Skip. Go to upper.
        // Is it Prepare? Skip. Go to upper.
        // Is is empty-sourced? Add it to stack (`new_code`). Mark as taken. Go to upper.
        // Else, start an ordered DFS numbering (dfs with lowest offset tracking), assert that none of them are `taken`.
        //   Find the lowest numbered instruction. Also note that prepare comes later!!
        //   Starting from there, for each numbered pair till this, check if constraints are satisfied.
        //   For each unmarked, check if constraints are satisfied with each numbered, assuming unmarked goes before.
        //   If yes, leave all the unmarked, copy all numbered to the stack (in reverse order). Mark them as taken.
        //   If not, add it to the stack. Go to upper.
        //
        // Note: for numbered pairs, data dependence is always satisfied. For unmarked, it may not be.
        // Number of instructions cannot grow (to make sure there are no double copies).
        // Note on Prepare instructions:
        //   When we have ordered a DFS numbering, check for each `Prepare` if:
        //   - from `Prepare x` to its use, there are no `&mut x` or `freeze_ref x`; if there are, skip it.
        //   - from `Prepare x` to its use, there are no reads of `x`, if there are,
        //     insert a `copy_only` hint for that `Prepare`. Copy only hints make `Prepare` a no-op
        //     if copy is not possible.
        let mut taken = vec![false; new_block.len()];
        let mut reordered_block = vec![];
        let ref_args = Self::get_ref_args(&new_block, target);
        let reads = Self::get_reads(&new_block);
        let prepares = Self::get_prepares(&new_block);
        for (offset, instr) in new_block.iter().enumerate().rev() {
            if taken[offset] {
                continue;
            }
            if let Bytecode::Call(_, _, Operation::Prepare, ..) = instr {
                continue;
            }
            if instr.sources().is_empty() {
                // May not be needed, as it is covered by the DFS case.
                reordered_block.push((offset, instr.clone()));
                taken[offset] = true;
                continue;
            }
            let ordered_offsets = Self::dfs_post_order(&use_def_graph, offset as CodeOffset);
            let (valid, ordered_offsets, copy_only_prepares) = Self::is_valid_order(
                ordered_offsets,
                offset as CodeOffset,
                &dependencies,
                &ref_args,
                &reads,
                &prepares,
            );
            if valid {
                for off in ordered_offsets.into_iter().rev() {
                    let off = usize::from(off);
                    reordered_block.push((off, new_block[off].clone()));
                    taken[off] = true;
                }
                for copy_only_offset in copy_only_prepares {
                    prepare_use_map
                        .get_mut(&copy_only_offset)
                        .expect("prepare offset")
                        .1 = true;
                }
            } else {
                reordered_block.push((offset, instr.clone()));
                taken[offset] = true;
            }
        }
        reordered_block.reverse();
        let remap = reordered_block
            .iter()
            .enumerate()
            .map(|(i, (off, _))| (*off as CodeOffset, i as CodeOffset))
            .collect::<BTreeMap<_, _>>();
        let prepare_use_map = prepare_use_map.into_iter().filter_map(|(p, u)| {
            remap
                .get(&p)
                .map(|new_p| (*new_p, (*remap.get(&u.0).expect("existing offset"), u.1)))
        });
        // Start DFS port-order numbering from unvisited relatively immovable instructions.
        // Iteration is in reverse direction from the end of the block.
        // let dfs_numberings = Self::dfs_post_order_numbering(&new_block, &use_def_graph);
        // let constraints = OrderingConstraints {
        //     dependencies,
        //     dfs_numberings,
        // };
        // let reordered_indices = constraints.get_ordered_instr_indices();
        // // Re-order the instructions in the block based on ordering (after sort).
        // let reordered_block = reordered_indices
        //     .iter()
        //     .map(|v| new_block[usize::from(*v)].clone())
        //     .collect::<Vec<_>>();
        // let mut index_remapping = vec![0; reordered_indices.len()];
        // for (i, reordered_index) in reordered_indices.iter().enumerate() {
        //     index_remapping[usize::from(*reordered_index)] = i as CodeOffset;
        // }
        // let prepare_use_map = prepare_use_map
        //     .into_iter()
        //     .map(|(k, v)| {
        //         (
        //             index_remapping[usize::from(k)],
        //             (index_remapping[usize::from(v.0)], v.1, v.2),
        //         )
        //     })
        //     .collect::<BTreeMap<_, _>>();
        ReorderedBlock {
            block: reordered_block
                .into_iter()
                .map(|(_, instr)| instr)
                .collect(),
            ordering: OrderingAnnotation(BTreeMap::new()),
            touch_use: PrepareUseAnnotation(prepare_use_map.collect()),
        }
    }

    fn dfs_post_order(graph: &UseDefGraph, node: CodeOffset) -> Vec<CodeOffset> {
        let mut visited = BTreeSet::new();
        let mut post_order = vec![];
        Self::dfs_post_order_recurse(node, graph, &mut visited, &mut post_order);
        post_order
    }

    fn dfs_post_order_recurse(
        node: CodeOffset,
        graph: &UseDefGraph,
        visited: &mut BTreeSet<CodeOffset>,
        post_order: &mut Vec<CodeOffset>,
    ) {
        if !visited.insert(node) {
            return;
        }
        for dependent in graph
            .0
            .get(&node)
            .map(|deps| deps.iter().filter_map(|d| *d).collect::<Vec<_>>())
            .unwrap_or_default()
        {
            Self::dfs_post_order_recurse(dependent, graph, visited, post_order);
        }
        post_order.push(node);
    }

    fn get_ref_args(
        block: &[Bytecode],
        target: &FunctionTarget,
    ) -> BTreeMap<TempIndex, BTreeSet<CodeOffset>> {
        let mut ref_args: BTreeMap<TempIndex, BTreeSet<CodeOffset>> = BTreeMap::new();
        for (offset, instr) in block.iter().enumerate() {
            let offset = offset as CodeOffset;
            if matches!(instr, Bytecode::Call(_, _, Operation::Prepare, ..)) {
                // `Prepare` is always inserted at the end.
                break;
            }
            if is_ref_arg_instr(instr, target) {
                for src in instr.sources() {
                    ref_args.entry(src).or_default().insert(offset);
                }
            }
        }
        ref_args
    }

    fn get_reads(block: &[Bytecode]) -> BTreeMap<TempIndex, BTreeSet<CodeOffset>> {
        let mut reads: BTreeMap<TempIndex, BTreeSet<CodeOffset>> = BTreeMap::new();
        for (offset, instr) in block.iter().enumerate() {
            let offset = offset as CodeOffset;
            if matches!(instr, Bytecode::Call(_, _, Operation::Prepare, ..)) {
                // `Prepare` is always inserted at the end.
                break;
            }
            for src in instr.sources() {
                reads.entry(src).or_default().insert(offset);
            }
        }
        reads
    }

    fn get_prepares(block: &[Bytecode]) -> BTreeMap<CodeOffset, TempIndex> {
        let mut prepares: BTreeMap<CodeOffset, TempIndex> = BTreeMap::new();
        for (offset, instr) in block.iter().enumerate() {
            let offset = offset as CodeOffset;
            if let Bytecode::Call(_, _, Operation::Prepare, sources, _) = instr {
                prepares.insert(offset, sources[0]);
            }
        }
        prepares
    }

    fn is_valid_order(
        offsets: Vec<CodeOffset>,
        node: CodeOffset,
        dependencies: &BTreeMap<CodeOffset, BTreeSet<CodeOffset>>,
        ref_args: &BTreeMap<TempIndex, BTreeSet<CodeOffset>>,
        reads: &BTreeMap<TempIndex, BTreeSet<CodeOffset>>,
        prepares: &BTreeMap<CodeOffset, TempIndex>,
    ) -> (bool, Vec<CodeOffset>, BTreeSet<CodeOffset>) {
        assert!(offsets.contains(&node));
        for i in 0..offsets.len() {
            for j in i + 1..offsets.len() {
                let before = offsets[i];
                let after = offsets[j];
                if dependencies
                    .get(&after)
                    .is_some_and(|nodes| nodes.contains(&before))
                {
                    return (false, vec![], BTreeSet::new());
                }
            }
        }
        let seen = offsets.iter().collect::<BTreeSet<_>>();
        let min = **seen.first().expect("at least one offset");
        for i in min..=node {
            if !seen.contains(&i) {
                // `i` is not visited.
                // Is it safe to move `i` before everything else?
                for o in offsets.iter() {
                    if dependencies
                        .get(o)
                        .map_or(false, |nodes| nodes.contains(&i))
                    {
                        return (false, vec![], BTreeSet::new());
                    }
                }
            }
        }
        let mut skip_prepares = BTreeSet::new();
        let mut copy_only = BTreeSet::new();
        for (i, offset) in offsets.iter().enumerate() {
            if let Some(prepared_tmp) = prepares.get(offset) {
                // This is a `Prepare` instruction.
                let use_offset = dependencies
                    .get(offset)
                    .expect("Prepare must have a use")
                    .first()
                    .expect("Prepare must have exactly one use");
                let mut j = i + 1;
                while j < offsets.len() {
                    let scanned_offset = offsets[j];
                    if scanned_offset == *use_offset {
                        break;
                    }
                    j += 1;
                    if ref_args
                        .get(prepared_tmp)
                        .map_or(false, |nodes| nodes.contains(&scanned_offset))
                    {
                        // skip the `offset`
                        copy_only.remove(offset);
                        skip_prepares.insert(*offset);
                        break;
                    }
                    if reads
                        .get(prepared_tmp)
                        .map_or(false, |nodes| nodes.contains(&scanned_offset))
                    {
                        // insert a `copy_only` hint for the `Prepare`.
                        copy_only.insert(*offset);
                        break;
                    }
                }
            }
        }
        let new_offsets = offsets
            .into_iter()
            .filter(|o| !skip_prepares.contains(o))
            .collect::<Vec<_>>();
        (true, new_offsets, copy_only)
    }

    fn ordered_edge_data_dependence_graph(
        block: &mut Vec<Bytecode>,
    ) -> (UseDefGraph, BTreeMap<CodeOffset, (CodeOffset, bool)>) {
        // Map a temp to the offset of its latest write.
        let mut latest_write: BTreeMap<TempIndex, CodeOffset> = BTreeMap::new();
        let mut uses: BTreeMap<(TempIndex, Option<CodeOffset>), BTreeSet<(CodeOffset, usize)>> =
            BTreeMap::new();
        let mut graph: BTreeMap<CodeOffset, Vec<Option<CodeOffset>>> = BTreeMap::new();
        // Create a basic ordered-edge data dependence graph, without any `Prepare` instructions.
        for (offset, instr) in block.iter().enumerate() {
            let offset = offset as CodeOffset;
            let sources = instr.sources();
            if !sources.is_empty() {
                // to avoid unnecessary entry.
                let edges = graph.entry(offset).or_default();
                for (pos, src) in sources.iter().enumerate() {
                    let def_offset = latest_write.get(src).copied();
                    edges.push(def_offset);
                    uses.entry((*src, def_offset))
                        .or_default()
                        .insert((offset, pos));
                }
            }
            for dest in instr.dests() {
                latest_write.insert(dest, offset);
            }
        }
        // Insert `Prepare` instructions (which attempt to bring a value onto the top
        // of the stack) in the following cases, only for sources that are not the last:
        // 1. If a source is not defined in the block (then it is definitely not on
        //    the stack).
        // 2. If a source is defined by an `Assign` (then the assignment has taken
        //    it off the stack).
        // 3. If a source's definition is used multiple times (then the first use
        //    will take it off the stack, but re-ordering might change which is the
        //    first use).
        // In addition, also update the `graph`.
        let mut prepare_use_map: BTreeMap<CodeOffset, (CodeOffset, bool)> = BTreeMap::new();
        let mut prepare_instrs: Vec<Bytecode> = vec![];
        // let mut prepare_edges: BTreeMap<CodeOffset, CodeOffset> = BTreeMap::new();
        for (usage_offset, usage_instr) in block.iter().enumerate() {
            let sources = usage_instr.sources();
            if sources.len() < 2 {
                // No need to insert `Prepare` for single source instructions.
                continue;
            }
            let usage_offset = usage_offset as CodeOffset;
            if let Some(defs) = graph.get_mut(&usage_offset) {
                // We do not have to `Prepare` the last operand, as it can be brought
                // to the top of the stack when actually needed by the use.
                let without_last_len = if defs.is_empty() { 0 } else { defs.len() - 1 };
                for (pos, (def, tmp)) in defs
                    .iter_mut()
                    .zip(sources.iter())
                    .take(without_last_len)
                    .enumerate()
                {
                    match def {
                        None => {
                            // The definition is not explicit in the block.
                            // So, let's insert a `Prepare` instruction.
                            let prepare_offset = (block.len() + prepare_instrs.len()) as CodeOffset;
                            let prepare = Bytecode::Call(
                                usage_instr.get_attr_id(),
                                vec![],
                                Operation::Prepare,
                                vec![*tmp],
                                None,
                            );
                            prepare_instrs.push(prepare);
                            *def = Some(prepare_offset);
                            prepare_use_map.insert(prepare_offset, (usage_offset, false));
                        },
                        Some(_def_offset) => {
                            // The definition is explicit in the block.
                            // If it is an `Assign`, then we need to insert a `Prepare`.
                            // let def_instr = &block[usize::from(*def_offset)];
                            // if matches!(def_instr, Bytecode::Assign(..)) {
                            //     let prepare_offset =
                            //         (block.len() + prepare_instrs.len()) as CodeOffset;
                            //     let prepare = Bytecode::Call(
                            //         usage_instr.get_attr_id(),
                            //         vec![],
                            //         Operation::Prepare,
                            //         vec![*tmp],
                            //         None,
                            //     );
                            //     prepare_instrs.push(prepare);
                            //     prepare_edges.insert(prepare_offset, *def_offset);
                            //     *def = Some(prepare_offset);
                            //     prepare_use_map.insert(prepare_offset, (usage_offset, pos, false));
                            // }
                        },
                    }
                }
            }
        }
        // for ((tmp, _), use_pairs) in uses {
        //     if use_pairs.len() > 1 {
        //         // The definition is used multiple times.
        //         // So, let's insert a `Prepare` instruction for each use,
        //         // except for the last one, unless one is already present.
        //         for (use_offset, pos) in use_pairs {
        //             // All use instructions are guaranteed to be within the `block`.
        //             // Because, `Prepare` instructions were not considered in `uses`.
        //             let use_instr = &block[use_offset as usize];
        //             let sources = use_instr.sources();
        //             if sources.is_empty() || pos == sources.len() - 1 {
        //                 // The last operand does not need a `Prepare`.
        //                 continue;
        //             }
        //             if let Some(defs) = graph.get_mut(&use_offset) {
        //                 if let Some(def) = defs[pos] {
        //                     if def as usize >= block.len() {
        //                         // `Prepare` instruction.
        //                         // No need to insert a `Prepare` instruction.
        //                         // Just update the `prepare_use_map` to say this is a multi-use.
        //                         if let Some(triplet) = prepare_use_map.get_mut(&def) {
        //                             triplet.2 = true;
        //                         }
        //                     } else {
        //                         // Not a `Prepare` instruction.
        //                         let prepare_offset =
        //                             (block.len() + prepare_instrs.len()) as CodeOffset;
        //                         let prepare = Bytecode::Call(
        //                             use_instr.get_attr_id(),
        //                             vec![],
        //                             Operation::Prepare,
        //                             vec![tmp],
        //                             None,
        //                         );
        //                         prepare_instrs.push(prepare);
        //                         prepare_edges.insert(prepare_offset, def);
        //                         defs[pos] = Some(prepare_offset);
        //                         prepare_use_map.insert(prepare_offset, (use_offset, pos, true));
        //                     }
        //                 }
        //             }
        //         }
        //     }
        // }
        block.extend(prepare_instrs);
        // for (prepare_offset, def_offset) in prepare_edges {
        //     graph
        //         .entry(prepare_offset)
        //         .or_default()
        //         .push(Some(def_offset));
        // }
        (UseDefGraph(graph), prepare_use_map)
    }

    fn _use_def_graph_for_block(
        code: &[Bytecode],
        lower: CodeOffset,
        upper: CodeOffset,
        live_vars: &LiveVarAnnotation,
    ) -> UseDefGraph {
        let mut use_def_graph = BTreeMap::new();
        for usage_offset in lower..=upper {
            let adjusted_usage_offset = usage_offset - lower;
            let usage_instr = &code[usage_offset as usize];
            use_def_graph
                .entry(adjusted_usage_offset)
                .or_insert(vec![None; usage_instr.sources().len()]);
        }
        for def_offset in lower..=upper {
            // Only considering definitions inside the block.
            for (temp_defined, info) in live_vars.get_info_at(def_offset).after.iter() {
                for usage_offset in info.usage_offsets() {
                    if usage_offset < lower || usage_offset > upper {
                        // Usage is outside of the block, ignore.
                        continue;
                    }
                    let usage_instr = &code[usage_offset as usize];
                    let defining_instr = &code[def_offset as usize];
                    let sources = usage_instr.sources();
                    let dests = defining_instr.dests();
                    if dests.contains(temp_defined) && !sources.is_empty() {
                        if usage_offset <= def_offset {
                            // The usage is before the definition in the block.
                            // Thus, it is must be a loop carried dependency, which we will ignore.
                            continue;
                        }
                        let adjusted_usage_offset = usage_offset - lower;
                        let adjusted_def_offset = def_offset - lower;
                        for (pos, tmp_used) in sources.iter().enumerate() {
                            if tmp_used == temp_defined {
                                use_def_graph
                                    .entry(adjusted_usage_offset)
                                    .and_modify(|edges| edges[pos] = Some(adjusted_def_offset));
                            }
                        }
                    }
                }
            }
        }
        UseDefGraph(use_def_graph)
    }

    fn _insert_prepares(
        block: &mut Vec<Bytecode>,
        use_def_graph: &mut UseDefGraph,
    ) -> BTreeMap<CodeOffset, CodeOffset> {
        // Mapping from the `Prepare` instructions to the corresponding use instructions
        // that they are preparing for.
        let mut prepare_use_map = BTreeMap::new();
        // Newly inserted `Prepare` instructions.
        let mut prepare_instrs = vec![];
        for (usage_offset, usage_instr) in block.iter().enumerate() {
            // If a `usage_instr` has more than 1 sources, then we may need to insert a `Prepare`.
            // Note that `BorrowLoc` does not need its operand on the stack, so it does not need
            // a `Prepare`. However, it also happens to have only one source, so it is disregarded
            // by the following match.
            match usage_instr {
                Bytecode::Call(_, _, _, sources, _) | Bytecode::Ret(_, sources)
                    if sources.len() > 1 => {},
                _ => {
                    continue;
                },
            }
            let usage_offset = usage_offset as CodeOffset;
            if let Some(defs) = use_def_graph.0.get_mut(&usage_offset) {
                // We do not have to `Prepare` the last operand.
                let without_last_len = if defs.is_empty() { 0 } else { defs.len() - 1 };
                let sources = usage_instr.sources();
                for (def, tmp) in defs.iter_mut().zip(sources.iter()).take(without_last_len) {
                    if def.is_none() {
                        // The definition is not explicit in the block.
                        // So, let's insert a `Prepare` instruction.
                        let prepare_offset = (block.len() + prepare_instrs.len()) as CodeOffset;
                        let prepare = Bytecode::Call(
                            usage_instr.get_attr_id(),
                            vec![],
                            Operation::Prepare,
                            vec![*tmp],
                            None,
                        );
                        prepare_instrs.push(prepare);
                        *def = Some(prepare_offset);
                        prepare_use_map.insert(prepare_offset, usage_offset);
                    }
                }
            }
        }
        block.extend(prepare_instrs);
        prepare_use_map
    }

    fn dfs_post_order_numbering(
        block: &[Bytecode],
        graph: &UseDefGraph,
    ) -> Vec<Vec<Option<CodeOffset>>> {
        let mut true_dependencies = vec![vec![]; block.len()];
        let mut visited_by_any_run: BTreeSet<CodeOffset> = BTreeSet::new();
        for (offset, instr) in block.iter().enumerate().rev() {
            if !visited_by_any_run.contains(&(offset as CodeOffset))
                // && Self::is_relatively_non_reorderable(instr)
                && !instr.sources().is_empty()
            {
                let mut visited_by_this_run = BTreeSet::new();
                Self::dfs_recurse(
                    offset as CodeOffset,
                    graph,
                    &mut visited_by_this_run,
                    &mut true_dependencies,
                    &mut 0,
                );
                // Any instruction that was not numbered by the above run of DFS,
                // should be numbered `None`.
                let max_len = true_dependencies[offset].len();
                for order in true_dependencies.iter_mut() {
                    if order.len() < max_len {
                        order.push(None);
                    }
                }
                visited_by_any_run.append(&mut visited_by_this_run);
            }
        }
        true_dependencies
    }

    fn dfs_recurse(
        node: CodeOffset,
        graph: &UseDefGraph,
        visited: &mut BTreeSet<CodeOffset>,
        ordering: &mut Vec<Vec<Option<CodeOffset>>>,
        num: &mut u16,
    ) {
        if !visited.insert(node) {
            return;
        }
        for dependent in graph
            .0
            .get(&node)
            .map(|deps| deps.iter().filter_map(|d| *d).collect::<Vec<_>>())
            .unwrap_or_default()
        {
            Self::dfs_recurse(dependent, graph, visited, ordering, num);
        }
        ordering[usize::from(node)].push(Some(*num));
        *num += 1;
    }

    fn is_relatively_non_reorderable(instr: &Bytecode) -> bool {
        use Bytecode::*;
        use Operation::*;
        match instr {
            Ret(..) | Branch(..) | Jump(..) | Label(..) | Abort(..) => true,
            Call(_, _, op, ..) => {
                op.can_abort() || matches!(op, WriteRef | ReadRef | FreezeRef(_) | Drop)
            },
            _ => false,
        }
    }
}

#[derive(Default)]
struct DependenceConstraints {
    edges: BTreeMap<CodeOffset, BTreeSet<CodeOffset>>,
    num_nodes: CodeOffset,
}

impl DependenceConstraints {
    fn empty() -> Self {
        Self::default()
    }

    /// Compute the false dependence graph for a `block` (of straight-line code).
    /// A false dependence graph includes write-after-read and write-after-write dependencies.
    /// Since it is computed within a block, it is a directed acyclic graph (by construction).
    fn add_false_dependencies(&mut self, block: &[Bytecode]) -> &mut Self {
        self.num_nodes = block.len() as CodeOffset;
        // Track all the reads of a tmp before a write to it.
        let mut reads_before: BTreeMap<TempIndex, BTreeSet<CodeOffset>> = BTreeMap::new();
        // Track the most recent write to a tmp.
        let mut latest_write: BTreeMap<TempIndex, CodeOffset> = BTreeMap::new();
        for (offset, instr) in block.iter().enumerate() {
            if matches!(instr, Bytecode::Call(_, _, Operation::Prepare, ..)) {
                // `Prepare` is always inserted at the end.
                // We are not adding false dependencies for `Prepare` instructions.
                break;
            }
            let offset = offset as CodeOffset;
            for tmp in instr.sources() {
                reads_before.entry(tmp).or_default().insert(offset);
            }
            for tmp in instr.dests() {
                if let Some(nodes) = reads_before.remove(&tmp) {
                    // Add write-after-read dependencies.
                    for node in nodes.iter().filter(|n| **n != offset) {
                        self.edges.entry(*node).or_default().insert(offset);
                    }
                }
                if let Some(node) = latest_write.insert(tmp, offset) {
                    if node != offset {
                        // Add write-after-write dependencies.
                        self.edges.entry(node).or_default().insert(offset);
                    }
                }
            }
        }
        self
    }

    fn add_true_dependencies(&mut self, use_def_graph: &UseDefGraph) -> &mut Self {
        for (use_offset, def_offsets) in use_def_graph.0.iter() {
            for def_offset in def_offsets.iter().filter_map(|d| *d) {
                self.edges
                    .entry(def_offset)
                    .or_default()
                    .insert(*use_offset);
            }
        }
        self
    }

    fn add_ref_arg_dependencies(
        &mut self,
        block: &[Bytecode],
        target: &FunctionTarget,
    ) -> &mut Self {
        let mut reads: BTreeMap<TempIndex, BTreeSet<CodeOffset>> = BTreeMap::new();
        let mut ref_args: BTreeMap<TempIndex, CodeOffset> = BTreeMap::new();
        for (offset, instr) in block.iter().enumerate() {
            let offset = offset as CodeOffset;
            if matches!(instr, Bytecode::Call(_, _, Operation::Prepare, ..)) {
                // `Prepare` is always inserted at the end.
                // We are not adding ref-arg dependencies for `Prepare` instructions.
                break;
            }
            if is_ref_arg_instr(instr, target) {
                for src in instr.sources() {
                    if let Some(prev_reads) = reads.remove(&src) {
                        for prev_read in prev_reads {
                            self.edges.entry(prev_read).or_default().insert(offset);
                        }
                    }
                    if let Some(prev_ref_arg) = ref_args.insert(src, offset) {
                        self.edges.entry(prev_ref_arg).or_default().insert(offset);
                    }
                }
            } else {
                for src in instr.sources() {
                    reads.entry(src).or_default().insert(offset);
                    if let Some(prev_ref_arg_offset) = ref_args.get(&src) {
                        self.edges
                            .entry(*prev_ref_arg_offset)
                            .or_default()
                            .insert(offset);
                    }
                }
            }
        }
        self
    }

    fn add_move_dependencies(&mut self, block: &[Bytecode]) -> &mut Self {
        let mut reads_before_move: BTreeMap<TempIndex, BTreeSet<CodeOffset>> = BTreeMap::new();
        use AssignKind::*;
        for (offset, instr) in block.iter().enumerate() {
            if matches!(instr, Bytecode::Call(_, _, Operation::Prepare, ..)) {
                // `Prepare` is always inserted at the end.
                // We are not adding false dependencies for `Prepare` instructions.
                break;
            }
            if let Bytecode::Assign(_, _, src, Move | Inferred | Store) = instr {
                if let Some(reads) = reads_before_move.remove(src) {
                    for read in reads {
                        self.edges
                            .entry(read)
                            .or_default()
                            .insert(offset as CodeOffset);
                    }
                }
            } else {
                for src in instr.sources() {
                    reads_before_move
                        .entry(src)
                        .or_default()
                        .insert(offset as CodeOffset);
                }
            }
        }
        self
    }

    fn add_relatively_non_reorderable_dependencies(&mut self, block: &[Bytecode]) -> &mut Self {
        let mut prev_offset = None;
        for (offset, instr) in block.iter().enumerate() {
            if InstructionReordering::is_relatively_non_reorderable(instr) {
                let offset = offset as CodeOffset;
                if let Some(prev_offset) = prev_offset {
                    self.edges.entry(prev_offset).or_default().insert(offset);
                }
                prev_offset = Some(offset);
            }
        }
        self
    }

    fn make_transitively_closed(&mut self) -> &mut Self {
        assert!(!self.has_cycle());
        // Floyd-Warshall algorithm to compute the transitive closure.
        // TODO: Consider using a more efficient algorithm if this is a fairly sparse graph.
        for k in 0..self.num_nodes {
            for i in 0..self.num_nodes {
                for j in 0..self.num_nodes {
                    if self.edges.get(&i).map_or(false, |nodes| nodes.contains(&k))
                        && self.edges.get(&k).map_or(false, |nodes| nodes.contains(&j))
                    {
                        self.edges.entry(i).or_default().insert(j);
                    }
                }
            }
        }
        self
    }

    pub fn get_constraints(&mut self) -> BTreeMap<CodeOffset, BTreeSet<CodeOffset>> {
        mem::take(self).edges
    }

    fn has_cycle(&self) -> bool {
        let mut visited_ever = BTreeSet::new();
        let mut ancestors = BTreeSet::new();
        for node in 0..self.num_nodes {
            if self.dfs(node, &mut visited_ever, &mut ancestors) {
                return true;
            }
        }
        false
    }

    fn dfs(
        &self,
        node: CodeOffset,
        visited_ever: &mut BTreeSet<CodeOffset>,
        ancestors: &mut BTreeSet<CodeOffset>,
    ) -> bool {
        if !visited_ever.insert(node) {
            return false;
        }
        ancestors.insert(node);
        if let Some(children) = self.edges.get(&node) {
            for child in children {
                if ancestors.contains(child) || self.dfs(*child, visited_ever, ancestors) {
                    return true;
                }
            }
        }
        ancestors.remove(&node);
        false
    }
}

fn is_ref_arg_instr(instr: &Bytecode, target: &FunctionTarget) -> bool {
    use Operation::*;
    match instr {
        Bytecode::Call(_, dsts, BorrowLoc, _, _) => {
            true
            // target.get_local_type(dsts[0]).is_mutable_reference()
        },
        Bytecode::Call(_, _, op, _, _) => {
            matches!(
                op,
                FreezeRef(_) | WriteRef | BorrowGlobal(..) | BorrowField(..)
            )
        },
        _ => false,
    }
}

#[derive(Debug)]
struct OrderingConstraints {
    dependencies: BTreeMap<CodeOffset, BTreeSet<CodeOffset>>,
    dfs_numberings: Vec<Vec<Option<CodeOffset>>>,
}

impl OrderingConstraints {
    pub fn get_ordered_instr_indices(&self) -> Vec<CodeOffset> {
        let mut order = (0..self.dfs_numberings.len() as CodeOffset).collect::<Vec<_>>();
        order.sort_by(|a, b| {
            // If there is a dependence between `a` and `b`, then ordering should respect it.
            if self
                .dependencies
                .get(a)
                .is_some_and(|nodes| nodes.contains(b))
            {
                return std::cmp::Ordering::Less;
            } else if self
                .dependencies
                .get(b)
                .is_some_and(|nodes| nodes.contains(a))
            {
                return std::cmp::Ordering::Greater;
            }
            // Try to order based on the true dependencies.
            for (a_num, b_num) in self.dfs_numberings[*a as usize]
                .iter()
                .zip(self.dfs_numberings[*b as usize].iter())
            {
                if let (Some(a_num), Some(b_num)) = (a_num, b_num) {
                    debug_assert!(a_num != b_num);
                    return a_num.cmp(b_num);
                }
            }
            self.dfs_numberings[*a as usize]
                .cmp(&self.dfs_numberings[*b as usize])
                .then(a.cmp(b))
        });
        order
    }

    pub fn remap_and_convert_to_annotation(mut self, remap: &[CodeOffset]) -> OrderingAnnotation {
        let mut ordering = BTreeMap::new();
        for (offset, dfs_numberings) in self.dfs_numberings.into_iter().enumerate() {
            let dependencies = self
                .dependencies
                .remove(&(offset as CodeOffset))
                .unwrap_or_default()
                .into_iter()
                .map(|d| remap[d as usize])
                .collect();
            let dfs_numberings = dfs_numberings
                .into_iter()
                .map(|num| num.map(|n| remap[n as usize]))
                .collect();
            ordering.insert(remap[offset] as CodeOffset, OrderInfo {
                dependencies,
                dfs_numberings,
            });
        }
        OrderingAnnotation(ordering)
    }
}
pub struct InstructionReorderingProcessor {}

impl FunctionTargetProcessor for InstructionReorderingProcessor {
    fn process(
        &self,
        _targets: &mut FunctionTargetsHolder,
        func_env: &FunctionEnv,
        mut data: FunctionData,
        _scc_opt: Option<&[FunctionEnv]>,
    ) -> FunctionData {
        if func_env.is_native() {
            return data;
        }
        let target = FunctionTarget::new(func_env, &data);
        if let Some(ReorderedFunction {
            code,
            ordering,
            touch_use,
        }) = InstructionReordering::compute_reordered_instructions(&target)
        {
            // Clear all previous annotations.
            data.annotations.clear();
            /*
            println!(
                "func: {}\n{}\n\n",
                func_env.get_name_str(),
                code.iter()
                    .enumerate()
                    .map(|(i, instr)| format!("{:?} - {:?}", instr, ordering.0.get(&(i as CodeOffset))))
                    .collect::<Vec<_>>()
                    .join("\n")
            );
            */
            data.code = code;
            data.annotations.set(ordering, true);
            data.annotations.set(touch_use, true);
        }
        data
    }

    fn name(&self) -> String {
        "InstructionReorderingProcessor".to_string()
    }
}

impl InstructionReorderingProcessor {
    pub fn register_formatters(target: &FunctionTarget) {
        target.register_annotation_formatter(Box::new(format_instruction_reordering_annotation));
    }
}

pub fn format_instruction_reordering_annotation(
    target: &FunctionTarget,
    code_offset: CodeOffset,
) -> Option<String> {
    let annot = target.get_annotations().get::<OrderingAnnotation>()?;
    let annot = annot.0.get(&code_offset)?;
    Some(format!(
        "deps: {:?}, dfs: {:?}",
        annot.dependencies, annot.dfs_numberings
    ))
}
