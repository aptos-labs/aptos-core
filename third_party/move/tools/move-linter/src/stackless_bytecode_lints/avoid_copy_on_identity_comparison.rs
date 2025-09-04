// Copyright (c) Velor Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module implements a stackless-bytecode linter that checks identity comparisons
//! between copied values (of type vector or struct, which can involve expensive copies)
//! and suggests to use reference-based identity comparison instead.
//! For example, instead of `a == b`, use `&a == &b` (where `a` or `b` has to be copied,
//! and are of type vector or struct).
//! The comparison itself can still be expensive, but using references for the comparison
//! can avoid unnecessary copies.

use move_binary_format::file_format::CodeOffset;
use move_compiler_v2::{
    external_checks::StacklessBytecodeChecker,
    pipeline::livevar_analysis_processor::LiveVarAnnotation,
};
use move_stackless_bytecode::{
    function_target::FunctionTarget,
    stackless_bytecode::{Bytecode, Operation},
};

pub struct AvoidCopyOnIdentityComparison {}

impl StacklessBytecodeChecker for AvoidCopyOnIdentityComparison {
    fn get_name(&self) -> String {
        "avoid_copy_on_identity_comparison".to_string()
    }

    fn check(&self, target: &FunctionTarget) {
        let code = target.get_bytecode();
        let live_vars = target
            .get_annotations()
            .get::<LiveVarAnnotation>()
            .expect("live variable annotation is a prerequisite");
        for (offset, bc) in code.iter().enumerate() {
            if let Bytecode::Call(id, _, Operation::Eq | Operation::Neq, sources, _) = bc {
                let (lhs, rhs) = (sources[0], sources[1]);
                // For `Eq` and `Neq`, the type of both operands must be the same, so we only get
                // the lhs type.
                let ty = target.get_local_type(lhs);
                if ty.is_vector() || ty.is_struct() {
                    let live_info = live_vars.get_info_at(offset as CodeOffset);
                    // If either `lhs` or `rhs` is live after the comparison, then at least one of them must be copied
                    // for this comparison.
                    if live_info.after.contains_key(&lhs) || live_info.after.contains_key(&rhs) {
                        self.report(
                            target.global_env(),
                            &target.get_bytecode_loc(*id),
                            "Compare using references of these values instead (i.e., place `&` on both the operands), to avoid unnecessary copies.",
                        );
                    }
                }
            }
        }
    }
}
