// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Normalizes function exits so that every function has at most a single `Ret`
//! instruction at the end, preceded by a `Label` that all early returns jump to.
//!
//! This runs before `LoopAnalysisProcessor`, ensuring that early `return`s inside
//! loop bodies are converted to `Jump`s out of the loop. Without this, loop analysis
//! would leave `Ret` instructions inside the loop body intact, and the subsequent
//! spec-instrumentation rewrite of those `Ret`s into jumps to the unified exit would
//! create paths that bypass loop-invariant context, leading to unsound spec inference.
//!
//! When the function already has a single trailing `Ret`, no transformation is
//! applied — the existing exit is already unified.

use itertools::Itertools;
use move_model::{ast::TempIndex, exp_generator::ExpGenerator, model::FunctionEnv};
use move_stackless_bytecode::{
    function_data_builder::FunctionDataBuilder,
    function_target::FunctionData,
    function_target_pipeline::{FunctionTargetProcessor, FunctionTargetsHolder},
    stackless_bytecode::{AssignKind, Bytecode},
};

pub struct NormalizeExitsProcessor {}

impl NormalizeExitsProcessor {
    pub fn new() -> Box<Self> {
        Box::new(NormalizeExitsProcessor {})
    }
}

impl FunctionTargetProcessor for NormalizeExitsProcessor {
    fn process(
        &self,
        _targets: &mut FunctionTargetsHolder,
        func_env: &FunctionEnv,
        data: FunctionData,
        _scc_opt: Option<&[FunctionEnv]>,
    ) -> FunctionData {
        if func_env.is_native() || func_env.is_intrinsic() {
            return data;
        }

        let ret_count = data
            .code
            .iter()
            .filter(|bc| matches!(bc, Bytecode::Ret(..)))
            .count();
        let last_is_ret = matches!(data.code.last(), Some(Bytecode::Ret(..)));
        if ret_count == 0 || (ret_count == 1 && last_is_ret) {
            return data;
        }

        let mut builder = FunctionDataBuilder::new(func_env, data);
        let ret_locals: Vec<TempIndex> = builder
            .data
            .result_type
            .clone()
            .flatten()
            .into_iter()
            .map(|ty| builder.new_temp(ty))
            .collect_vec();
        let ret_label = builder.new_label();

        let old_code = std::mem::take(&mut builder.data.code);
        for bc in old_code {
            match bc {
                Bytecode::Ret(id, results) => {
                    builder.set_loc_from_attr(id);
                    for (i, r) in ret_locals.iter().copied().enumerate() {
                        builder
                            .emit_with(|id| Bytecode::Assign(id, r, results[i], AssignKind::Move));
                    }
                    builder.emit_with(|id| Bytecode::Jump(id, ret_label));
                },
                _ => builder.emit(bc),
            }
        }

        builder.set_loc(builder.fun_env.get_loc().at_end());
        builder.emit_with(|id| Bytecode::Label(id, ret_label));
        let final_ret_locals = ret_locals;
        builder.emit_with(move |id| Bytecode::Ret(id, final_ret_locals));

        builder.data
    }

    fn name(&self) -> String {
        "normalize_exits".to_string()
    }
}
