// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Implements the "dead store elimination" transformation. This transformation pairs well with
//! copy propagation transformation, as it removes the dead stores that copy propagation may introduce.
//!
//! prerequisite: the `LiveVarAnnotation` should already be computed by running the `LiveVarAnalysisProcessor`.
//! side effect: all annotations will be removed from the function target annotations.
//!
//! Given live variables at each program point, this transformation removes dead stores, i.e.,
//! assignments and loads to locals which are not live afterwards.
//! In addition, it also removes self-assignments, i.e., assignments of the form `x = x`.

use crate::pipeline::livevar_analysis_processor::LiveVarAnnotation;
use move_binary_format::file_format::CodeOffset;
use move_model::model::FunctionEnv;
use move_stackless_bytecode::{
    function_target::{FunctionData, FunctionTarget},
    function_target_pipeline::{FunctionTargetProcessor, FunctionTargetsHolder},
    stackless_bytecode::Bytecode,
};

/// A processor which performs dead store elimination transformation.
pub struct DeadStoreElimination {}

impl DeadStoreElimination {
    /// Transforms the `code` of a function using the `live_vars_annotation`,
    /// by removing assignments and loads to locals which are not live afterwards.
    /// Also removes self-assignments.
    ///
    /// Returns the transformed code.
    fn transform(code: Vec<Bytecode>, live_vars_annotation: &LiveVarAnnotation) -> Vec<Bytecode> {
        let mut new_code = vec![];
        for (offset, instr) in code.into_iter().enumerate() {
            if let Bytecode::Assign(_, dst, ..) | Bytecode::Load(_, dst, _) = instr {
                // Is the local that was just assigned to/loaded into, not live afterwards?
                // Then this is a dead store, we don't need to emit it.
                if !live_vars_annotation
                    .get_live_var_info_at(offset as CodeOffset)
                    .expect("live var info is a prerequisite")
                    .after
                    .contains_key(&dst)
                {
                    continue;
                }
            }
            if let Bytecode::Assign(_, dst, src, _) = instr {
                if dst == src {
                    // This is a self-assignment, we don't need to emit it.
                    continue;
                }
            }
            // None of the above special cases, so we emit the instruction.
            new_code.push(instr);
        }
        new_code
    }
}

impl FunctionTargetProcessor for DeadStoreElimination {
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
        let code = std::mem::take(&mut data.code);
        let target = FunctionTarget::new(func_env, &data);
        let live_var_annotation = target
            .get_annotations()
            .get::<LiveVarAnnotation>()
            .expect("live variable annotation is a prerequisite");
        let new_code = Self::transform(code, live_var_annotation);
        // Note that the file format generator will not include unused locals in the generated code,
        // so we don't need to prune unused locals here for various fields of `data` (like `local_types`).
        data.code = new_code;
        // Annotations may no longer be valid after this transformation because code offsets have changed.
        // So remove them.
        data.annotations.clear();
        data
    }

    fn name(&self) -> String {
        "DeadStoreElimination".to_string()
    }
}
