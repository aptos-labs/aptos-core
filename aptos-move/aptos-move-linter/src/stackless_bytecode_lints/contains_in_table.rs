// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Contains-in-Table Safety Detector
//!
//! This detector implements a flow-sensitive analysis to prevent runtime failures in table operations
//! by ensuring proper existence checks before accessing table entries. It detects two main violation patterns:
//!
//! 1. **Unsafe borrow**: `table::borrow(t, k)` without prior `table::contains(t, k)` check
//! 2. **Unsafe add**: `table::add(t, k, v)` without ensuring key doesn't exist
//!
//! ## Analysis Architecture
//!
//! The detector uses a three-pass analysis combining value flow tracking with control flow sensitivity:
//!
//! ### Pass 1: Value Equivalence Analysis
//! Builds assignment chains and reference relationships using `AssignmentTracker` to determine
//! when different temporary variables refer to the same logical table/key values. This enables
//! matching table operations across complex assignment patterns and field accesses.
//!
//! ### Pass 2: Control Flow & State Propagation
//! Performs flow-sensitive analysis of branches conditioned on `table::contains()` calls.
//! Tracks positive/negative table existence knowledge through conditional branches, handling
//! both direct conditions (`if (contains(t,k))`) and negated ones (`if (!contains(t,k))`).
//! Knowledge is propagated via a branch stack that accumulates constraints along control paths.
//!
//! ### Pass 3: Safety Validation
//! Validates each table operation against accumulated branch knowledge, using value equivalence
//! to match operations with prior existence checks. Reports violations when operations cannot
//! be proven safe based on the current control flow context.
//!
//! ## Key Limitations
//! - **Intra-procedural only**: No cross-function analysis
//! - **Public functions only**: Private functions assumed to have validated inputs
//! - **Single-pass CFG traversal**: May miss complex control flow patterns

use std::collections::HashMap;

use crate::assignment_tracker::AssignmentTracker;
use move_compiler_v2::external_checks::StacklessBytecodeChecker;
use move_model::ast::TempIndex;

use move_model::model::{FunId, ModuleId, Visibility};
use move_stackless_bytecode::{
    function_target::FunctionTarget,
    stackless_bytecode::{Bytecode, Label, Operation},
};

/// Branch-sensitive table existence knowledge
///
/// Represents what we know about table key existence within a specific control flow branch.
/// This knowledge is derived from `table::contains()` call conditions and accumulated as
/// we traverse through conditional branches.
#[derive(Debug, Clone)]
struct BranchKnowledge {
    /// Table/key pairs we know exist in this branch (from positive contains checks)
    known_existing: Vec<(TempIndex, TempIndex)>, // (table_temp, key_temp)
    /// Table/key pairs we know don't exist in this branch (from negative contains checks)
    known_not_existing: Vec<(TempIndex, TempIndex)>,
}

/// Global analysis state tracking control flow and table operations
///
/// Maintains the state needed for flow-sensitive analysis of table operations across
/// control flow boundaries. Combines tracking of contains call results with branch-sensitive
/// knowledge propagation.
struct AnalysisState {
    /// Maps contains call results to their arguments: result_temp -> (table_temp, key_temp)
    /// Used to identify when branch conditions are based on table existence checks
    contains_calls: HashMap<TempIndex, (TempIndex, TempIndex)>,
    /// Maps negated contains results to original contains temp: !result_temp -> result_temp
    /// Handles negated conditions like `if (!table::contains(t, k))`
    negated_contains: HashMap<TempIndex, TempIndex>,
    /// Stack of branch knowledge accumulated along current control path
    /// Each entry represents constraints learned from a conditional branch
    branch_stack: Vec<BranchKnowledge>,
    /// Maps control flow labels to branch knowledge when entering that label
    /// Enables restoration of appropriate knowledge state when jumping to labels
    label_knowledge: HashMap<Label, Vec<BranchKnowledge>>,
}

#[derive(Default)]
pub struct ContainsInTable {}

impl ContainsInTable {
    fn get_table_instance(srcs: &[TempIndex]) -> Option<TempIndex> {
        srcs.first().copied()
    }

    fn get_key_argument(srcs: &[TempIndex]) -> Option<TempIndex> {
        if srcs.len() >= 2 {
            srcs.get(1).copied()
        } else {
            None
        }
    }
}

impl StacklessBytecodeChecker for ContainsInTable {
    fn get_name(&self) -> String {
        "contains_in_table".to_string()
    }

