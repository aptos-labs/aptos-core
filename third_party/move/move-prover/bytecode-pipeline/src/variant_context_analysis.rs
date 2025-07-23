// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

//! Analysis which tracks which enum variant is active at each program point.
//! This is used by the data invariant instrumentation to apply the correct
//! invariants for each variant.

use move_binary_format::file_format::CodeOffset;
use move_model::{
    model::{FunctionEnv, QualifiedInstId, StructId},
    symbol::Symbol,
};
use move_stackless_bytecode::{
    dataflow_analysis::{DataflowAnalysis, TransferFunctions},
    dataflow_domains::{AbstractDomain, JoinResult},
    function_target::{FunctionData, FunctionTarget},
    function_target_pipeline::{FunctionTargetProcessor, FunctionTargetsHolder},
    stackless_bytecode::{Bytecode, Operation},
    stackless_control_flow_graph::StacklessControlFlowGraph,
};
use std::collections::BTreeMap;

/// Represents the variant context at a program point.
/// Maps enum structs to their active variant (if any).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct VariantContext {
    /// Maps (struct_id, temp_index) to the active variant for that struct instance
    active_variants: BTreeMap<(QualifiedInstId<StructId>, usize), Symbol>,
}

impl VariantContext {
    /// Get the active variant for a struct instance
    pub fn get_variant(&self, struct_id: &QualifiedInstId<StructId>, temp: usize) -> Option<Symbol> {
        self.active_variants.get(&(struct_id.clone(), temp)).copied()
    }

    /// Set the active variant for a struct instance
    pub fn set_variant(&mut self, struct_id: QualifiedInstId<StructId>, temp: usize, variant: Symbol) {
        self.active_variants.insert((struct_id, temp), variant);
    }

    /// Remove variant information for a temp (when it's reassigned)
    pub fn kill_variant(&mut self, temp: usize) {
        self.active_variants.retain(|(_, t), _| *t != temp);
    }

    /// Check if a struct instance has a known variant
    pub fn has_variant(&self, struct_id: &QualifiedInstId<StructId>, temp: usize) -> bool {
        self.active_variants.contains_key(&(struct_id.clone(), temp))
    }

    /// Get the number of active variants
    pub fn num_active_variants(&self) -> usize {
        self.active_variants.len()
    }

    /// Get all active variants for debugging
    pub fn get_all_active_variants(&self) -> &BTreeMap<(QualifiedInstId<StructId>, usize), Symbol> {
        &self.active_variants
    }
}

impl AbstractDomain for VariantContext {
    fn join(&mut self, other: &Self) -> JoinResult {
        let mut result = JoinResult::Unchanged;

        // For each variant in the other context
        for ((struct_id, temp), variant) in &other.active_variants {
            if let Some(existing_variant) = self.active_variants.get(&(struct_id.clone(), *temp)) {
                // If we have conflicting variants, we lose precision
                if existing_variant != variant {
                    self.active_variants.remove(&(struct_id.clone(), *temp));
                    result = JoinResult::Changed;
                }
            } else {
                // Add new variant information
                self.active_variants.insert((struct_id.clone(), *temp), *variant);
                result = JoinResult::Changed;
            }
        }

        result
    }
}

/// Annotation that maps each code offset to its variant context
#[derive(Clone, Default)]
pub struct VariantContextAnnotation(BTreeMap<CodeOffset, VariantContext>);

impl VariantContextAnnotation {
    /// Get the variant context before the instruction at the given offset
    pub fn get_context(&self, offset: CodeOffset) -> Option<&VariantContext> {
        self.0.get(&offset)
    }

    /// Get all contexts for debugging
    pub fn get_all_contexts(&self) -> &BTreeMap<CodeOffset, VariantContext> {
        &self.0
    }
}

/// Dataflow analysis to track variant context
pub struct VariantContextAnalysis {}

impl VariantContextAnalysis {
    pub fn new(_func_target: &FunctionTarget) -> Self {
        Self {}
    }
}

impl TransferFunctions for VariantContextAnalysis {
    type State = VariantContext;

    const BACKWARD: bool = false;

