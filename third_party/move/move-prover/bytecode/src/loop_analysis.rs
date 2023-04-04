// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    function_data_builder::{FunctionDataBuilder, FunctionDataBuilderOptions},
    function_target::{FunctionData, FunctionTarget},
    function_target_pipeline::{FunctionTargetProcessor, FunctionTargetsHolder},
    graph::{Graph, NaturalLoop},
    options::ProverOptions,
    stackless_bytecode::{AttrId, Bytecode, HavocKind, Label, Operation, PropKind},
    stackless_control_flow_graph::{BlockContent, BlockId, StacklessControlFlowGraph},
};
use move_binary_format::file_format::CodeOffset;
use move_model::{
    ast::{self, TempIndex},
    exp_generator::ExpGenerator,
    model::FunctionEnv,
    pragmas::UNROLL_PRAGMA,
    ty::{PrimitiveType, Type},
};
use std::collections::{BTreeMap, BTreeSet};

const LOOP_INVARIANT_BASE_FAILED: &str = "base case of the loop invariant does not hold";
const LOOP_INVARIANT_INDUCTION_FAILED: &str = "induction case of the loop invariant does not hold";

/// A fat-loop captures the information of one or more natural loops that share the same loop
/// header. This shared header is called the header of the fat-loop.
///
/// Conceptually, every back edge defines a unique natural loop and different back edges may points
/// to the same loop header (e.g., when there are two "continue" statements in the loop body).
///
/// However, since these natural loops share the same loop header, they share the same loop
/// invariants too and the fat-loop targets (i.e., variables that may be changed in any sub-loop)
/// is the union of loop targets per each natural loop that share the header.
#[derive(Debug, Clone)]
pub struct FatLoop {
    pub invariants: BTreeMap<CodeOffset, (AttrId, ast::Exp)>,
    pub val_targets: BTreeSet<TempIndex>,
    pub mut_targets: BTreeMap<TempIndex, bool>,
    pub back_edges: BTreeSet<CodeOffset>,
}

/// A summary of loops *with invariants specified by developers*.
#[derive(Debug, Clone)]
pub struct LoopAnnotation {
    pub fat_loops: BTreeMap<Label, FatLoop>,
}

impl LoopAnnotation {
    fn back_edges_locations(&self) -> BTreeSet<CodeOffset> {
        self.fat_loops
            .values()
            .flat_map(|l| l.back_edges.iter())
            .copied()
            .collect()
    }

    fn invariants_locations(&self) -> BTreeSet<CodeOffset> {
        self.fat_loops
            .values()
            .flat_map(|l| l.invariants.keys())
            .copied()
            .collect()
    }
}

#[derive(Debug, Clone)]
pub struct LoopUnrollingMark {
    pub marker: Option<AttrId>,
    pub loop_body: Vec<Vec<Bytecode>>,
    pub back_edges: BTreeSet<CodeOffset>,
    pub iter_count: usize,
}

/// A summary of loops *without any invariant specified*.
#[derive(Debug, Clone)]
pub struct LoopUnrolling {
    pub fat_loops: BTreeMap<Label, LoopUnrollingMark>,
}

pub struct LoopAnalysisProcessor {}

impl LoopAnalysisProcessor {
    pub fn new() -> Box<Self> {
        Box::new(LoopAnalysisProcessor {})
    }
}

impl FunctionTargetProcessor for LoopAnalysisProcessor {
    fn process(
        &self,
        _targets: &mut FunctionTargetsHolder,
        func_env: &FunctionEnv,
        data: FunctionData,
        _scc_opt: Option<&[FunctionEnv]>,
    ) -> FunctionData {
        if func_env.is_native() {
            return data;
        }
        let (loops_with_invariants, loops_for_unrolling) = Self::build_loop_info(func_env, &data);
        let mut data = Self::transform(func_env, data, &loops_with_invariants);
        for (header_label, unrolling_instruction) in loops_for_unrolling.fat_loops {
            data = Self::unroll(func_env, data, &header_label, &unrolling_instruction);
        }
        // we have unrolled the loop into a DAG, and there will be no loop unrolling marks left
        data.loop_unrolling.clear();
        data
    }

    fn name(&self) -> String {
        "loop_analysis".to_string()
    }
}

