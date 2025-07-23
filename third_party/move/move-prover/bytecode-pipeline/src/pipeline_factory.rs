// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    clean_and_optimize::CleanAndOptimizeProcessor,
    data_invariant_instrumentation::DataInvariantInstrumentationProcessor,
    eliminate_imm_refs::EliminateImmRefsProcessor,
    global_invariant_analysis::GlobalInvariantAnalysisProcessor,
    global_invariant_instrumentation::GlobalInvariantInstrumentationProcessor,
    inconsistency_check::InconsistencyCheckInstrumenter,
    loop_analysis::LoopAnalysisProcessor,
    memory_instrumentation::MemoryInstrumentationProcessor,
    mono_analysis::MonoAnalysisProcessor,
    mut_ref_instrumentation::MutRefInstrumenter,
    number_operation_analysis::NumberOperationProcessor,
    options::ProverOptions,
    spec_instrumentation::SpecInstrumentationProcessor,
    variant_context_analysis::VariantContextAnalysisProcessor,
    verification_analysis::VerificationAnalysisProcessor,
    well_formed_instrumentation::WellFormedInstrumentationProcessor,
};
use move_stackless_bytecode::{
    borrow_analysis::BorrowAnalysisProcessor,
    debug_instrumentation::DebugInstrumenter,
    function_target_pipeline::{FunctionTargetPipeline, FunctionTargetProcessor},
    livevar_analysis::LiveVarAnalysisProcessor,
    reaching_def_analysis::ReachingDefProcessor,
    usage_analysis::UsageProcessor,
};

pub fn default_pipeline_with_options(options: &ProverOptions) -> FunctionTargetPipeline {
    // NOTE: the order of these processors is import!
    let mut processors: Vec<Box<dyn FunctionTargetProcessor>> = vec![
        DebugInstrumenter::new(),
        // transformation and analysis
        EliminateImmRefsProcessor::new(),
        MutRefInstrumenter::new(),
        ReachingDefProcessor::new(),
        LiveVarAnalysisProcessor::new(),
        BorrowAnalysisProcessor::new_borrow_natives(options.borrow_natives.clone()),
        MemoryInstrumentationProcessor::new(),
        CleanAndOptimizeProcessor::new(),
        UsageProcessor::new(),
        VerificationAnalysisProcessor::new(),
    ];

    if !options.skip_loop_analysis {
        processors.push(LoopAnalysisProcessor::new());
    }

    processors.append(&mut vec![
        // spec instrumentation
        SpecInstrumentationProcessor::new(),
        GlobalInvariantAnalysisProcessor::new(),
        GlobalInvariantInstrumentationProcessor::new(),
        WellFormedInstrumentationProcessor::new(),
        // variant context analysis (must come before data invariant instrumentation)
        VariantContextAnalysisProcessor::new(),
        DataInvariantInstrumentationProcessor::new(),
        // monomorphization
        MonoAnalysisProcessor::new(),
    ]);

    // inconsistency check instrumentation should be the last one in the pipeline
    if options.check_inconsistency {
        processors.push(InconsistencyCheckInstrumenter::new());
    }

    if !options.for_interpretation {
        processors.push(NumberOperationProcessor::new());
    }

    let mut res = FunctionTargetPipeline::default();
    for p in processors {
        res.add_processor(p);
    }
    res
}

#[allow(unused)]
pub fn default_pipeline() -> FunctionTargetPipeline {
    default_pipeline_with_options(&ProverOptions::default())
}

pub fn experimental_pipeline() -> FunctionTargetPipeline {
    // Enter your pipeline here
    let processors: Vec<Box<dyn FunctionTargetProcessor>> = vec![
        DebugInstrumenter::new(),
        // transformation and analysis
        EliminateImmRefsProcessor::new(),
        MutRefInstrumenter::new(),
        ReachingDefProcessor::new(),
        LiveVarAnalysisProcessor::new(),
        BorrowAnalysisProcessor::new(),
        MemoryInstrumentationProcessor::new(),
        CleanAndOptimizeProcessor::new(),
        UsageProcessor::new(),
        VerificationAnalysisProcessor::new(),
        LoopAnalysisProcessor::new(),
        // spec instrumentation
        SpecInstrumentationProcessor::new(),
        DataInvariantInstrumentationProcessor::new(),
        GlobalInvariantAnalysisProcessor::new(),
        GlobalInvariantInstrumentationProcessor::new(),
        // optimization
        MonoAnalysisProcessor::new(),
    ];

    let mut res = FunctionTargetPipeline::default();
    for p in processors {
        res.add_processor(p);
    }
    res
}
