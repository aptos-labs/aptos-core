// Parts of the file are Copyright (c) The Diem Core Contributors
// Parts of the file are Copyright (c) The Move Contributors
// Parts of the file are Copyright (c) Aptos Foundation
// All Aptos Foundation code and content is licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    memory_instrumentation::Instrumenter, options::ProverOptions,
    spec_inference::infer_loop_head_evidence,
};
use move_binary_format::file_format::CodeOffset;
use move_model::{
    ast::{self, QuantKind, TempIndex},
    exp_generator::ExpGenerator,
    model::{FunctionEnv, Loc},
    ty::{PrimitiveType, Type},
};
use move_stackless_bytecode::{
    fat_loop::{self, FatLoopFunctionInfo, LoopUnrollingMark},
    function_data_builder::{FunctionDataBuilder, FunctionDataBuilderOptions},
    function_target::{FunctionData, FunctionTarget},
    function_target_pipeline::{FunctionTargetProcessor, FunctionTargetsHolder},
    stackless_bytecode::{Bytecode, HavocKind, Label, Operation, PropKind},
};
use std::collections::{BTreeMap, BTreeSet};

const LOOP_INVARIANT_BASE_FAILED: &str = "base case of the loop invariant does not hold";
const LOOP_INVARIANT_INDUCTION_FAILED: &str = "induction case of the loop invariant does not hold";

/// Source locations of loops that were transformed without any invariant.
///
/// `LoopAnalysisProcessor` turns every loop into a DAG by havocking its modified
/// targets.  This is enough for ordinary verification, but WP inference has to
/// quantify those targets when it summarizes the function.  Keeping the source
/// locations lets the inference pass report a resulting vacuous or solver-hard
/// contract at the loop that needs a repair, rather than only at the enclosing
/// function.
#[derive(Clone, Debug, Default)]
pub struct LoopsWithoutInvariants(pub Vec<LoopWithoutInvariant>);

#[derive(Clone, Debug)]
pub struct LoopWithoutInvariant {
    /// Stable within this function and this pipeline run.
    pub loop_id: usize,
    /// Header in the normalized, pre-transformation bytecode.
    pub header: Label,
    pub loc: Loc,
    /// True when the loop's source location carries an inline-expansion chain.
    /// Such a loop may be the body of an inline higher-order iterator, in which
    /// case a `folds_of` invariant can be preferable to rewriting the call.
    pub is_inlined: bool,
    /// Source-visible value/reference locals modified by the loop.
    pub carried: Vec<(TempIndex, String)>,
    /// Number of modified user values or memories omitted from `carried`
    /// because they have no usable source-level name. Compiler expression
    /// temporaries are outside the source-level state and are not counted.
    pub omitted_carried: usize,
}

/// Diagnostic-only bounded WP observations. This annotation never contributes
/// conditions to the inferred specification.
#[derive(Clone, Debug, Default)]
pub struct LoopInvariantEvidence(pub Vec<LoopInvariantEvidenceForLoop>);

#[derive(Clone, Debug)]
pub struct LoopInvariantEvidenceForLoop {
    pub loop_id: usize,
    pub depth: usize,
    pub carried_names: Vec<String>,
    pub heads: Vec<LoopHeadObservation>,
    pub partial_notes: Vec<String>,
    pub unavailable: Option<String>,
}