impl LoopAnalysisProcessor {
    /// Perform a loop transformation that eliminate back-edges in a loop and flatten the function
    /// CFG into a directed acyclic graph (DAG).
    ///
    /// The general procedure works as following (assuming the loop invariant expression is L):
    ///
    /// - At the beginning of the loop header (identified by the label bytecode), insert the
    ///   following statements:
    ///     - assert L;
    ///     - havoc T;
    ///     - assume L;
    /// - Create a new dummy block (say, block X) with only the following statements
    ///     - assert L;
    ///     - stop;
    /// - For each backedge in this loop:
    ///     - In the source block of the back edge, replace the last statement (must be a jump or
    ///       branch) with the new label of X.
    fn transform(
        func_env: &FunctionEnv<'_>,
        data: FunctionData,
        loop_annotation: &LoopAnnotation,
    ) -> FunctionData {
        let options = ProverOptions::get(func_env.module_env.env);

        let back_edge_locs = loop_annotation.back_edges_locations();
        let invariant_locs = loop_annotation.invariants_locations();
        let mut builder =
            FunctionDataBuilder::new_with_options(func_env, data, FunctionDataBuilderOptions {
                no_fallthrough_jump_removal: true,
            });
        let mut goto_fixes = vec![];
        let code = std::mem::take(&mut builder.data.code);
        for (offset, bytecode) in code.into_iter().enumerate() {
            match bytecode {
                Bytecode::Label(attr_id, label) => {
                    builder.emit(bytecode);
                    builder.set_loc_from_attr(attr_id);
                    if let Some(loop_info) = loop_annotation.fat_loops.get(&label) {
                        // assert loop invariants -> this is the base case
                        for (attr_id, exp) in loop_info.invariants.values() {
                            builder.set_loc_and_vc_info(
                                builder.get_loc(*attr_id),
                                LOOP_INVARIANT_BASE_FAILED,
                            );
                            builder.emit_with(|attr_id| {
                                Bytecode::Prop(attr_id, PropKind::Assert, exp.clone())
                            });
                        }

                        // havoc all loop targets
                        for idx in &loop_info.val_targets {
                            builder.emit_with(|attr_id| {
                                Bytecode::Call(
                                    attr_id,
                                    vec![*idx],
                                    Operation::Havoc(HavocKind::Value),
                                    vec![],
                                    None,
                                )
                            });
                            // add a well-formed assumption explicitly and immediately
                            let exp = builder.mk_call(
                                &Type::Primitive(PrimitiveType::Bool),
                                ast::Operation::WellFormed,
                                vec![builder.mk_temporary(*idx)],
                            );
                            builder.emit_with(move |id| Bytecode::Prop(id, PropKind::Assume, exp));
                        }
                        for (idx, havoc_all) in &loop_info.mut_targets {
                            let havoc_kind = if *havoc_all {
                                HavocKind::MutationAll
                            } else {
                                HavocKind::MutationValue
                            };
                            builder.emit_with(|attr_id| {
                                Bytecode::Call(
                                    attr_id,
                                    vec![*idx],
                                    Operation::Havoc(havoc_kind),
                                    vec![],
                                    None,
                                )
                            });
                            // add a well-formed assumption explicitly and immediately
                            let exp = builder.mk_call(
                                &Type::Primitive(PrimitiveType::Bool),
                                ast::Operation::WellFormed,
                                vec![builder.mk_temporary(*idx)],
                            );
                            builder.emit_with(move |id| Bytecode::Prop(id, PropKind::Assume, exp));
                        }

                        // trace implicitly reassigned variables after havocking
                        let affected_variables: BTreeSet<_> = loop_info
                            .val_targets
                            .iter()
                            .chain(loop_info.mut_targets.keys())
                            .collect();

                        // Only emit this for user declared locals, not for ones introduced
                        // by stack elimination.
                        let affected_non_temporary_variables: BTreeSet<_> = affected_variables
                            .into_iter()
                            .filter(|&idx| !func_env.is_temporary(*idx))
                            .collect();

                        if affected_non_temporary_variables.is_empty() {
                            // no user declared local is havocked
                            builder.set_next_debug_comment(format!(
                                "info: enter loop {}",
                                match loop_info.invariants.is_empty() {
                                    true => "",
                                    false => ", loop invariant holds at current state",
                                }
                            ));
                        } else {
                            // show the havocked locals to user
                            let affected_non_temporary_variable_names: Vec<_> =
                                affected_non_temporary_variables
                                    .iter()
                                    .map(|&idx| {
                                        func_env
                                            .symbol_pool()
                                            .string(func_env.get_local_name(*idx))
                                            .to_string()
                                    })
                                    .collect();
                            let joined_variables_names_str =
                                affected_non_temporary_variable_names.join(", ");
                            builder.set_next_debug_comment(format!(
                                "info: enter loop, variable(s) {} havocked and reassigned",
                                joined_variables_names_str
                            ));
                        }

                        // track the new values of havocked user declared locals
                        for idx_ in &affected_non_temporary_variables {
                            let idx = *idx_;
                            builder.emit_with(|id| {
                                Bytecode::Call(
                                    id,
                                    vec![],
                                    Operation::TraceLocal(*idx),
                                    vec![*idx],
                                    None,
                                )
                            });
                        }

                        // after showing the havocked locals and their new values, show the following message
                        if !affected_non_temporary_variables.is_empty()
                            && !loop_info.invariants.is_empty()
                        {
                            builder.set_next_debug_comment(
                                "info: loop invariant holds at current state".to_string(),
                            );
                        }

                        // add an additional assumption that the loop did not abort
                        let exp =
                            builder.mk_not(builder.mk_bool_call(ast::Operation::AbortFlag, vec![]));
                        builder.emit_with(|attr_id| Bytecode::Prop(attr_id, PropKind::Assume, exp));

                        // re-assume loop invariants
                        for (attr_id, exp) in loop_info.invariants.values() {
                            builder.emit(Bytecode::Prop(*attr_id, PropKind::Assume, exp.clone()));
                        }
                    }
                },
                Bytecode::Prop(_, PropKind::Assert, _)
                    if invariant_locs.contains(&(offset as CodeOffset)) =>
                {
                    // skip it, as the invariant should have been added as an assert after the label
                },
                _ => {
                    builder.emit(bytecode);
                },
            }
            // mark that the goto labels in this bytecode needs to be updated to a new label
            // representing the invariant-checking block for the loop.
            if back_edge_locs.contains(&(offset as CodeOffset)) {
                goto_fixes.push(builder.data.code.len() - 1);
            }
        }

        // create one invariant-checking block for each fat loop
        let invariant_checker_labels: BTreeMap<_, _> = loop_annotation
            .fat_loops
            .keys()
            .map(|label| (*label, builder.new_label()))
            .collect();

        for (label, loop_info) in &loop_annotation.fat_loops {
            let checker_label = invariant_checker_labels.get(label).unwrap();
            builder.set_next_debug_comment(format!(
                "Loop invariant checking block for the loop started with header: L{}",
                label.as_usize()
            ));
            builder.emit_with(|attr_id| Bytecode::Label(attr_id, *checker_label));
            builder.clear_next_debug_comment();

            // add instrumentations to assert loop invariants -> this is the induction case
            for (attr_id, exp) in loop_info.invariants.values() {
                builder.set_loc_and_vc_info(
                    builder.get_loc(*attr_id),
                    LOOP_INVARIANT_INDUCTION_FAILED,
                );
                builder.emit_with(|attr_id| Bytecode::Prop(attr_id, PropKind::Assert, exp.clone()));
            }

            // stop the checking in proving mode (branch back to loop header for interpretation mode)
            builder.emit_with(|attr_id| {
                if options.for_interpretation {
                    Bytecode::Jump(attr_id, *label)
                } else {
                    Bytecode::Call(attr_id, vec![], Operation::Stop, vec![], None)
                }
            });
        }

        // fix the goto statements in the loop latch blocks
        for code_offset in goto_fixes {
            let updated_goto = match &builder.data.code[code_offset] {
                Bytecode::Jump(attr_id, old_label) => {
                    Bytecode::Jump(*attr_id, *invariant_checker_labels.get(old_label).unwrap())
                },
                Bytecode::Branch(attr_id, if_label, else_label, idx) => {
                    let new_if_label = *invariant_checker_labels.get(if_label).unwrap_or(if_label);
                    let new_else_label = *invariant_checker_labels
                        .get(else_label)
                        .unwrap_or(else_label);
                    Bytecode::Branch(*attr_id, new_if_label, new_else_label, *idx)
                },
                _ => panic!("Expect a branch statement"),
            };
            builder.data.code[code_offset] = updated_goto;
        }

        // we have unrolled the loop into a DAG, and there will be no loop invariants left
        builder.data.loop_invariants.clear();
        builder.data
    }

