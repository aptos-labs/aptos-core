// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Implements a pipeline that checks and gives warning on unused assignments.
//! Prerequisite: live variable annotation.

use crate::pipeline::livevar_analysis_processor::LiveVarAnnotation;
use codespan_reporting::diagnostic::Severity;
use move_binary_format::file_format::CodeOffset;
use move_model::{ast::TempIndex, model::FunctionEnv};
use move_stackless_bytecode::{
    function_target::{FunctionData, FunctionTarget},
    function_target_pipeline::{FunctionTargetProcessor, FunctionTargetsHolder},
    stackless_bytecode::{AttrId, Bytecode},
};

pub struct UnusedAssignmentChecker {}

impl UnusedAssignmentChecker {
    /// Check if the assignment to `dst` is used at offset after the position given by `offset` and `after`.
    fn check_unused_assignment(
        target: &FunctionTarget,
        id: AttrId,
        offset: CodeOffset,
        dst: TempIndex,
    ) {
        let loc = target.get_bytecode_loc(id);
        // skip inlined code
        if loc.is_inlined() {
            return;
        }
        let data = target.data;
        // only check for user defined variables
        if let Some(dst_name) = data.local_names.get(&dst) {
            let live_var_info = target
                .get_annotations()
                .get::<LiveVarAnnotation>()
                .expect("live variable annotation")
                .get_info_at(offset);
            let live_after = &live_var_info.after;
            let dst_name = dst_name.display(target.func_env.symbol_pool()).to_string();
            if !dst_name.starts_with('_') && live_after.get(&dst).is_none() {
                target
                    .global_env()
                    .diag(
                        Severity::Warning,
                        &loc,
                        &format!("Unused assignment to `{}`. Consider removing or prefixing with an underscore: `_{}`", dst_name, dst_name)
                    );
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
        if func_env.is_native() {
            return data;
        }
        let target = FunctionTarget::new(func_env, &data);
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
