// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

//! Implements the "unreachable code remover" transformation.
//!
//! prerequisite: the `ReachableStateAnnotation` should already be computed by running the
//! `UnreachableCodeProcessor`.
//! side effect: all annotations will be removed from the function target annotations.
//!
//! Given reachable states information at each program point, this transformation removes
//! any definitely unreachable code.
//!
//! Note that any warnings about user's unreachable code should be emitted before running
//! this transformation.

use crate::pipeline::unreachable_code_analysis::ReachableStateAnnotation;
use move_binary_format::file_format::CodeOffset;
use move_model::model::FunctionEnv;
use move_stackless_bytecode::{
    function_target::{FunctionData, FunctionTarget},
    function_target_pipeline::{FunctionTargetProcessor, FunctionTargetsHolder},
    stackless_bytecode::Bytecode,
};

/// A processor which performs unreachable code removal transformation.
pub struct UnreachableCodeRemover {}

impl UnreachableCodeRemover {
    /// Transforms the `code` of a function using the `reachable_state_annotation`,
    /// by removing any definitely unreachable code.
    ///
    /// Returns the transformed code.
    fn transform(
        code: Vec<Bytecode>,
        reachable_state_annotation: &ReachableStateAnnotation,
    ) -> Vec<Bytecode> {
        let mut new_code = vec![];
        for (offset, instr) in code.into_iter().enumerate() {
            // If a program point is definitely not reachable, it is safe to remove that instruction
            // because no execution path starting at the beginning of the function can reach it
            // (and we cannot start execution from an arbitrary point in the function).
            if reachable_state_annotation.is_definitely_not_reachable(offset as CodeOffset) {
                continue; // skip emitting definitely unreachable code
            }
            new_code.push(instr);
        }
        new_code
    }
}

impl FunctionTargetProcessor for UnreachableCodeRemover {
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
        let reachable_state_annotation = target
            .get_annotations()
            .get::<ReachableStateAnnotation>()
            .expect("unreachable code annotation is a prerequisite");
        let new_code = Self::transform(code, reachable_state_annotation);
        data.code = new_code;
        // Annotations may no longer be valid after this transformation, because code offsets have changed.
        // So remove them.
        data.annotations.clear();
        data
    }

    fn name(&self) -> String {
        "UnreachableCodeRemover".to_string()
    }
}
