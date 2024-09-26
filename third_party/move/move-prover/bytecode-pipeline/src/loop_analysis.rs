// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::options::ProverOptions;
use move_binary_format::file_format::CodeOffset;
use move_model::{
    ast::{self},
    exp_generator::ExpGenerator,
    model::FunctionEnv,
    ty::{PrimitiveType, Type},
};
use move_stackless_bytecode::{
    fat_loop,
    fat_loop::{FatLoopFunctionInfo, LoopUnrollingMark},
    function_data_builder::{FunctionDataBuilder, FunctionDataBuilderOptions},
    function_target::{FunctionData, FunctionTarget},
    function_target_pipeline::{FunctionTargetProcessor, FunctionTargetsHolder},
    stackless_bytecode::{Bytecode, HavocKind, Label, Operation, PropKind},
};
use std::collections::{BTreeMap, BTreeSet};

const LOOP_INVARIANT_BASE_FAILED: &str = "base case of the loop invariant does not hold";
const LOOP_INVARIANT_INDUCTION_FAILED: &str = "induction case of the loop invariant does not hold";

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
        match fat_loop::build_loop_info_for_spec(&FunctionTarget::new(func_env, &data)) {
            Ok((loops_with_invariants, loops_for_unrolling)) => {
                let mut data = Self::transform(func_env, data, &loops_with_invariants);
                for (header_label, unrolling_instruction) in loops_for_unrolling.fat_loops {
                    data = Self::unroll(func_env, data, &header_label, &unrolling_instruction);
                }
                // we have unrolled the loop into a DAG, and there will be no loop unrolling marks left
                data.loop_unrolling.clear();
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
                        for (attr_id, exp) in loop_info.spec_info().invariants.values() {
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
                            builder.emit_with(move |id| Bytecode::Prop(id, PropKind::Assume, exp));
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
            for (attr_id, exp) in loop_info.spec_info().invariants.values() {
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
}
