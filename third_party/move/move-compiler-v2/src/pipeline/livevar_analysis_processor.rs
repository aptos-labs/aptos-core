// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Implements a live-variable analysis processor, annotating lifetime information about locals.
//! See also https://en.wikipedia.org/wiki/Live-variable_analysis

use move_model::model::FunctionEnv;
use move_stackless_bytecode::{
    function_target::{FunctionData, FunctionTarget},
    function_target_pipeline::{FunctionTargetProcessor, FunctionTargetsHolder},
    livevar_analysis,
};

pub struct LiveVarAnalysisProcessor();

impl FunctionTargetProcessor for LiveVarAnalysisProcessor {
    fn process(
        &self,
        _targets: &mut FunctionTargetsHolder,
        fun_env: &FunctionEnv,
        mut data: FunctionData,
        _scc_opt: Option<&[FunctionEnv]>,
    ) -> FunctionData {
        // Call the existing live-var analysis from the move-prover.
        let target = FunctionTarget::new(fun_env, &data);
        let offset_to_live_refs = livevar_analysis::LiveVarAnnotation::from_map(
            livevar_analysis::run_livevar_analysis(&target, &data.code),
        );
        // Annotate the result on the function data.
        data.annotations.set(offset_to_live_refs, true);
        data
    }

    fn name(&self) -> String {
        "LiveVarAnalysisProcessor".to_owned()
    }
}

impl LiveVarAnalysisProcessor {
    /// Registers annotation formatter at the given function target. This is for debugging and
    /// testing.
    pub fn register_formatters(target: &FunctionTarget) {
        target.register_annotation_formatter(Box::new(livevar_analysis::format_livevar_annotation))
    }
}