    /// Perform unrolling on the loop (if explicitly requested).
    ///
    /// NOTE: this turns verification into *bounded* verification. All verification conditions post
    /// loop exit is only conditionally verified, conditioned when loop exits within a pre-defined
    /// number of iteration. If the loop iterates more than the pre-defined limit, the prover will
    /// not attempt to prove (or disprove) those verification conditions.
    fn unroll(
        func_env: &FunctionEnv<'_>,
        data: FunctionData,
        loop_header: &Label,
        unrolling_mark: &LoopUnrollingMark,
    ) -> FunctionData {
        let options = ProverOptions::get(func_env.module_env.env);
        let mut builder =
            FunctionDataBuilder::new_with_options(func_env, data, FunctionDataBuilderOptions {
                no_fallthrough_jump_removal: true,
            });

        // collect labels that belongs to this loop
        let in_loop_labels: BTreeSet<_> = unrolling_mark
            .loop_body
            .iter()
            .flatten()
            .filter_map(|bc| match bc {
                Bytecode::Label(_, label) => Some(*label),
                _ => None,
            })
            .collect();
        assert!(in_loop_labels.contains(loop_header));

        // create the stop block
        let stop_label = builder.new_label();
        builder.set_next_debug_comment(format!(
            "End of bounded loop unrolling for loop: L{}",
            loop_header.as_usize()
        ));
        builder.emit_with(|attr_id| Bytecode::Label(attr_id, stop_label));
        builder.clear_next_debug_comment();

        builder.emit_with(|attr_id| {
            if options.for_interpretation {
                Bytecode::Jump(attr_id, *loop_header)
            } else {
                Bytecode::Call(attr_id, vec![], Operation::Stop, vec![], None)
            }
        });

        // pre-populate the labels in unrolled iterations
        let mut label_remapping = BTreeMap::new();
        for i in 0..unrolling_mark.iter_count {
            for label in &in_loop_labels {
                label_remapping.insert((*label, i), builder.new_label());
            }
        }
        // the last back edge points to the stop block
        label_remapping.insert((*loop_header, unrolling_mark.iter_count), stop_label);

        // pre-populate the bytecode in unrolled iterations
        for i in 0..unrolling_mark.iter_count {
            for bc in unrolling_mark.loop_body.iter().flatten() {
                let mut new_bc = bc.clone();
                let new_attr_id = builder.new_attr_with_cloned_info(bc.get_attr_id());
                new_bc.set_attr_id(new_attr_id);
                // fix the labels
                match &mut new_bc {
                    Bytecode::Label(_, label) => {
                        *label = *label_remapping.get(&(*label, i)).unwrap();
                    },
                    Bytecode::Jump(_, label) => {
                        if in_loop_labels.contains(label) {
                            if label == loop_header {
                                *label = *label_remapping.get(&(*label, i + 1)).unwrap();
                            } else {
                                *label = *label_remapping.get(&(*label, i)).unwrap();
                            }
                        }
                    },
                    Bytecode::Branch(_, then_label, else_label, _) => {
                        if in_loop_labels.contains(then_label) {
                            if then_label == loop_header {
                                *then_label = *label_remapping.get(&(*then_label, i + 1)).unwrap();
                            } else {
                                *then_label = *label_remapping.get(&(*then_label, i)).unwrap();
                            }
                        }
                        if in_loop_labels.contains(else_label) {
                            if then_label == loop_header {
                                *else_label = *label_remapping.get(&(*else_label, i + 1)).unwrap();
                            } else {
                                *else_label = *label_remapping.get(&(*else_label, i)).unwrap();
                            }
                        }
                    },
                    _ => (),
                }
                builder.emit(new_bc);
            }
        }

        // bridge the back edges into the newly populated code
        let code = std::mem::take(&mut builder.data.code);
        for (offset, mut bytecode) in code.into_iter().enumerate() {
            if unrolling_mark.marker == Some(bytecode.get_attr_id()) {
                continue;
            }
            if unrolling_mark.back_edges.contains(&(offset as CodeOffset)) {
                match &mut bytecode {
                    Bytecode::Jump(_, label) => {
                        assert_eq!(label, loop_header);
                        *label = *label_remapping.get(&(*label, 0)).unwrap();
                    },
                    Bytecode::Branch(_, then_label, else_label, _) => {
                        if then_label == loop_header {
                            *then_label = *label_remapping.get(&(*then_label, 0)).unwrap();
                        } else {
                            assert_eq!(else_label, loop_header);
                            *else_label = *label_remapping.get(&(*else_label, 0)).unwrap();
                        }
                    },
                    _ => (),
                }
            }
            builder.emit(bytecode);
        }

        builder.data
    }

