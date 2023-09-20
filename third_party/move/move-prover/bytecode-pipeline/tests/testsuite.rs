// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use anyhow::anyhow;
use move_prover_bytecode_pipeline::{
    clean_and_optimize::CleanAndOptimizeProcessor,
    data_invariant_instrumentation::DataInvariantInstrumentationProcessor,
    eliminate_imm_refs::EliminateImmRefsProcessor,
    global_invariant_analysis::GlobalInvariantAnalysisProcessor,
    global_invariant_instrumentation::GlobalInvariantInstrumentationProcessor,
    memory_instrumentation::MemoryInstrumentationProcessor, mono_analysis::MonoAnalysisProcessor,
    mut_ref_instrumentation::MutRefInstrumenter,
    spec_instrumentation::SpecInstrumentationProcessor,
    verification_analysis::VerificationAnalysisProcessor,
    well_formed_instrumentation::WellFormedInstrumentationProcessor,
};
use move_stackless_bytecode::{
    borrow_analysis::BorrowAnalysisProcessor, function_target_pipeline::FunctionTargetPipeline,
    livevar_analysis::LiveVarAnalysisProcessor, reaching_def_analysis::ReachingDefProcessor,
    usage_analysis::UsageProcessor,
};
use std::path::Path;

fn get_tested_transformation_pipeline(
    dir_name: &str,
) -> anyhow::Result<Option<FunctionTargetPipeline>> {
    match dir_name {
        "eliminate_imm_refs" => {
            let mut pipeline = FunctionTargetPipeline::default();
            pipeline.add_processor(EliminateImmRefsProcessor::new());
            Ok(Some(pipeline))
        },
        "mut_ref_instrumentation" => {
            let mut pipeline = FunctionTargetPipeline::default();
            pipeline.add_processor(EliminateImmRefsProcessor::new());
            pipeline.add_processor(MutRefInstrumenter::new());
            Ok(Some(pipeline))
        },
        "memory_instr" => {
            let mut pipeline = FunctionTargetPipeline::default();
            pipeline.add_processor(EliminateImmRefsProcessor::new());
            pipeline.add_processor(MutRefInstrumenter::new());
            pipeline.add_processor(ReachingDefProcessor::new());
            pipeline.add_processor(LiveVarAnalysisProcessor::new());
            pipeline.add_processor(BorrowAnalysisProcessor::new());
            pipeline.add_processor(MemoryInstrumentationProcessor::new());
            Ok(Some(pipeline))
        },
        "clean_and_optimize" => {
            let mut pipeline = FunctionTargetPipeline::default();
            pipeline.add_processor(EliminateImmRefsProcessor::new());
            pipeline.add_processor(MutRefInstrumenter::new());
            pipeline.add_processor(ReachingDefProcessor::new());
            pipeline.add_processor(LiveVarAnalysisProcessor::new());
            pipeline.add_processor(BorrowAnalysisProcessor::new());
            pipeline.add_processor(MemoryInstrumentationProcessor::new());
            pipeline.add_processor(CleanAndOptimizeProcessor::new());
            Ok(Some(pipeline))
        },
        "verification_analysis" => {
            let mut pipeline = FunctionTargetPipeline::default();
            pipeline.add_processor(EliminateImmRefsProcessor::new());
            pipeline.add_processor(MutRefInstrumenter::new());
            pipeline.add_processor(ReachingDefProcessor::new());
            pipeline.add_processor(LiveVarAnalysisProcessor::new());
            pipeline.add_processor(BorrowAnalysisProcessor::new());
            pipeline.add_processor(MemoryInstrumentationProcessor::new());
            pipeline.add_processor(CleanAndOptimizeProcessor::new());
            pipeline.add_processor(UsageProcessor::new());
            pipeline.add_processor(VerificationAnalysisProcessor::new());
            Ok(Some(pipeline))
        },
        "spec_instrumentation" => {
            let mut pipeline = FunctionTargetPipeline::default();
            pipeline.add_processor(EliminateImmRefsProcessor::new());
            pipeline.add_processor(MutRefInstrumenter::new());
            pipeline.add_processor(ReachingDefProcessor::new());
            pipeline.add_processor(LiveVarAnalysisProcessor::new());
            pipeline.add_processor(BorrowAnalysisProcessor::new());
            pipeline.add_processor(MemoryInstrumentationProcessor::new());
            pipeline.add_processor(CleanAndOptimizeProcessor::new());
            pipeline.add_processor(UsageProcessor::new());
            pipeline.add_processor(VerificationAnalysisProcessor::new());
            pipeline.add_processor(SpecInstrumentationProcessor::new());
            Ok(Some(pipeline))
        },
        "data_invariant_instrumentation" => {
            let mut pipeline = FunctionTargetPipeline::default();
            pipeline.add_processor(EliminateImmRefsProcessor::new());
            pipeline.add_processor(MutRefInstrumenter::new());
            pipeline.add_processor(ReachingDefProcessor::new());
            pipeline.add_processor(LiveVarAnalysisProcessor::new());
            pipeline.add_processor(BorrowAnalysisProcessor::new());
            pipeline.add_processor(MemoryInstrumentationProcessor::new());
            pipeline.add_processor(CleanAndOptimizeProcessor::new());
            pipeline.add_processor(UsageProcessor::new());
            pipeline.add_processor(VerificationAnalysisProcessor::new());
            pipeline.add_processor(SpecInstrumentationProcessor::new());
            pipeline.add_processor(GlobalInvariantAnalysisProcessor::new());
            pipeline.add_processor(WellFormedInstrumentationProcessor::new());
            pipeline.add_processor(DataInvariantInstrumentationProcessor::new());
            Ok(Some(pipeline))
        },
        "global_invariant_analysis" => {
            let mut pipeline = FunctionTargetPipeline::default();
            pipeline.add_processor(EliminateImmRefsProcessor::new());
            pipeline.add_processor(MutRefInstrumenter::new());
            pipeline.add_processor(ReachingDefProcessor::new());
            pipeline.add_processor(LiveVarAnalysisProcessor::new());
            pipeline.add_processor(BorrowAnalysisProcessor::new());
            pipeline.add_processor(MemoryInstrumentationProcessor::new());
            pipeline.add_processor(CleanAndOptimizeProcessor::new());
            pipeline.add_processor(UsageProcessor::new());
            pipeline.add_processor(VerificationAnalysisProcessor::new());
            pipeline.add_processor(SpecInstrumentationProcessor::new());
            pipeline.add_processor(GlobalInvariantAnalysisProcessor::new());
            Ok(Some(pipeline))
        },
        "global_invariant_instrumentation" => {
            let mut pipeline = FunctionTargetPipeline::default();
            pipeline.add_processor(EliminateImmRefsProcessor::new());
            pipeline.add_processor(MutRefInstrumenter::new());
            pipeline.add_processor(ReachingDefProcessor::new());
            pipeline.add_processor(LiveVarAnalysisProcessor::new());
            pipeline.add_processor(BorrowAnalysisProcessor::new());
            pipeline.add_processor(MemoryInstrumentationProcessor::new());
            pipeline.add_processor(CleanAndOptimizeProcessor::new());
            pipeline.add_processor(UsageProcessor::new());
            pipeline.add_processor(VerificationAnalysisProcessor::new());
            pipeline.add_processor(SpecInstrumentationProcessor::new());
            pipeline.add_processor(GlobalInvariantAnalysisProcessor::new());
            pipeline.add_processor(GlobalInvariantInstrumentationProcessor::new());
            Ok(Some(pipeline))
        },
        "mono_analysis" => {
            let mut pipeline = FunctionTargetPipeline::default();
            pipeline.add_processor(UsageProcessor::new());
            pipeline.add_processor(VerificationAnalysisProcessor::new());
            pipeline.add_processor(SpecInstrumentationProcessor::new());
            pipeline.add_processor(GlobalInvariantAnalysisProcessor::new());
            pipeline.add_processor(WellFormedInstrumentationProcessor::new());
            pipeline.add_processor(DataInvariantInstrumentationProcessor::new());
            pipeline.add_processor(MonoAnalysisProcessor::new());
            Ok(Some(pipeline))
        },
        _ => Err(anyhow!(
            "the sub-directory `{}` has no associated pipeline to test",
            dir_name
        )),
    }
}

fn test_runner(path: &Path) -> datatest_stable::Result<()> {
    let dir_name = path
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|p| p.to_str())
        .ok_or_else(|| anyhow!("bad file name"))?;
    let pipeline_opt = get_tested_transformation_pipeline(dir_name)?;
    move_stackless_bytecode_test_utils::test_runner(path, pipeline_opt)?;
    Ok(())
}

datatest_stable::harness!(test_runner, "tests", r".*\.move");
