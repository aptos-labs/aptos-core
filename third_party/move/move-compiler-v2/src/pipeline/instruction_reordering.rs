// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::pipeline::livevar_analysis_processor::LiveVarAnnotation;
use move_binary_format::file_format::CodeOffset;
use move_model::model::FunctionEnv;
use move_stackless_bytecode::{
    function_target::{FunctionData, FunctionTarget},
    function_target_pipeline::{FunctionTargetProcessor, FunctionTargetsHolder},
    stackless_bytecode::{Bytecode, Operation},
    stackless_control_flow_graph::StacklessControlFlowGraph,
};
use std::collections::{BTreeMap, BTreeSet};

type OrderPriorities = Vec<Option<CodeOffset>>;

#[derive(Clone, Debug)]
pub struct OrderingAnnotation(pub BTreeMap<CodeOffset, OrderPriorities>);

/// Mapping from every touch instruction to the "use" instruction they correspond to.
#[derive(Clone, Debug)]
pub struct TouchUseAnnotation(pub BTreeMap<CodeOffset, CodeOffset>);

struct ReorderedBlock {
    block: Vec<Bytecode>,
    ordering: Option<Vec<OrderPriorities>>,
    touch_use: TouchUseAnnotation,
}

struct ReorderedFunction {
    code: Vec<Bytecode>,
    ordering: OrderingAnnotation,
    touch_use: TouchUseAnnotation,
}

struct UseDefGraph(pub BTreeMap<CodeOffset, Vec<Option<CodeOffset>>>);

struct AntiAndOutputDepGraph(pub BTreeMap<CodeOffset, BTreeSet<CodeOffset>>);

struct InstructionReordering();

impl InstructionReordering {
    pub fn compute_reordered_instructions(target: &FunctionTarget) -> ReorderedFunction {
        let code = target.get_bytecode();
        let cfg = StacklessControlFlowGraph::new_forward(code);
        let live_vars = target
            .get_annotations()
            .get::<LiveVarAnnotation>()
            .expect("live variable annotation is a prerequisite");
        let mut block_ranges = cfg
            .blocks()
            .iter()
            .filter_map(|block_id| cfg.instr_offset_bounds(*block_id))
            .collect::<Vec<_>>();
        block_ranges.sort_by_key(|k| k.0);
        let mut new_code = vec![];
        let mut ordering_annotation = OrderingAnnotation(BTreeMap::new());
        let mut touch_use_annotation = TouchUseAnnotation(BTreeMap::new());
        for (lower, upper) in block_ranges {
            let ReorderedBlock {
                block,
                ordering,
                touch_use,
            } = Self::optimize_block_for_stack_machine(code, lower, upper, live_vars);
            let new_lower = new_code.len() as CodeOffset;
            new_code.extend(block);
            if let Some(ordering) = ordering {
                for (offset, order_constraints) in ordering.into_iter().enumerate() {
                    ordering_annotation
                        .0
                        .insert(new_lower + offset as CodeOffset, order_constraints);
                }
            }
            for (touch_offset, use_offset) in touch_use.0 {
                touch_use_annotation
                    .0
                    .insert(new_lower + touch_offset, new_lower + use_offset);
            }
        }
        ReorderedFunction {
            code: new_code,
            ordering: ordering_annotation,
            touch_use: touch_use_annotation,
        }
    }

    fn dfs_post_order_numbering(
        block: &[Bytecode],
        graph: &UseDefGraph,
        ordering: &mut Vec<Vec<Option<CodeOffset>>>,
    ) {
        let mut visited = vec![false; block.len()];
        for (offset, instr) in block.iter().enumerate().rev() {
            if Self::is_relatively_immovable(instr) {
                Self::dfs_recurse(offset as CodeOffset, graph, &mut visited, ordering, &mut 0);
                // Any instruction that was not numbered by this DFS, should be numbered `None`.
                let max_len = ordering[offset].len();
                for order in ordering.iter_mut() {
                    if order.len() < max_len {
                        order.push(None);
                    }
                }
            }
        }
    }