    /// Collect invariants in the given loop header block
    ///
    /// Loop invariants are defined as
    /// 1) the longest sequence of consecutive
    /// 2) `PropKind::Assert` propositions
    /// 3) in the loop header block, immediately after the `Label` statement,
    /// 4) which are also marked in the `loop_invariants` field in the `FunctionData`.
    /// All above conditions must be met to be qualified as a loop invariant.
    ///
    /// The reason we piggyback on `PropKind::Assert` instead of introducing a new
    /// `PropKind::Invariant` is that we don't want to introduce a`PropKind::Invariant` type which
    /// only exists to be eliminated. The same logic applies for other invariants in the system
    /// (e.g., data invariants, global invariants, etc).
    ///
    /// In other words, for the loop header block:
    /// - the first statement must be a `label`,
    /// - followed by N `assert` statements, N >= 0
    /// - all these N `assert` statements are marked as loop invariants,
    /// - statement N + 1 is either not an `assert` or is not marked in `loop_invariants`.
    fn collect_loop_invariants(
        cfg: &StacklessControlFlowGraph,
        func_target: &FunctionTarget<'_>,
        loop_header: BlockId,
    ) -> BTreeMap<CodeOffset, (AttrId, ast::Exp)> {
        let code = func_target.get_bytecode();
        let asserts_as_invariants = &func_target.data.loop_invariants;

        let mut invariants = BTreeMap::new();
        for (index, code_offset) in cfg.instr_indexes(loop_header).unwrap().enumerate() {
            let bytecode = &code[code_offset as usize];
            if index == 0 {
                assert!(matches!(bytecode, Bytecode::Label(_, _)));
            } else {
                match bytecode {
                    Bytecode::Prop(attr_id, PropKind::Assert, exp)
                        if asserts_as_invariants.contains(attr_id) =>
                    {
                        invariants.insert(code_offset, (*attr_id, exp.clone()));
                    },
                    _ => break,
                }
            }
        }
        invariants
    }

