// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Implements copy propagation transformation.
//!
//! prerequisite: the `AvailCopiesAnnotation` should already be computed by running the `AvailCopiesAnalysis`.
//! side effect: all annotations will be removed from the function target annotations as the code
//!     potentially change, possibly rendering the annotations incorrect.
//!
//! Given definitely available copies at each program point, this transformation replaces the use of locals
//! with their copy-chain heads, possibly rendering several copies redundant (i.e., creating dead stores).
//! For example, consider the following code:
//! ```move
//! let b = a;
//! let c = b;
//! let d = c + 1;
//! ```
//! This transformation will modify the code will to:
//! ```move
//! let b = a;  // redundant copy
//! let c = a;  // redundant copy
//! let d = a + 1;
//! ```

use crate::pipeline::avail_copies_analysis::{AvailCopies, AvailCopiesAnnotation};
use move_binary_format::file_format::CodeOffset;
use move_model::model::FunctionEnv;
use move_stackless_bytecode::{
    function_target::{FunctionData, FunctionTarget},
    function_target_pipeline::{FunctionTargetProcessor, FunctionTargetsHolder},
    stackless_bytecode::Bytecode,
};

/// A processor which performs copy propagation transformation.
pub struct CopyPropagation {}

impl CopyPropagation {
    /// Transforms the `code` of a `target` function using the `avail_copies_annotation`,
    /// by replacing the use of locals with their copy-chain heads.
    /// Returns the transformed code.
    fn transform(
        target: &FunctionTarget,
        code: Vec<Bytecode>,
        avail_copies_annotation: &AvailCopiesAnnotation,
    ) -> Vec<Bytecode> {
        let mut new_code = vec![];
        let default_avail_copies = AvailCopies::default();
        for (offset, instr) in code.into_iter().enumerate() {
            let avail_copies = avail_copies_annotation
                .before(&(offset as CodeOffset))
                .unwrap_or(&default_avail_copies);
            let mut propagated_src = |dst| avail_copies.get_head_of_copy_chain(dst);
            new_code.push(instr.remap_src_vars(target, &mut propagated_src));
        }
        new_code
    }
}

impl FunctionTargetProcessor for CopyPropagation {
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
        let avail_copies = target
            .get_annotations()
            .get::<AvailCopiesAnnotation>()
            .expect("avail copies annotation is a prerequisite");
        let new_code = Self::transform(&target, code, avail_copies);
        data.code = new_code;
        // Annotations may no longer be valid after this transformation, so remove them.
        data.annotations.clear();
        data
    }

    fn name(&self) -> String {
        "CopyPropagation".to_string()
    }
}