    fn check(&self, target: &FunctionTarget) {
        // Only analyze public functions - private functions assumed to have validated inputs
        if target.func_env.visibility() != Visibility::Public {
            return;
        }

        let code = target.get_bytecode();
        let mut assignment_tracker = AssignmentTracker::new();
        let mut state = AnalysisState {
            contains_calls: HashMap::new(),
            negated_contains: HashMap::new(),
            branch_stack: Vec::new(),
            label_knowledge: HashMap::new(),
        };

        // === PASS 1: Value Equivalence Analysis ===
        // Build assignment tracking to determine when temporaries hold equivalent values.
        // This enables matching table operations with their corresponding contains checks.
        for bytecode in code.iter() {
            assignment_tracker.process_bytecode(bytecode);
        }

        // === PASS 2: Control Flow & State Propagation ===
        // Flow-sensitive analysis tracking table existence knowledge through branches.
        // Identifies contains-based conditions and propagates positive/negative existence
        // knowledge to appropriate branch targets. Handles both normal and negated conditions.
        for bytecode in code.iter() {
            match bytecode {
                Bytecode::Label(_, label) => {
                    // Restore branch knowledge when entering a labeled block
                    if let Some(knowledge) = state.label_knowledge.get(label) {
                        state.branch_stack = knowledge.clone();
                    }
                },
                Bytecode::Branch(_, then_label, else_label, condition_temp) => {
                    // Propagate table existence knowledge based on contains call conditions
                    self.handle_branch(&mut state, *then_label, *else_label, *condition_temp);
                },
                Bytecode::Call(attr_id, dests, operation, srcs, _) => {
                    match operation {
                        // Handle negation operations
                        Operation::Not => {
                            if let (Some(dest), Some(src)) = (dests.first(), srcs.first()) {
                                // Check if we're negating a contains call result
                                if state.contains_calls.contains_key(src) {
                                    state.negated_contains.insert(*dest, *src);
                                }
                            }
                        },

                        // Handle function calls
                        Operation::Function(module_id, function_id, _type_args) => {
                            let function_name =
                                self.get_function_name(target, *module_id, *function_id);
                            let loc = target.get_bytecode_loc(*attr_id);

                            // Track contains calls for branch analysis
                            if self.is_table_function(&function_name, "contains") {
                                if let (Some(dest), Some(table_arg), Some(key_arg)) = (
                                    dests.first(),
                                    Self::get_table_instance(srcs),
                                    Self::get_key_argument(srcs),
                                ) {
                                    state.contains_calls.insert(*dest, (table_arg, key_arg));
                                }
                            }

                            // Validate table operations against accumulated branch knowledge
                            if self.is_table_function(&function_name, "borrow") {
                                if let (Some(table_arg), Some(key_arg)) =
                                    (Self::get_table_instance(srcs), Self::get_key_argument(srcs))
                                {
                                    if !self.is_safe_borrow(
                                        &state,
                                        &assignment_tracker,
                                        table_arg,
                                        key_arg,
                                    ) {
                                        let msg = "table::borrow called without checking if key exists. Consider using table::contains first to avoid runtime errors.";
                                        self.report(target.global_env(), &loc, msg);
                                    }
                                }
                            }

                            if self.is_table_function(&function_name, "add") {
                                if let (Some(table_arg), Some(key_arg)) =
                                    (Self::get_table_instance(srcs), Self::get_key_argument(srcs))
                                {
                                    if !self.is_safe_add(
                                        &state,
                                        &assignment_tracker,
                                        target,
                                        table_arg,
                                        key_arg,
                                    ) {
                                        let msg = "table::add called without ensuring key doesn't exist. This may fail if key already exists. Consider using table::upsert or check with table::contains first.";
                                        self.report(target.global_env(), &loc, msg);
                                    }
                                }
                            }
                        },
                        _ => {},
                    }
                },
                _ => {},
            }
        }
    }
}

impl ContainsInTable {
    /// Propagate table existence knowledge through conditional branches
    ///
    /// When branching on table::contains results, this creates appropriate BranchKnowledge
    /// for each target label, encoding positive/negative existence information based on
    /// the branch condition. Handles both direct and negated contains conditions.
    fn handle_branch(
        &self,
        state: &mut AnalysisState,
        then_label: Label,
        else_label: Label,
        condition_temp: TempIndex,
    ) {
        // Check if the condition is based on a contains call or negated contains call
        if let Some((table_temp, key_temp)) = state.contains_calls.get(&condition_temp) {
            self.setup_branch_knowledge_for_contains(
                state,
                then_label,
                else_label,
                *table_temp,
                *key_temp,
                false,
            );
        } else if let Some(original_contains_temp) = state.negated_contains.get(&condition_temp) {
            // This is a negated contains call - flip the logic
            if let Some((table_temp, key_temp)) = state.contains_calls.get(original_contains_temp) {
                self.setup_branch_knowledge_for_contains(
                    state,
                    then_label,
                    else_label,
                    *table_temp,
                    *key_temp,
                    true,
                );
            }
        }
    }