    /// Collect loop unrolling instruction in the given loop header block
    ///
    /// A loop unrolling instruction defined as
    /// - an `assume true;`
    /// - in the loop header block, immediately after the `Label` statement,
    /// - with its `attr_id` marked in the `loop_unrolling` field in the `FunctionData`
    fn probe_loop_unrolling_mark(
        cfg: &StacklessControlFlowGraph,
        func_target: &FunctionTarget<'_>,
        loop_header: BlockId,
    ) -> Option<(AttrId, usize)> {
        let code = func_target.get_bytecode();
        let assumes_as_unrolling_marks = &func_target.data.loop_unrolling;

        let mut marks = BTreeMap::new();
        for (index, code_offset) in cfg.instr_indexes(loop_header).unwrap().enumerate() {
            let bytecode = &code[code_offset as usize];
            if index == 0 {
                assert!(matches!(bytecode, Bytecode::Label(_, _)));
            } else {
                match bytecode {
                    Bytecode::Prop(attr_id, PropKind::Assume, _) => {
                        match assumes_as_unrolling_marks.get(attr_id) {
                            None => {
                                break;
                            },
                            Some(count) => {
                                marks.insert(code_offset, (*attr_id, *count));
                            },
                        }
                    },

                    _ => break,
                }
            }
        }

        // check that there is at most one unrolling mark
        let env = func_target.global_env();
        if marks.len() > 1 {
            for (attr_id, _) in marks.values() {
                env.error(
                    &func_target.get_bytecode_loc(*attr_id),
                    "Loop unrolling mark can only be specified once per loop",
                );
            }
        }
        marks
            .into_iter()
            .next()
            .map(|(_, (attr_id, count))| (attr_id, count))
    }