    fn execute(&self, state: &mut VariantContext, instr: &Bytecode, _offset: CodeOffset) {
        use Bytecode::*;
        use Operation::*;

        eprintln!("DEBUG: Processing instruction: {:?}", instr);

        match instr {
            // When we assign to a temp, kill any existing variant information
            Assign(_, dest, _, _) => {
                eprintln!("DEBUG: Assign to temp {}", dest);
                // Only kill variant info if we're assigning a different value
                // For now, be conservative and keep variant info
                // state.kill_variant(*dest);
            },

            // When we load into a temp, kill any existing variant information
            Load(_, dest, _) => {
                eprintln!("DEBUG: Load into temp {}", dest);
                // Only kill variant info if we're loading a different value
                // For now, be conservative and keep variant info
                // state.kill_variant(*dest);
            },

            // Handle variant-specific operations
            Call(_, dests, oper, srcs, _) => {
                eprintln!("DEBUG: Call operation: {:?}", oper);
                match oper {
                    // PackVariant creates a struct with a known variant
                    PackVariant(mid, sid, variant, targs) => {
                        eprintln!("DEBUG: PackVariant for variant {:?}", variant);
                        if let Some(dest) = dests.first() {
                            let struct_id = QualifiedInstId {
                                module_id: *mid,
                                id: *sid,
                                inst: targs.clone(),
                            };
                            state.set_variant(struct_id, *dest, *variant);
                            eprintln!("DEBUG: Set variant {:?} for struct {:?} at temp {}", variant, sid, dest);
                        }
                    },

                    // TestVariant tests if a struct is a specific variant
                    TestVariant(_mid, _sid, variant, _targs) => {
                        eprintln!("DEBUG: TestVariant for variant {:?}", variant);
                        // We don't set variant info here, but we could track the test result
                        // for optimization purposes (not implemented here)
                    },

                    // BorrowVariantField extracts a field from a specific variant
                    BorrowVariantField(mid, sid, variants, targs, field_idx) => {
                        eprintln!("DEBUG: BorrowVariantField for variants {:?}, field {}", variants, field_idx);
                        if let Some(variant) = variants.first() {
                            if let Some(src) = srcs.first() {
                                let struct_id = QualifiedInstId {
                                    module_id: *mid,
                                    id: *sid,
                                    inst: targs.clone(),
                                };
                                // The source temp now has a known variant
                                state.set_variant(struct_id, *src, *variant);
                                eprintln!("DEBUG: Set variant {:?} for struct {:?} at temp {} (from borrow)", variant, sid, src);
                            }
                        }
                    },

                    // UnpackVariant extracts fields from a known variant
                    UnpackVariant(mid, sid, variant, targs) => {
                        eprintln!("DEBUG: UnpackVariant for variant {:?}", variant);
                        if let Some(src) = srcs.first() {
                            let struct_id = QualifiedInstId {
                                module_id: *mid,
                                id: *sid,
                                inst: targs.clone(),
                            };
                            // The source temp now has a known variant
                            state.set_variant(struct_id, *src, *variant);
                            eprintln!("DEBUG: Set variant {:?} for struct {:?} at temp {} (from unpack)", variant, sid, src);
                        }
                    },

                    // Other operations might kill variant information
                    _ => {
                        // For any operation that might modify the struct, kill variant info
                        for dest in dests {
                            state.kill_variant(*dest);
                        }
                    }
                }
            },

            // Other instructions don't affect variant context
            _ => {}
        }
    }
}

impl DataflowAnalysis for VariantContextAnalysis {}

/// Processor that runs variant context analysis
pub struct VariantContextAnalysisProcessor {}

impl VariantContextAnalysisProcessor {
    pub fn new() -> Box<Self> {
        Box::new(Self {})
    }
}

impl FunctionTargetProcessor for VariantContextAnalysisProcessor {
    fn process(
        &self,
        _targets: &mut FunctionTargetsHolder,
        func_env: &FunctionEnv,
        mut data: FunctionData,
        _scc_opt: Option<&[FunctionEnv]>,
    ) -> FunctionData {
        if func_env.is_native() || func_env.is_intrinsic() {
            return data;
        }

        // Defensive check: if the bytecode is empty, skip analysis
        if data.code.is_empty() {
            return data;
        }

        // Try to create the control flow graph, but handle errors gracefully
        let cfg = match std::panic::catch_unwind(|| {
            StacklessControlFlowGraph::new_forward(&data.code)
        }) {
            Ok(cfg) => cfg,
            Err(_) => {
                // If CFG creation fails, skip variant context analysis
                eprintln!("WARNING: Failed to create control flow graph for function {}, skipping variant context analysis",
                         func_env.get_full_name_str());
                return data;
            }
        };

        let func_target = FunctionTarget::new(func_env, &data);
        let analyzer = VariantContextAnalysis::new(&func_target);

        // Only proceed with analysis if we have a valid CFG
        if cfg.num_blocks() > 0 {
            let state_map = match std::panic::catch_unwind(|| {
                analyzer.analyze_function(
                    VariantContext::default(),
                    &data.code,
                    &cfg,
                )
            }) {
                Ok(state_map) => state_map,
                Err(_) => {
                    eprintln!("WARNING: Failed to analyze variant context for function {}, skipping",
                             func_env.get_full_name_str());
                    return data;
                }
            };

            let per_bytecode_state = match std::panic::catch_unwind(|| {
                analyzer.state_per_instruction(
                    state_map,
                    &data.code,
                    &cfg,
                    |before, _| before.clone(),
                )
            }) {
                Ok(per_bytecode_state) => per_bytecode_state,
                Err(_) => {
                    eprintln!("WARNING: Failed to compute per-bytecode state for function {}, skipping",
                             func_env.get_full_name_str());
                    return data;
                }
            };

            let annotation = VariantContextAnnotation(per_bytecode_state);
            data.annotations.set(annotation, true);
        }

        data
    }

    fn name(&self) -> String {
        "variant_context_analysis".to_string()
    }
}