    fn dfs_recurse(
        node: CodeOffset,
        graph: &UseDefGraph,
        visited: &mut [bool],
        ordering: &mut Vec<Vec<Option<CodeOffset>>>,
        num: &mut u16,
    ) {
        if visited[usize::from(node)] {
            return;
        }
        visited[usize::from(node)] = true;
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

    fn use_def_graph_for_block(
        code: &[Bytecode],
        lower: CodeOffset,
        upper: CodeOffset,
        live_vars: &LiveVarAnnotation,
    ) -> UseDefGraph {
        let mut use_def_graph = BTreeMap::new();
        for def_offset in lower..=upper {
            // Only considering definitions inside the block.
            for (temp_defined, info) in live_vars.get_info_at(def_offset).after.iter() {
                for usage_offset in info.usage_offsets() {
                    if usage_offset < lower || usage_offset > upper {
                        // Usage is outside of the block, ignore.
                        continue;
                    }
                    let usage_instr = &code[usage_offset as usize];
                    let sources = usage_instr.sources();
                    if !sources.is_empty() {
                        let pos = sources
                            .iter()
                            .position(|t| t == temp_defined)
                            .expect("temp_defined should be in sources");
                        let adjusted_usage_offset = usage_offset - lower;
                        let adjusted_def_offset = def_offset - lower;
                        let use_def_edges = use_def_graph
                            .entry(adjusted_usage_offset)
                            .or_insert(vec![None; sources.len()]);
                        use_def_edges[pos] = Some(adjusted_def_offset);
                    }
                }
            }
        }
        UseDefGraph(use_def_graph)
    }

    fn insert_touch_instructions(
        code: &[Bytecode],
        lower: CodeOffset,
        upper: CodeOffset,
        block: &mut Vec<Bytecode>,
        use_def_graph: &mut UseDefGraph,
    ) -> BTreeMap<CodeOffset, CodeOffset> {
        let mut touch_use_map = BTreeMap::new();
        for usage_offset in lower..=upper {
            if let Some(defs) = use_def_graph.0.get_mut(&usage_offset) {
                for (pos, def) in defs.iter_mut().enumerate() {
                    if def.is_none() {
                        // The definition is not explicit in the block.
                        // So, let's insert a `Touch` instruction.
                        // TODO: we should also add `Touch` when the definition is alive
                        // after the use, this brings back the temporary onto the stack.
                        let usage_instr = &code[usize::from(usage_offset)];
                        let temp = *usage_instr
                            .sources()
                            .get(pos)
                            .expect("source at this position must exist");
                        let touch_offset = block.len() as CodeOffset;
                        let touch = Bytecode::Call(
                            usage_instr.get_attr_id(),
                            Vec::new(),
                            Operation::Touch,
                            vec![temp],
                            None,
                        );
                        block.push(touch);
                        *def = Some(touch_offset);
                        touch_use_map.insert(touch_offset, usage_offset);
                    }
                }
            }
        }
        touch_use_map
    }

    fn optimize_block_for_stack_machine(
        code: &[Bytecode],
        lower: CodeOffset,
        upper: CodeOffset,
        live_vars: &LiveVarAnnotation,
    ) -> ReorderedBlock {
        let mut new_block = code[usize::from(lower)..=usize::from(upper)].to_vec();
        // If there are any spec blocks, we do not perform any optimizations, as data
        // dependencies and anti-dependencies in spec blocks are not captured.
        // We could relax this limitation in the future.
        if new_block.iter().any(|instr| instr.is_spec_only()) {
            return {
                ReorderedBlock {
                    block: new_block, // No reordering or insertion of `Touch`.
                    ordering: None,
                    touch_use: TouchUseAnnotation(BTreeMap::new()),
                }
            };
        }
        // Compute the use-def graph for this block.
        let mut use_def_graph = Self::use_def_graph_for_block(code, lower, upper, live_vars);
        // Insert `Touch` instructions as needed to the end and update the use-def graph.
        // `Touch` instructions will be re-ordered below, along with the rest of the instructions.
        let touch_use_map =
            Self::insert_touch_instructions(code, lower, upper, &mut new_block, &mut use_def_graph);
        // Number all the relatively immovable instructions, rest get `None`.
        // Iteration is in forward direction from the beginning of the block.
        let mut ordering = new_block
            .iter()
            .enumerate()
            .map(|(offset, instr)| {
                if Self::is_relatively_immovable(instr) {
                    vec![Some(offset as CodeOffset)]
                } else {
                    vec![None]
                }
            })
            .collect::<Vec<_>>();
        // TODO: Perform a DFS topological sort on the anti-and-output dependence graph (acyclic).
        // Start DFS port-order numbering from unvisited relatively immovable instructions.
        // Iteration is in reverse direction from the end of the block.
        Self::dfs_post_order_numbering(&new_block, &use_def_graph, &mut ordering);
        // Number the rest of the instructions based on the original order for tie-breaks.
        for (offset, order_constraints) in ordering.iter_mut().enumerate() {
            order_constraints.push(Some(offset as CodeOffset));
        }
        // Re-order the instructions in the block based on ordering (after sort).
        ordering.sort_by(|a, b| {
            for (p, q) in a.iter().zip(b.iter()) {
                if let (Some(p), Some(q)) = (p, q) {
                    return p.cmp(q);
                }
            }
            unreachable!("at least one comparison should have been Some-Some");
        });
        let reordered_block = ordering
            .iter()
            .map(|v| {
                let offset = v
                    .last()
                    .expect("there is always a last")
                    .expect("last is always Some");
                new_block[usize::from(offset)].clone()
            })
            .collect::<Vec<_>>();
        ReorderedBlock {
            block: reordered_block,
            ordering: Some(ordering),
            touch_use: TouchUseAnnotation(touch_use_map),
        }
    }

    fn is_relatively_immovable(instr: &Bytecode) -> bool {
        use Bytecode::*;
        use Operation::*;
        match instr {
            Ret(..) | Branch(..) | Jump(..) | Label(..) | Abort(..) => true,
            Call(_, _, op, ..) => op.can_abort() || matches!(op, WriteRef | ReadRef | Drop),
            _ => false,
        }
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
        let ReorderedFunction {
            code,
            ordering,
            touch_use,
        } = InstructionReordering::compute_reordered_instructions(&target);
        data.annotations.clear(); // Clear all previous annotations.
        data.code = code;
        data.annotations.set(ordering, true);
        data.annotations.set(touch_use, true);
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
    let ordering = annot
        .iter()
        .map(|x| format!("{:?}", x))
        .collect::<Vec<_>>()
        .join(", ");
    Some(format!("ordering: {ordering}"))
}
