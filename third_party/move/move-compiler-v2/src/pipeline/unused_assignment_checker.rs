// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Implements a pipeline that checks and gives warning on unused bindings and assignments.
//! Prerequisite: live variable annotation.

use crate::pipeline::livevar_analysis_processor::LiveVarAnnotation;
use codespan_reporting::diagnostic::Severity;
use move_binary_format::file_format::CodeOffset;
use move_model::{ast::TempIndex, model::FunctionEnv, well_known::RECEIVER_PARAM_NAME};
use move_stackless_bytecode::{
    function_target::{FunctionData, FunctionTarget},
    function_target_pipeline::{FunctionTargetProcessor, FunctionTargetsHolder},
    stackless_bytecode::{AttrId, Bytecode},
};

pub struct UnusedAssignmentChecker {}

impl UnusedAssignmentChecker {
    /// Check if the assignment to `dst` is used after the position given by `offset`.
    fn check_unused_assignment(
        target: &FunctionTarget,
        id: AttrId,
        offset: CodeOffset,
        dst: TempIndex,
    ) {
        let loc = target.get_bytecode_loc(id);
        // Skip inlined code.
        if loc.is_inlined() {
            return;
        }
        let live_var_annotations = target
            .get_annotations()
            .get::<LiveVarAnnotation>()
            .expect("live variable annotation");
        // Only check for user defined variables.
        if let Some(dst_name) = target.get_local_name_opt(dst) {
            let live_var_info = live_var_annotations.get_info_at(offset);
            let live_after = &live_var_info.after;
            if !dst_name.starts_with('_') && live_after.get(&dst).is_none() {
                target.global_env().diag(
                    Severity::Warning,
                    &loc,
                    &format!(
                        "This assignment/binding to the left-hand-side variable `{}` is unused. \
                        Consider removing this assignment/binding, \
                        or prefixing the left-hand-side variable with an underscore (e.g., `_{}`), or renaming to `_`",
                        dst_name, dst_name
                    ),
                );
            }
        }
    }

    /// Check if `target` function's parameters are unused.
    fn check_params(target: &FunctionTarget) {
        let live_after_first_instr = &target
            .get_annotations()
            .get::<LiveVarAnnotation>()
            .expect("live variable annotation should be present")
            .get_info_at(0 as CodeOffset)
            .after;
        let sources_of_first_instr = target.get_bytecode()[0].sources();
        let dests_of_first_instr = target.get_bytecode()[0].dests();
        let params = target.func_env.get_parameters();
        let temps_used_in_specs = target.get_temps_used_in_spec_blocks();
        for (param_index, param) in params.iter().enumerate() {
            if temps_used_in_specs.contains(&param_index) {
                // If the parameter is used in the spec blocks, we do not warn about it.
                continue;
            }
            if let Some(param_name) = target.get_local_name_opt(param_index) {
                if !param_name.starts_with('_') && param_name != RECEIVER_PARAM_NAME {
                    {
                        let live_var_info = live_after_first_instr.get(&param_index);
                        // If a parameter is not a source in the first instruction and is:
                        // - a destination of the first instruction (which means it is overwritten), or
                        // - not live after the first instruction (which means it is not used after the first instruction), then
                        // it is unused.
                        if !sources_of_first_instr.contains(&param_index)
                            && (dests_of_first_instr.contains(&param_index)
                                || live_var_info.is_none())
                        {
                            target.global_env().diag(
                                Severity::Warning,
                                &param.get_loc(),
                                format!(
                                    "Unused value of parameter `{}`. Consider removing the parameter, \
                                    or prefixing with an underscore (e.g., `_{}`), or binding to `_`",
                                    param_name, param_name
                                )
                                .as_str(),
                            );
                        }
                    }
                }
            }
        }
    }
}

impl FunctionTargetProcessor for UnusedAssignmentChecker {
    fn process(
        &self,
        _targets: &mut FunctionTargetsHolder,
        func_env: &FunctionEnv,
        data: FunctionData,
        _scc_opt: Option<&[FunctionEnv]>,
    ) -> FunctionData {
        if func_env.is_native() || func_env.is_inline() {
            return data;
        }
        let target = FunctionTarget::new(func_env, &data);

        UnusedAssignmentChecker::check_params(&target);
        for (offset, bytecode) in data.code.iter().enumerate() {
            let offset = offset as u16;
            use Bytecode::*;
            match bytecode {
                Load(id, dst, _) | Assign(id, dst, _, _) => {
                    UnusedAssignmentChecker::check_unused_assignment(&target, *id, offset, *dst)
                },
                Call(id, dsts, _, _, _) => {
                    for dst in dsts {
                        UnusedAssignmentChecker::check_unused_assignment(&target, *id, offset, *dst)
                    }
                },
                _ => {},
            }
        }
        data
    }

    fn name(&self) -> String {
        "UnusedAssignmentChecker".to_string()
    }
}