#[derive(Clone, Debug)]
pub struct LoopHeadObservation {
    pub index: usize,
    pub facts: Vec<String>,
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
        targets: &mut FunctionTargetsHolder,
        func_env: &FunctionEnv,
        data: FunctionData,
        _scc_opt: Option<&[FunctionEnv]>,
    ) -> FunctionData {
        if !func_env.is_compiled() {
            return data;
        }
        match fat_loop::build_loop_info_for_spec(&FunctionTarget::new(func_env, &data), targets) {
            Ok((loops_with_invariants, loops_for_unrolling)) => {
                let loops_without_invariants =
                    Self::loops_without_invariants(func_env, &data, &loops_with_invariants);
                let loop_invariant_evidence = func_env
                    .module_env
                    .is_target()
                    .then(|| {
                        ProverOptions::get(func_env.module_env.env).loop_invariant_evidence_depth
                    })
                    .flatten()
                    .map(|depth| {
                        Self::collect_loop_invariant_evidence(
                            targets,
                            func_env,
                            &data,
                            &loops_without_invariants,
                            depth,
                        )
                    });
                let mut data = Self::transform(func_env, data, &loops_with_invariants);
                for (header_label, unrolling_instruction) in loops_for_unrolling.fat_loops {
                    data = Self::unroll(func_env, data, &header_label, &unrolling_instruction).0;
                }
                // we have unrolled the loop into a DAG, and there will be no loop unrolling marks left
                data.loop_unrolling.clear();
                if !loops_without_invariants.0.is_empty() {
                    data.annotations.set(loops_without_invariants, true);
                }
                if let Some(evidence) = loop_invariant_evidence {
                    data.annotations.set(evidence, true);
                }
                data
            },
            Err(err) => {
                func_env.module_env.env.error(
                    &func_env.get_loc(),
                    &format!("loop analysis failed: {}", err),
                );
                data
            },
        }
    }

    fn name(&self) -> String {
        "loop_analysis".to_string()
    }
}

impl LoopAnalysisProcessor {
    fn loops_without_invariants(
        func_env: &FunctionEnv<'_>,
        data: &FunctionData,
        loop_annotation: &FatLoopFunctionInfo,
    ) -> LoopsWithoutInvariants {
        let target = FunctionTarget::new(func_env, data);
        let loops = loop_annotation
            .fat_loops
            .iter()
            .filter(|(_, loop_info)| loop_info.spec_info().invariants.is_empty())
            .filter_map(|(header_label, loop_info)| {
                let attr_id = data.code.iter().find_map(|bytecode| match bytecode {
                    Bytecode::Label(attr_id, label) if label == header_label => Some(*attr_id),
                    _ => None,
                })?;
                let loc = data
                    .locations
                    .get(&attr_id)
                    .cloned()
                    .unwrap_or_else(|| func_env.get_loc());
                let spec_info = loop_info.spec_info();
                let carried_indices = spec_info
                    .val_targets
                    .iter()
                    .copied()
                    .chain(spec_info.mut_targets.keys().copied())
                    .collect::<BTreeSet<_>>();
                let omitted_carried = spec_info.mem_targets.len();
                let carried = carried_indices
                    .iter()
                    .filter_map(|index| {
                        if target.is_temporary(*index) {
                            None
                        } else {
                            let name = target
                                .get_local_name(*index)
                                .display(func_env.symbol_pool())
                                .to_string();
                            if name.starts_with('$') {
                                None
                            } else {
                                Some((*index, name))
                            }
                        }
                    })
                    .collect::<Vec<_>>();
                Some(LoopWithoutInvariant {
                    loop_id: 0,
                    header: *header_label,
                    is_inlined: loc.is_inlined(),
                    loc,
                    carried,
                    omitted_carried,
                })
            })
            .enumerate()
            .map(|(loop_id, mut loop_info)| {
                loop_info.loop_id = loop_id;
                loop_info
            })
            .collect();
        LoopsWithoutInvariants(loops)
    }

    fn collect_loop_invariant_evidence(
        targets: &FunctionTargetsHolder,
        func_env: &FunctionEnv<'_>,
        data: &FunctionData,
        missing: &LoopsWithoutInvariants,
        depth: usize,
    ) -> LoopInvariantEvidence {
        LoopInvariantEvidence(
            missing
                .0
                .iter()
                .map(|loop_info| Self::evidence_for_loop(targets, func_env, data, loop_info, depth))
                .collect(),
        )
    }

