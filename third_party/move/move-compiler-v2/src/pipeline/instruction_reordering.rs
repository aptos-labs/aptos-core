// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! In this module, we implement an intra-block instruction reordering optimization.
//! The goal of this reordering is to make the code friendlier for generating stack
//! machine bytecode.
//!
//! It is expected that this pass is run as the last transformation in the pipeline.
//! The prerequisite for this transformation is that all error checks have already been
//! performed and erroneous code has been rejected.
//!
//! In order to perform the reordering of instructions within a block, we construct
//! an edge-ordered data dependence graph. The data dependencies are use-def edges,
//! and are ordered based on the order of sources in the use instruction.
//!
//! There are a set of constraints that restrict what reorderings are safe to perform.
//! * True data dependencies (read-after-write) must be respected.
//! * False data dependencies (write-after-read and write-after-write) must be respected.
//! * Certain reference-related instructions (like freezing a reference) cannot be
//!   reordered with respect to any other reads of the reference.
//! * When a temp is moved, any reads of the temp cannot be reordered with respect to
//!   the move.
//! * Certain instructions are relatively non-reorderable. These cannot be reordered
//!   with respect to each other.
//!
//! We start from the bottom of a block, and perform a post-order depth-first search
//! on the edge-ordered data dependence graph and check if any of the constraints are
//! violated. If the constraints are not violated, we reorder the instructions.
//! Else, we move on to the instruction above.
//!
//! In addition to instruction reordering, this transformation also inserts `Prepare`
//! instructions and corresponding annotations. A `Prepare` instruction is instructs
//! the file format generator to prepare a value on the stack (i.e., move or copy it)
//! for a future use. A `Prepare` instruction can always be safely ignored.
//! Currently, a `Prepare` instruction is inserted when a source is not defined within
//! the block, and the source is not the last source of an instruction.

use move_binary_format::file_format::CodeOffset;
use move_model::{ast::TempIndex, model::FunctionEnv};
use move_stackless_bytecode::{
    function_target::{FunctionData, FunctionTarget},
    function_target_pipeline::{FunctionTargetProcessor, FunctionTargetsHolder},
    stackless_bytecode::{AssignKind, Bytecode, Operation},
    stackless_control_flow_graph::StacklessControlFlowGraph,
};
use std::collections::{BTreeMap, BTreeSet};

/// Mapping from every `Prepare` instruction's offset to the corresponding use
/// information (use of a `Prepare` instruction is the original instruction
/// for which we are preparing a value on the stack).
#[derive(Clone, Default)]
pub struct PrepareUseAnnotation(pub BTreeMap<CodeOffset, UseInfo>);

impl PrepareUseAnnotation {
    /// Remap the offsets in the `PrepareUseAnnotation` based on the given `mapping`.
    /// Note that `mapping` could be missing some offsets for `Prepare` instructions,
    /// in which case, the entries are dropped.
    /// However, `mapping` cannot be missing offsets for non-`Prepare` instructions
    /// (specifically, offsets for use instructions).
    pub fn filter_remap(&mut self, mapping: BTreeMap<CodeOffset, CodeOffset>) {
        self.0 = self
            .0
            .iter()
            .filter_map(|(prepare_offset, use_info)| {
                mapping.get(prepare_offset).map(|remapped_prepare_offset| {
                    (*remapped_prepare_offset, UseInfo {
                        offset: *mapping.get(&use_info.offset).expect("existing offset"),
                        copy_only: use_info.copy_only,
                    })
                })
            })
            .collect();
    }

    /// Extend with the `other` annotation.
    /// The `base_offset` is added to the instruction offsets in `other`.
    pub fn extend_with(&mut self, other: Self, base_offset: CodeOffset) {
        for (prepare_offset, use_info) in other.0.into_iter() {
            self.0.insert(base_offset + prepare_offset, UseInfo {
                offset: base_offset + use_info.offset,
                ..use_info
            });
        }
    }
}

