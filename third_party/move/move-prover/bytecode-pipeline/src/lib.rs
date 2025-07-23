// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

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
pub mod packed_types_analysis;
pub mod pipeline_factory;
pub mod spec_instrumentation;
pub mod variant_context_analysis;
pub mod verification_analysis;
pub mod well_formed_instrumentation;