    /// Collect variables that may be changed during the loop execution.
    ///
    /// The input to this function should include all the sub loops that constitute a fat-loop.
    /// This function will return two sets of variables that represents, respectively,
    /// - the set of values to be havoc-ed, and
    /// - the set of mutations to he havoc-ed and how they should be havoc-ed.
    fn collect_loop_targets(
        cfg: &StacklessControlFlowGraph,
        func_target: &FunctionTarget<'_>,
        sub_loops: &[NaturalLoop<BlockId>],
    ) -> (BTreeSet<TempIndex>, BTreeMap<TempIndex, bool>) {
        let code = func_target.get_bytecode();
        let mut val_targets = BTreeSet::new();
        let mut mut_targets = BTreeMap::new();
        let fat_loop_body: BTreeSet<_> = sub_loops
            .iter()
            .flat_map(|l| l.loop_body.iter())
            .copied()
            .collect();
        for block_id in fat_loop_body {
            for code_offset in cfg
                .instr_indexes(block_id)
                .expect("A loop body should never contain a dummy block")
            {
                let bytecode = &code[code_offset as usize];
                let (bc_val_targets, bc_mut_targets) = bytecode.modifies(func_target);
                val_targets.extend(bc_val_targets);
                for (idx, is_full_havoc) in bc_mut_targets {
                    mut_targets
                        .entry(idx)
                        .and_modify(|v| {
                            *v = *v || is_full_havoc;
                        })
                        .or_insert(is_full_havoc);
                }
            }
        }
        (val_targets, mut_targets)
    }

    /// Collect code offsets that are branch instructions forming loop back-edges
    ///
    /// The input to this function should include all the sub loops that constitute a fat-loop.
    /// This function will return one back-edge location for each sub loop.
    fn collect_loop_back_edges(
        code: &[Bytecode],
        cfg: &StacklessControlFlowGraph,
        header_label: Label,
        sub_loops: &[NaturalLoop<BlockId>],
    ) -> BTreeSet<CodeOffset> {
        sub_loops
            .iter()
            .map(|l| {
                let code_offset = match cfg.content(l.loop_latch) {
                    BlockContent::Dummy => {
                        panic!("A loop body should never contain a dummy block")
                    },
                    BlockContent::Basic { upper, .. } => *upper,
                };
                match &code[code_offset as usize] {
                    Bytecode::Jump(_, goto_label) if *goto_label == header_label => {},
                    Bytecode::Branch(_, if_label, else_label, _)
                        if *if_label == header_label || *else_label == header_label => {},
                    _ => panic!("The latch bytecode of a loop does not branch into the header"),
                };
                code_offset
            })
            .collect()
    }

    /// Collect bytecodes that constitute the loop
    ///
    /// The input to this function should include all the sub loops that constitute a fat-loop.
    /// This function will return a vector of basic blocks, where each basic block is a vector
    /// of bytecode.
    fn collect_loop_body_bytecode(
        code: &[Bytecode],
        cfg: &StacklessControlFlowGraph,
        sub_loops: &[NaturalLoop<BlockId>],
    ) -> Vec<Vec<Bytecode>> {
        sub_loops
            .iter()
            .flat_map(|l| l.loop_body.iter())
            .map(|block_id| match cfg.content(*block_id) {
                BlockContent::Dummy => {
                    panic!("A loop body should never contain a dummy block")
                },
                BlockContent::Basic { lower, upper } => {
                    let block: Vec<_> = (*lower..=*upper)
                        .map(|i| code.get(i as usize).unwrap().clone())
                        .collect();
                    block
                },
            })
            .collect()
    }