/// Use information corresponding to a `Prepare` instruction.
/// It refers to the original instruction for which the `Prepare` instruction
/// is being inserted.
#[derive(Clone)]
pub struct UseInfo {
    // Offset of the original instruction for which the `Prepare` instruction is inserted.
    pub offset: CodeOffset,
    // Instructs the file format generator that when true, the corresponding `Prepare`
    // can only result in a copy (not a move). If a copy is not possible, then
    // the `Prepare` instruction should be a no-op.
    // When false, the `Prepare` instruction can result in a move or a copy.
    pub copy_only: bool,
}

/// Hold various information to safely perform reordering of instructions within a block.
struct BlockReordering {
    instructions: Vec<Bytecode>,
    dd_graph: DataDependenceGraph,
    prepare_use: PrepareUseAnnotation,
    constraints: BTreeMap<CodeOffset, BTreeSet<CodeOffset>>,
    original_block_len: usize,
}

impl BlockReordering {
    /// Given `instructions` of a basic block, insert required `Prepare` instructions
    /// and safely reorder the instructions when possible, to be friendlier for
    /// generating stack machine bytecode.
    pub fn prepare_and_reorder(instructions: Vec<Bytecode>) -> ReorderedInstructions {
        let original_block_len = instructions.len();
        let mut this = Self {
            instructions,
            dd_graph: DataDependenceGraph(BTreeMap::new()),
            prepare_use: PrepareUseAnnotation::default(),
            constraints: BTreeMap::new(),
            original_block_len,
        };
        this.fill_data_dependence_graph();
        this.fill_prepare_instructions();
        this.fill_constraints();
        this.reorder_instructions();

        ReorderedInstructions {
            instructions: this.instructions,
            prepare_use: this.prepare_use,
        }
    }

    /// Populate an edge-ordered data dependence graph based on original instructions
    /// in the basic block.
    fn fill_data_dependence_graph(&mut self) {
        // Map a temp to the offset of its latest write.
        let mut latest_write: BTreeMap<TempIndex, CodeOffset> = BTreeMap::new();
        let graph = &mut self.dd_graph.0;
        for (offset, instr) in self.instructions.iter().enumerate() {
            let offset = offset as CodeOffset;
            let sources = instr.sources();
            if !sources.is_empty() {
                let edges = graph.entry(offset).or_default();
                for src in sources.iter() {
                    let def_offset = latest_write.get(src).copied();
                    edges.push(def_offset);
                }
            }
            for dest in instr.dests() {
                latest_write.insert(dest, offset);
            }
        }
    }

    /// Insert `Prepare` instructions to the basic block.
    /// Also, fill the corresponding `prepare_use` annotations.
    fn fill_prepare_instructions(&mut self) {
        // When a source is not defined within the block, then insert a `Prepare` instruction.
        // Do this only when the source is not the last source of an instruction.
        let prepare_use_map = &mut self.prepare_use.0;
        let graph = &mut self.dd_graph.0;
        let mut prepare_instrs: Vec<Bytecode> = vec![];
        for (usage_offset, usage_instr) in self.instructions.iter().enumerate() {
            let sources = usage_instr.sources();
            if sources.len() < 2 {
                // No need to insert `Prepare` for non-multi-source instructions.
                continue;
            }
            let usage_offset = usage_offset as CodeOffset;
            if let Some(defs) = graph.get_mut(&usage_offset) {
                // We do not have to `Prepare` the last operand, as it can be brought
                // to the top of the stack when actually needed by the use.
                let without_last_len = if defs.is_empty() { 0 } else { defs.len() - 1 };
                for (def, tmp) in defs.iter_mut().zip(sources.iter()).take(without_last_len) {
                    if def.is_none() {
                        // The definition is not explicit in the block.
                        // So, let's insert a `Prepare` instruction.
                        let prepare_offset =
                            (self.original_block_len + prepare_instrs.len()) as CodeOffset;
                        let prepare = Bytecode::Call(
                            usage_instr.get_attr_id(),
                            vec![],
                            Operation::Prepare,
                            vec![*tmp],
                            None,
                        );
                        prepare_instrs.push(prepare);
                        *def = Some(prepare_offset);
                        prepare_use_map.insert(prepare_offset, UseInfo {
                            offset: usage_offset,
                            copy_only: false, // this may be updated later to true
                        });
                    }
                }
            }
        }
        self.instructions.extend(prepare_instrs);
    }

