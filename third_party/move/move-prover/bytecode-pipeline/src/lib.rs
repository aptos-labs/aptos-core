// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE
pub mod clean_and_optimize;
pub mod data_invariant_instrumentation;
pub mod eliminate_imm_refs;
pub mod global_invariant_analysis;
pub mod global_invariant_instrumentation;
pub mod inconsistency_check;
pub mod loop_analysis;
pub mod memory_instrumentation;
pub mod mono_analysis;
pub mod mut_ref_instrumentation;
pub mod number_operation;
pub mod number_operation_analysis;
pub mod options;
pub mod pipeline_factory;
pub mod spec_instrumentation;
pub mod verification_analysis;
pub mod well_formed_instrumentation;