    /// Find all loops in the function and collect information needed for invariant instrumentation
    /// (i.e., loop-to-DAG transformation) and loop unrolling (if requested by user).
    fn build_loop_info(
        func_env: &FunctionEnv<'_>,
        data: &FunctionData,
    ) -> (LoopAnnotation, LoopUnrolling) {
        // build for natural loops
        let func_target = FunctionTarget::new(func_env, data);
        let env = func_target.global_env();
        let code = func_target.get_bytecode();
        let cfg = StacklessControlFlowGraph::new_forward(code);
        let entry = cfg.entry_block();
        let nodes = cfg.blocks();
        let edges: Vec<(BlockId, BlockId)> = nodes
            .iter()
            .flat_map(|x| {
                cfg.successors(*x)
                    .iter()
                    .map(|y| (*x, *y))
                    .collect::<Vec<(BlockId, BlockId)>>()
            })
            .collect();
        let graph = Graph::new(entry, nodes, edges);
        let natural_loops = graph.compute_reducible().expect(
            "A well-formed Move function is expected to have a reducible control-flow graph",
        );
        let unroll_pragma = func_env.get_num_pragma(UNROLL_PRAGMA);

        // collect shared headers from loops
        let mut fat_headers = BTreeMap::new();
        for single_loop in natural_loops {
            fat_headers
                .entry(single_loop.loop_header)
                .or_insert_with(Vec::new)
                .push(single_loop);
        }

        // build fat loops by label
        let mut fat_loops_for_unrolling = BTreeMap::new();
        let mut fat_loops_with_invariants = BTreeMap::new();
        for (fat_root, sub_loops) in fat_headers {
            // get the label of the scc root
            let label = match cfg.content(fat_root) {
                BlockContent::Dummy => panic!("A loop header should never be a dummy block"),
                BlockContent::Basic { lower, upper: _ } => match code[*lower as usize] {
                    Bytecode::Label(_, label) => label,
                    _ => panic!("A loop header block is expected to start with a Label bytecode"),
                },
            };

            let invariants = Self::collect_loop_invariants(&cfg, &func_target, fat_root);
            let unrolling_mark = Self::probe_loop_unrolling_mark(&cfg, &func_target, fat_root)
                .map(|(marker, count)| (Some(marker), count))
                .or_else(|| unroll_pragma.map(|count| (None, count)));
            let back_edges = Self::collect_loop_back_edges(code, &cfg, label, &sub_loops);

            // loop invariants and unrolling should be mutual exclusive
            match unrolling_mark {
                None => {
                    // loop invariant instrumentation route
                    let (val_targets, mut_targets) =
                        Self::collect_loop_targets(&cfg, &func_target, &sub_loops);
                    fat_loops_with_invariants.insert(label, FatLoop {
                        invariants,
                        val_targets,
                        mut_targets,
                        back_edges,
                    });
                },
                Some((attr_id, count)) => {
                    if !invariants.is_empty() {
                        let error_loc = attr_id.map_or_else(
                            || env.unknown_loc(),
                            |attr_id| func_target.get_bytecode_loc(attr_id),
                        );
                        env.error(
                            &error_loc,
                            "loop invariants and loop unrolling is mutual exclusive",
                        );
                    }
                    // loop unrolling route
                    let loop_body = Self::collect_loop_body_bytecode(code, &cfg, &sub_loops);
                    fat_loops_for_unrolling.insert(label, LoopUnrollingMark {
                        marker: attr_id,
                        loop_body,
                        back_edges,
                        iter_count: count,
                    });
                },
            }
        }

        // check for redundant loop invariant declarations in the spec
        let all_invariants: BTreeSet<_> = fat_loops_with_invariants
            .values()
            .flat_map(|l| l.invariants.values().map(|(attr_id, _)| *attr_id))
            .collect();
        for attr_id in data.loop_invariants.difference(&all_invariants) {
            env.error(
                &func_target.get_bytecode_loc(*attr_id),
                "Loop invariants must be declared at the beginning of the loop header in a \
                consecutive sequence",
            );
        }

        // check for redundant loop unrolling marks in the spe
        let all_unrolling_marks: BTreeSet<_> = fat_loops_for_unrolling
            .values()
            .filter_map(|l| l.marker)
            .collect();
        let declared_unrolling_marks: BTreeSet<_> = data.loop_unrolling.keys().copied().collect();
        for attr_id in declared_unrolling_marks.difference(&all_unrolling_marks) {
            env.error(
                &func_target.get_bytecode_loc(*attr_id),
                "Loop unrolling mark must be declared at the beginning of the loop header",
            );
        }

        // done with information collection
        (
            LoopAnnotation {
                fat_loops: fat_loops_with_invariants,
            },
            LoopUnrolling {
                fat_loops: fat_loops_for_unrolling,
            },
        )
    }
}