    /// Set up branch knowledge for a contains call (normal or negated)
    ///
    /// Creates BranchKnowledge instances for true/false branches based on contains semantics.
    /// For normal contains: true branch knows key exists, false branch knows key doesn't exist.
    /// For negated contains: logic is flipped.
    fn setup_branch_knowledge_for_contains(
        &self,
        state: &mut AnalysisState,
        then_label: Label,
        else_label: Label,
        table_temp: TempIndex,
        key_temp: TempIndex,
        is_negated: bool,
    ) {
        // For normal contains: true branch = key exists, false branch = key doesn't exist
        // For negated contains: true branch = key doesn't exist, false branch = key exists
        let (true_existing, true_not_existing, false_existing, false_not_existing) = if is_negated {
            // Negated: true means key doesn't exist, false means key exists
            (
                vec![],
                vec![(table_temp, key_temp)],
                vec![(table_temp, key_temp)],
                vec![],
            )
        } else {
            // Normal: true means key exists, false means key doesn't exist
            (
                vec![(table_temp, key_temp)],
                vec![],
                vec![],
                vec![(table_temp, key_temp)],
            )
        };

        // Create branch knowledge for true branch
        let true_branch = BranchKnowledge {
            known_existing: true_existing,
            known_not_existing: true_not_existing,
        };

        // Create branch knowledge for false branch
        let false_branch = BranchKnowledge {
            known_existing: false_existing,
            known_not_existing: false_not_existing,
        };

        // Set up knowledge for each label
        let mut then_knowledge = state.branch_stack.clone();
        then_knowledge.push(true_branch);
        state.label_knowledge.insert(then_label, then_knowledge);

        let mut else_knowledge = state.branch_stack.clone();
        else_knowledge.push(false_branch);
        state.label_knowledge.insert(else_label, else_knowledge);
    }

    /// Validate that a borrow operation is safe in the current control flow context
    ///
    /// Searches accumulated branch knowledge for positive existence evidence matching
    /// the borrow arguments (using value equivalence). Returns true if any branch
    /// context guarantees the key exists in the table.
    fn is_safe_borrow(
        &self,
        state: &AnalysisState,
        assignment_tracker: &AssignmentTracker,
        table_temp: TempIndex,
        key_temp: TempIndex,
    ) -> bool {
        for branch in state.branch_stack.iter() {
            for (known_table, known_key) in branch.known_existing.iter() {
                let table_equiv = assignment_tracker.are_equivalent(*known_table, table_temp);
                let key_equiv = assignment_tracker.are_equivalent(*known_key, key_temp);

                if table_equiv && key_equiv {
                    return true;
                }
            }
        }
        false
    }

    /// Validate that an add operation is safe in the current control flow context
    ///
    /// Searches accumulated branch knowledge for negative existence evidence matching
    /// the add arguments. Returns true if any branch context guarantees the key
    /// does not exist in the table, making the add operation safe.
    fn is_safe_add(
        &self,
        state: &AnalysisState,
        assignment_tracker: &AssignmentTracker,
        _target: &FunctionTarget,
        table_temp: TempIndex,
        key_temp: TempIndex,
    ) -> bool {
        for branch in state.branch_stack.iter() {
            for (known_table, known_key) in branch.known_not_existing.iter() {
                let table_equiv = assignment_tracker.are_equivalent(*known_table, table_temp);
                let key_equiv = assignment_tracker.are_equivalent(*known_key, key_temp);

                if table_equiv && key_equiv {
                    return true;
                }
            }
        }
        false
    }

    /// Get the full function name including module path
    fn get_function_name(
        &self,
        target: &FunctionTarget,
        module_id: ModuleId,
        function_id: FunId,
    ) -> String {
        let global_env = target.global_env();
        let module_env = global_env.get_module(module_id);
        let function_env = module_env.get_function(function_id);
        format!(
            "{}::{}",
            module_env.get_full_name_str(),
            function_env.get_name_str()
        )
    }

    /// Check if the function is a table function with the given operation
    fn is_table_function(&self, full_name: &str, operation: &str) -> bool {
        let patterns = [
            format!("table::{}", operation),
            format!("aptos_std::table::{}", operation),
            format!("std::table::{}", operation),
        ];
        patterns.iter().any(|pattern| full_name.ends_with(pattern))
    }
}