    /// Add all the constraints that restrict the reordering of instructions.
    /// All these constraints must be respected when reordering instructions.
    fn fill_constraints(&mut self) {
        self.constraints = DependenceConstraints::compute_from(self);
    }

    /// Perform instruction reordering using the edge-ordered data dependence graph,
    /// subject to the constraints.
    fn reorder_instructions(&mut self) {
        let mut reordered_block = vec![];
        // Tracking instruction offsets that have already been included in the reordered block.
        let mut taken = vec![false; self.instructions.len()];
        let ref_args = self.get_ref_args();
        let reads = self.get_reads();
        let prepares = self.get_prepares();
        // Start traversing from the bottom of the original block.
        for (offset, instr) in self
            .instructions
            .iter()
            .enumerate()
            .take(self.original_block_len)
            .rev()
        {
            if taken[offset] {
                // Already taken, skip this instruction.
                continue;
            }
            let ordered_offsets = self.dfs_post_order(offset as CodeOffset);
            if let Some((ordered_offsets, copy_only_prepares)) = self.adjusted_ordering(
                ordered_offsets,
                offset as CodeOffset,
                &ref_args,
                &reads,
                &prepares,
            ) {
                // We have a valid ordering that respects all constraints.
                // Instructions are added to the reordered block in reverse order.
                for off in ordered_offsets.into_iter().rev() {
                    let off = usize::from(off);
                    reordered_block.push((off, self.instructions[off].clone()));
                    taken[off] = true;
                }
                // Adjust the `copy_only` flag for the `Prepare` instructions in `copy_only_prepares`.
                for copy_only_offset in copy_only_prepares {
                    self.prepare_use
                        .0
                        .get_mut(&copy_only_offset)
                        .expect("prepare offset")
                        .copy_only = true;
                }
            } else {
                // Only the current instruction is added to the reordered block.
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
        self.prepare_use.filter_remap(remap);
        self.instructions = reordered_block
            .into_iter()
            .map(|(_, instr)| instr)
            .collect();
    }

    /// Perform a post-order depth-first search on the edge-ordered data dependence graph.
    /// TODO: this DFS traversal can be memoized.
    fn dfs_post_order(&self, start_node: CodeOffset) -> Vec<CodeOffset> {
        let mut visited = BTreeSet::new();
        let mut post_order = vec![];
        self.dfs_post_order_recurse(start_node, &mut visited, &mut post_order);
        post_order
    }

    /// Helper function for `dfs_post_order`.
    fn dfs_post_order_recurse(
        &self,
        node: CodeOffset,
        visited: &mut BTreeSet<CodeOffset>,
        post_order: &mut Vec<CodeOffset>,
    ) {
        if !visited.insert(node) {
            return;
        }
        for dependent in self
            .dd_graph
            .0
            .get(&node)
            .map(|deps| deps.iter().filter_map(|d| *d).collect::<Vec<_>>())
            .unwrap_or_default()
        {
            self.dfs_post_order_recurse(dependent, visited, post_order);
        }
        post_order.push(node);
    }

    /// Map temps to the set of code offsets where the temp is used as a source to a
    /// reference-related instruction.
    fn get_ref_args(&self) -> BTreeMap<TempIndex, BTreeSet<CodeOffset>> {
        let mut ref_args: BTreeMap<TempIndex, BTreeSet<CodeOffset>> = BTreeMap::new();
        for (offset, instr) in self
            .instructions
            .iter()
            .take(self.original_block_len)
            .enumerate()
        {
            let offset = offset as CodeOffset;
            if is_ref_related_instr(instr) {
                for src in instr.sources() {
                    ref_args.entry(src).or_default().insert(offset);
                }
            }
        }
        ref_args
    }

    /// Map temps to the set of code offsets where the temp is read.
    fn get_reads(&self) -> BTreeMap<TempIndex, BTreeSet<CodeOffset>> {
        let mut reads: BTreeMap<TempIndex, BTreeSet<CodeOffset>> = BTreeMap::new();
        for (offset, instr) in self
            .instructions
            .iter()
            .take(self.original_block_len)
            .enumerate()
        {
            let offset = offset as CodeOffset;
            for src in instr.sources() {
                reads.entry(src).or_default().insert(offset);
            }
        }
        reads
    }

    /// Map `Prepare` instruction offsets to the temps they prepare.
    fn get_prepares(&self) -> BTreeMap<CodeOffset, TempIndex> {
        let mut prepares: BTreeMap<CodeOffset, TempIndex> = BTreeMap::new();
        for (offset, instr) in self
            .instructions
            .iter()
            .enumerate()
            .skip(self.original_block_len)
        {
            let offset = offset as CodeOffset;
            if let Bytecode::Call(_, _, Operation::Prepare, sources, _) = instr {
                prepares.insert(offset, sources[0]);
            } else {
                unreachable!("only `Prepare` instructions are inserted after the original block");
            }
        }
        prepares
    }

    /// If the ordering provided by `offsets` is not valid, return `None`.
    /// Otherwise, return the adjusted ordering (with some `Prepare` instructions potentially
    /// removed) and the set of `Prepare` instructions that can only result in a copy.
    fn adjusted_ordering(
        &self,
        offsets: Vec<CodeOffset>,
        node: CodeOffset,
        ref_args: &BTreeMap<TempIndex, BTreeSet<CodeOffset>>,
        reads: &BTreeMap<TempIndex, BTreeSet<CodeOffset>>,
        prepares: &BTreeMap<CodeOffset, TempIndex>,
    ) -> Option<(Vec<CodeOffset>, BTreeSet<CodeOffset>)> {
        assert!(offsets.contains(&node));
        // Check that for each ordered pair of DFS-ordered offsets starting from `node`,
        // all the constraints are satisfied.
        for i in 0..offsets.len() {
            for j in i + 1..offsets.len() {
                let before = offsets[i];
                let after = offsets[j];
                if self
                    .constraints
                    .get(&after)
                    .is_some_and(|nodes| nodes.contains(&before))
                {
                    return None;
                }
            }
        }
        // Check that for each DFS-unvisited offset that lies between any of the visited
        // offsets and `node`, all the constraints are satisfied when unvisited offsets
        // are moved above all visited offsets.
        // These checks do not have to consider `Prepare` instructions.
        let visited = offsets.iter().collect::<BTreeSet<_>>();
        let min_visited = **visited
            .first()
            .expect("at least one offset must be visited");
        for before in min_visited..=node {
            if !visited.contains(&before) {
                // Is it safe to move `before` before everything else?
                for after in offsets.iter() {
                    if self
                        .constraints
                        .get(after)
                        .map_or(false, |nodes| nodes.contains(&before))
                    {
                        return None;
                    }
                }
            }
        }
        // Additional checks and restrictions corresponding to the placement of
        // `Prepare` instructions.
        let mut skip_prepares = BTreeSet::new();
        let mut copy_only_prepares = BTreeSet::new();
        for (i, offset) in offsets.iter().enumerate() {
            if let Some(prepared_tmp) = prepares.get(offset) {
                // For each `Prepare tmp` instruction, check if:
                // 1. Until its use, whether there are any reference-related instructions
                //    targeting `tmp`. In such a case, we have to skip this `Prepare`
                //    instruction, because we cannot eagerly access `tmp`.
                // 2. Until its use, whether there are any reads of `tmp`. In such a case,
                //    we have to notify that this `Prepare` instruction can only result in
                //    a copy of `tmp` (or a no-op, which is always safe for `Prepare`
                //    instructions). If it results in a move, then a subsequent read of
                //    `tmp` will be invalid.
                let use_offset = self
                    .constraints
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
                        // Skip the `Prepare` instructions corresponding to `offset`,
                        // because of 1.
                        copy_only_prepares.remove(offset);
                        skip_prepares.insert(*offset);
                        break;
                    }
                    if reads
                        .get(prepared_tmp)
                        .map_or(false, |nodes| nodes.contains(&scanned_offset))
                    {
                        // Insert a `copy_only` notice for the `Prepare` instruction,
                        // because of 2.
                        copy_only_prepares.insert(*offset);
                        break;
                    }
                }
            }
        }
        let new_offsets = offsets
            .into_iter()
            .filter(|o| !skip_prepares.contains(o))
            .collect::<Vec<_>>();
        Some((new_offsets, copy_only_prepares))
    }
}

