// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module implements a stackless-bytecode linter that checks for unsafe usage of table operations.
//! Specifically, it detects:
//! 1. `table::borrow` calls without checking if the key exists first (should use `table::contains`)
//! 2. `table::add` calls that might fail if the key already exists (should use `table::upsert` or check with `table::contains`)
//!
//! This helps prevent runtime errors when working with tables in Move code.

use std::collections::HashMap;

use crate::assignment_tracker::AssignmentTracker;
use move_compiler_v2::external_checks::StacklessBytecodeChecker;
use move_model::ast::TempIndex;

use move_model::model::{FunId, ModuleId, Visibility};
use move_stackless_bytecode::{
    function_target::FunctionTarget,
    stackless_bytecode::{Bytecode, Label, Operation},
};

/// Represents what we know about table state in a specific branch
#[derive(Debug, Clone)]
struct BranchKnowledge {
    /// Table/key pairs we know exist (if condition was contains call)
    known_existing: Vec<(TempIndex, TempIndex)>, // (table_temp, key_temp)
    /// Table/key pairs we know don't exist
    known_not_existing: Vec<(TempIndex, TempIndex)>,
}

/// Analysis state for tracking control flow and table operations
struct AnalysisState {
    /// Map from temp variables to contains calls: result_temp -> (table_temp, key_temp)
    contains_calls: HashMap<TempIndex, (TempIndex, TempIndex)>,
    /// Map from temp variables to negated contains calls: negated_result_temp -> original_contains_temp
    negated_contains: HashMap<TempIndex, TempIndex>,
    /// Current branch knowledge stack
    branch_stack: Vec<BranchKnowledge>,
    /// Map from label to branch knowledge when entering that label
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
        // Only analyze public functions
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

        // First pass: build assignment tracking
        for bytecode in code.iter() {
            assignment_tracker.process_bytecode(bytecode);
        }

        // Second pass: analyze control flow and table operations
        for bytecode in code.iter() {
            match bytecode {
                Bytecode::Label(_, label) => {
                    if let Some(knowledge) = state.label_knowledge.get(label) {
                        state.branch_stack = knowledge.clone();
                    }
                },
                Bytecode::Branch(_, then_label, else_label, condition_temp) => {
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

                            // Track contains calls
                            if self.is_table_function(&function_name, "contains") {
                                if let (Some(dest), Some(table_arg), Some(key_arg)) = (
                                    dests.first(),
                                    Self::get_table_instance(srcs),
                                    Self::get_key_argument(srcs),
                                ) {
                                    state.contains_calls.insert(*dest, (table_arg, key_arg));
                                }
                            }

                            // Check borrow calls
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

                            // Check add calls
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
    /// Handle branch instructions and set up branch knowledge
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

    /// Check if a borrow operation is safe
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

    /// Check if an add operation is safe (key should not exist)
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
