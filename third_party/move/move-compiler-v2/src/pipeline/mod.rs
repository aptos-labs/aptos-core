// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::pipeline::{
    avail_copies_analysis::AvailCopiesAnalysisProcessor,
    exit_state_analysis::ExitStateAnalysisProcessor,
    livevar_analysis_processor::LiveVarAnalysisProcessor,
    reference_safety_processor::ReferenceSafetyProcessor,
    uninitialized_use_checker::UninitializedUseChecker,
    unreachable_code_analysis::UnreachableCodeProcessor, variable_coalescing::VariableCoalescing,
};
use move_stackless_bytecode::function_target::FunctionTarget;

pub mod ability_processor;
pub mod avail_copies_analysis;
pub mod copy_propagation;
pub mod dead_store_elimination;
pub mod exit_state_analysis;
pub mod livevar_analysis_processor;
pub mod reference_safety_processor;
pub mod split_critical_edges_processor;
pub mod uninitialized_use_checker;
pub mod unreachable_code_analysis;
pub mod unreachable_code_remover;
pub mod unused_assignment_checker;
pub mod variable_coalescing;
pub mod visibility_checker;

/// Function to register all annotation formatters in the pipeline. Those are used
/// to visualize the result of an analysis as annotations on the bytecode, for
/// debugging.
pub fn register_formatters(target: &FunctionTarget) {
    ExitStateAnalysisProcessor::register_formatters(target);
    LiveVarAnalysisProcessor::register_formatters(target);
    ReferenceSafetyProcessor::register_formatters(target);
    AvailCopiesAnalysisProcessor::register_formatters(target);
    UninitializedUseChecker::register_formatters(target);
    UnreachableCodeProcessor::register_formatters(target);
    VariableCoalescing::register_formatters(target);
}