/// Collection of instructions that have been reordered (along with the insertion of
/// `Prepare` instructions). Includes the corresponding `PrepareUseAnnotation`.
struct ReorderedInstructions {
    instructions: Vec<Bytecode>,
    prepare_use: PrepareUseAnnotation,
}

/// Edge-ordered data dependence graph for a basic block.
/// Maps a "use" instruction offset to the list of "def" instruction offsets that it
/// depends on (i.e., use-def edges). The edges are ordered based on the order of
/// sources in the use instruction.
/// The graph does not carry loop-carried dependencies, and is therefore a directed
/// acyclic graph.
/// If the definition of a source is not found in the block, then the definition is
/// represented as `None`.
#[derive(Debug)]
struct DataDependenceGraph(pub BTreeMap<CodeOffset, Vec<Option<CodeOffset>>>);

/// Collection of constraints that restrict the reordering of instructions within a block.
/// If there is an edge from instruction offsets `a` to `b`, then `a` must be appear
/// before `b` in a basic block.
#[derive(Default)]
struct DependenceConstraints {
    edges: BTreeMap<CodeOffset, BTreeSet<CodeOffset>>,
}

impl DependenceConstraints {
    /// Compute the all the constraints for block reordering.
    pub fn compute_from(block: &BlockReordering) -> BTreeMap<CodeOffset, BTreeSet<CodeOffset>> {
        let mut constraints = Self::default();
        constraints
            .add_false_dependencies(&block.instructions, block.original_block_len)
            .add_true_dependencies(&block.dd_graph)
            .add_ref_arg_dependencies(&block.instructions, block.original_block_len)
            .add_move_dependencies(&block.instructions, block.original_block_len)
            .add_relatively_non_reorderable_dependencies(
                &block.instructions,
                block.original_block_len,
            );
        debug_assert!(!constraints.has_cycle(block.instructions.len()));
        constraints.edges
    }