    /// Bounded facts for one loop.
    ///
    /// Every invariant-less loop is unrolled, so a loop reached only through an
    /// earlier one sees the facts of a bounded prefix rather than of all paths;
    /// that restriction is recorded as a partial note.
    fn evidence_for_loop(
        targets: &FunctionTargetsHolder,
        func_env: &FunctionEnv<'_>,
        data: &FunctionData,
        loop_info: &LoopWithoutInvariant,
        depth: usize,
    ) -> LoopInvariantEvidenceForLoop {
        let carried_names = loop_info
            .carried
            .iter()
            .map(|(_, name)| name.clone())
            .collect::<Vec<_>>();
        let unavailable = |reason: String| LoopInvariantEvidenceForLoop {
            loop_id: loop_info.loop_id,
            depth,
            carried_names: carried_names.clone(),
            heads: vec![],
            partial_notes: vec![],
            unavailable: Some(reason),
        };

        if loop_info.carried.is_empty() {
            return unavailable(
                "no source-visible loop-carried locals were found for bounded WP evidence"
                    .to_string(),
            );
        }

        let Ok((loops_for_transform, loops_for_unrolling)) =
            fat_loop::build_loop_info_for_spec_with_forced_unroll(
                &FunctionTarget::new(func_env, data),
                targets,
                depth,
            )
        else {
            return unavailable(
                "the isolated bounded loop analysis could not be built".to_string(),
            );
        };
        if !loops_for_transform.fat_loops.is_empty() {
            return unavailable(
                "this function contains a loop that is summarized rather than unrolled".to_string(),
            );
        }
        let unrolled_loops = loops_for_unrolling.fat_loops.len();

        let mut shadow = Self::transform(func_env, data.clone(), &loops_for_transform);
        let mut selected_heads = None;
        for (header, mark) in loops_for_unrolling.fat_loops {
            let (next, heads) = Self::unroll(func_env, shadow, &header, &mark);
            shadow = next;
            if header == loop_info.header {
                selected_heads = Some(heads);
            }
        }
        shadow.loop_unrolling.clear();
        let Some(selected_heads) = selected_heads else {
            return unavailable("the missing loop was not present in the bounded DAG".to_string());
        };

        let mut heads = vec![];
        let mut partial_notes = vec![];
        let mut omitted_facts = 0;
        if loop_info.omitted_carried > 0 {
            partial_notes.push(format!(
                "{} compiler-generated or memory loop target(s) were omitted from the source-level view",
                loop_info.omitted_carried
            ));
        }
        for (head_index, head_label) in selected_heads {
            let mut cutpoint_data = shadow.clone();
            let Some(label_offset) = cutpoint_data.code.iter().position(
                |bytecode| matches!(bytecode, Bytecode::Label(_, label) if *label == head_label),
            ) else {
                partial_notes.push(format!(
                    "head[{}] was absent from the bounded DAG",
                    head_index
                ));
                continue;
            };
            let cutpoint_offset = label_offset + 1;
            let attr_id = cutpoint_data.code[label_offset].get_attr_id();
            cutpoint_data.code.insert(
                cutpoint_offset,
                Bytecode::Call(attr_id, vec![], Operation::Stop, vec![], None),
            );
            let observation = infer_loop_head_evidence(
                func_env,
                &cutpoint_data,
                cutpoint_offset as CodeOffset,
                head_index,
                &loop_info.carried,
            );
            if observation.incomplete {
                partial_notes.push(format!(
                    "WP did not cover every path to head[{}]",
                    head_index
                ));
            }
            if observation.omitted_facts > 0 {
                omitted_facts += observation.omitted_facts;
            }
            heads.push(LoopHeadObservation {
                index: head_index,
                facts: observation.facts,
            });
        }
        if omitted_facts > 0 {
            partial_notes.push(format!(
                "{} fact(s) across the bounded heads used internal names or exceeded the display cap and were omitted",
                omitted_facts
            ));
        }
        if unrolled_loops > 1 {
            partial_notes.push(format!(
                "{} loops in this function are unrolled; facts at a loop reached through an \
                 earlier one hold for that bounded prefix only",
                unrolled_loops
            ));
        }
        partial_notes.sort();
        partial_notes.dedup();
        let unavailable = heads
            .iter()
            .all(|head| head.facts.is_empty())
            .then(|| "no source-level loop-head relation survived WP simplification".to_string());
        LoopInvariantEvidenceForLoop {
            loop_id: loop_info.loop_id,
            depth,
            carried_names,
            heads,
            partial_notes,
            unavailable,
        }
    }

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
        loop_annotation: &FatLoopFunctionInfo,
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
                        for (i, (attr_id, exp)) in
                            loop_info.spec_info().invariants.values().enumerate()
                        {
                            // insert write-back actions before the first assertion
                            if i == 0 {
                                if let Some((info, nodes)) =
                                    builder.data.loop_invariant_write_back_map.get(attr_id)
                                {
                                    let info_clone = info.clone();
                                    let nodes_clone = nodes.clone();
                                    Instrumenter::instrument_write_back_for_spec(
                                        &mut builder,
                                        &info_clone,
                                        nodes_clone,
                                    );
                                }
                            }
                            builder.set_loc_and_vc_info(
                                builder.get_loc(*attr_id),
                                LOOP_INVARIANT_BASE_FAILED,
                            );
                            builder.emit_with(|attr_id| {
                                Bytecode::Prop(attr_id, PropKind::Assert, exp.clone())
                            });
                        }

                        // havoc all loop targets
                        for idx in &loop_info.spec_info().val_targets {
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
                        for (idx, havoc_all) in &loop_info.spec_info().mut_targets {
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
                            let id = builder.new_attr();
                            builder.emit(Bytecode::Prop(id, PropKind::Assume, exp));
                            builder.data.shallow_wellformed_assumes.insert(id);
                        }

                        // havoc all memory the loop body may modify, so the
                        // loop-exit path does not retain pre-loop memory; the
                        // frame is re-established by user loop invariants
                        if !options.for_interpretation {
                            for mem in &loop_info.spec_info().mem_targets {
                                let mem = mem.clone();
                                builder.emit_with(|attr_id| {
                                    Bytecode::Call(
                                        attr_id,
                                        vec![],
                                        Operation::HavocGlobal(
                                            mem.module_id,
                                            mem.id,
                                            mem.inst.clone(),
                                        ),
                                        vec![],
                                        None,
                                    )
                                });
                                // re-assume well-formedness of the memory contents
                                if let Some(exp) = builder.mk_inst_mem_quant_opt(
                                    QuantKind::Forall,
                                    &mem,
                                    &mut |val| {
                                        Some(builder.mk_call(
                                            &Type::Primitive(PrimitiveType::Bool),
                                            ast::Operation::WellFormed,
                                            vec![val],
                                        ))
                                    },
                                ) {
                                    builder.emit_with(move |id| {
                                        Bytecode::Prop(id, PropKind::Assume, exp)
                                    });
                                }
                            }
                        }

                        // trace implicitly reassigned variables after havocking
                        let affected_variables: BTreeSet<_> = loop_info
                            .spec_info()
                            .val_targets
                            .iter()
                            .chain(loop_info.spec_info().mut_targets.keys())
                            .collect();

                        // Only emit this for user declared locals, not for ones introduced
                        // by stack elimination.
                        let affected_non_temporary_variables: BTreeSet<_> = affected_variables
                            .into_iter()
                            .filter(|&idx| {
                                !func_env
                                    .is_temporary(*idx)
                                    .expect("compiled module available")
                            })
                            .collect();

                        if affected_non_temporary_variables.is_empty() {
                            // no user declared local is havocked
                            builder.set_next_debug_comment(format!(
                                "info: enter loop {}",
                                match loop_info.spec_info().invariants.is_empty() {
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
                            && !loop_info.spec_info().invariants.is_empty()
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
                        for (attr_id, exp) in loop_info.spec_info().invariants.values() {
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
            for (i, (attr_id, exp)) in loop_info.spec_info().invariants.values().enumerate() {
                // insert write-back actions before the first assertion
                if i == 0 {
                    if let Some((info, nodes)) =
                        builder.data.loop_invariant_write_back_map.get(attr_id)
                    {
                        let info_clone = info.clone();
                        let nodes_clone = nodes.clone();
                        Instrumenter::instrument_write_back_for_spec(
                            &mut builder,
                            &info_clone,
                            nodes_clone,
                        );
                    }
                }
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
    ) -> (FunctionData, BTreeMap<usize, Label>) {
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
        let mut head_labels = BTreeMap::from([(0, *loop_header)]);
        for i in 0..unrolling_mark.iter_count {
            head_labels.insert(
                i + 1,
                *label_remapping
                    .get(&(*loop_header, i))
                    .expect("unrolled loop header label"),
            );
        }

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

        (builder.data, head_labels)
    }
}