    /// Compute and add false dependencies for a `block` (of straight-line code).
    /// False dependencies include write-after-read and write-after-write dependencies.
    /// Note: we do not add false dependencies for `Prepare` instructions.
    fn add_false_dependencies(&mut self, block: &[Bytecode], upper: usize) -> &mut Self {
        // Track all the reads of a tmp before a write to it.
        let mut reads_before: BTreeMap<TempIndex, BTreeSet<CodeOffset>> = BTreeMap::new();
        // Track the most recent write to a tmp.
        let mut latest_write: BTreeMap<TempIndex, CodeOffset> = BTreeMap::new();
        for (offset, instr) in block.iter().take(upper).enumerate() {
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

    /// Add all true dependencies in a block based on the data dependence graph.
    /// A true dependency is a read-after-write dependency.
    fn add_true_dependencies(&mut self, dd_graph: &DataDependenceGraph) -> &mut Self {
        for (use_offset, def_offsets) in dd_graph.0.iter() {
            for def_offset in def_offsets.iter().filter_map(|d| *d) {
                self.edges
                    .entry(def_offset)
                    .or_default()
                    .insert(*use_offset);
            }
        }
        self
    }

    /// Add dependencies for reference-related instructions and the reads of the
    /// temps the reference-related instructions target.
    /// Note: we do not add ref arg dependencies for `Prepare` instructions.
    fn add_ref_arg_dependencies(&mut self, block: &[Bytecode], upper: usize) -> &mut Self {
        let mut reads: BTreeMap<TempIndex, BTreeSet<CodeOffset>> = BTreeMap::new();
        let mut ref_args: BTreeMap<TempIndex, CodeOffset> = BTreeMap::new();
        for (offset, instr) in block.iter().take(upper).enumerate() {
            let offset = offset as CodeOffset;
            if is_ref_related_instr(instr) {
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

    /// Add dependencies between the reads of temps and their moves.
    /// Note: we do not add move dependencies for `Prepare` instructions.
    fn add_move_dependencies(&mut self, block: &[Bytecode], upper: usize) -> &mut Self {
        let mut reads_before_move: BTreeMap<TempIndex, BTreeSet<CodeOffset>> = BTreeMap::new();
        use AssignKind::*;
        for (offset, instr) in block.iter().take(upper).enumerate() {
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

    /// Add dependencies between instructions that are relatively non-reorderable.
    fn add_relatively_non_reorderable_dependencies(
        &mut self,
        block: &[Bytecode],
        upper: usize,
    ) -> &mut Self {
        let mut prev_offset = None;
        for (offset, instr) in block.iter().take(upper).enumerate() {
            if Self::is_relatively_non_reorderable(instr) {
                let offset = offset as CodeOffset;
                if let Some(prev_offset) = prev_offset {
                    self.edges.entry(prev_offset).or_default().insert(offset);
                }
                prev_offset = Some(offset);
            }
        }
        self
    }

    /// Is the `instr` relatively non-reorderable?
    /// Two relatively non-reorderable instructions cannot change their relative order.
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

    /// Check if the constraints form a cycle.
    fn has_cycle(&self, num_nodes: usize) -> bool {
        let mut visited_ever = BTreeSet::new();
        let mut ancestors = BTreeSet::new();
        for node in 0..num_nodes {
            if self.dfs(node as CodeOffset, &mut visited_ever, &mut ancestors) {
                return true;
            }
        }
        false
    }

    /// Helper for cycle detection.
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

/// Helper function to check if the instruction is a reference-related instruction.
fn is_ref_related_instr(instr: &Bytecode) -> bool {
    use Operation::*;
    match instr {
        Bytecode::Call(_, _, op, _, _) => {
            matches!(op, FreezeRef(_) | WriteRef | BorrowLoc | BorrowField(..))
        },
        _ => false,
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
        let ReorderedInstructions {
            instructions,
            prepare_use,
        } = Self::compute_reordered_instructions(&target);
        // Clear all previous annotations, because reordering can change code
        // and invalidate previous annotations.
        data.annotations.clear();
        data.code = instructions;
        data.annotations.set(prepare_use, true);
        data
    }

    fn name(&self) -> String {
        "InstructionReorderingProcessor".to_string()
    }
}

impl InstructionReorderingProcessor {
    fn compute_reordered_instructions(target: &FunctionTarget) -> ReorderedInstructions {
        let code = target.get_bytecode();
        let cfg = StacklessControlFlowGraph::new_forward(code);
        let mut block_ranges = cfg
            .blocks()
            .iter()
            .filter_map(|block_id| cfg.instr_offset_bounds(*block_id))
            .collect::<Vec<_>>();
        // TODO: Explicit sorting can be skipped if `block_ranges` can be guaranteed to be already
        // sorted (i.e., guaranteed based on the methods used on the `StacklessControlFlowGraph`).
        block_ranges.sort_by_key(|k| k.0);
        let mut new_code = vec![];
        let mut function_level_prepare_use = PrepareUseAnnotation::default();
        for (lower, upper) in block_ranges {
            let block = code[usize::from(lower)..=usize::from(upper)].to_vec();
            let ReorderedInstructions {
                instructions,
                prepare_use,
            } = Self::optimize_for_stack_machine(block);
            let new_lower = new_code.len() as CodeOffset;
            new_code.extend(instructions);
            function_level_prepare_use.extend_with(prepare_use, new_lower);
        }
        ReorderedInstructions {
            instructions: new_code,
            prepare_use: function_level_prepare_use,
        }
    }

    fn optimize_for_stack_machine(block: Vec<Bytecode>) -> ReorderedInstructions {
        // If there are any specification-only instructions or inline spec blocks,
        // we do not perform any reordering optimizations, as dependencies in spec blocks
        // are not captured. We may be able to relax this limitation in the future.
        if block.iter().any(|instr| {
            instr.is_spec_only()
                || matches!(instr, Bytecode::SpecBlock(..))
                || matches!(instr, Bytecode::Call(_, _, _, _, Some(_)))
        }) {
            return {
                ReorderedInstructions {
                    instructions: block,
                    prepare_use: PrepareUseAnnotation::default(),
                }
            };
        }
        BlockReordering::prepare_and_reorder(block)
    }
}
