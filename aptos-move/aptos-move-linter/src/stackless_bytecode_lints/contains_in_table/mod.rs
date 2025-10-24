// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0
//! Contains-in-Table Safety Detector
//!
//! Flow-sensitive lint that ensures table operations are guarded by the
//! appropriate `table::contains` check. Warns when
//!   * `table::borrow` / `table::borrow_mut` happen without proving the key exists
//!   * `table::add` happens without proving the key is absent

mod cfg_utils;
mod dataflow;

use move_compiler_v2::external_checks::StacklessBytecodeChecker;
use move_linter::temp_equivalence_analyzer::{TempEquivalenceAnalyzer, TempEquivalenceState};
use move_model::{
    ast::{Attribute, TempIndex},
    model::{FunId, ModuleId, Visibility},
};
use move_stackless_bytecode::{
    dataflow_analysis::DataflowAnalysis,
    function_target::FunctionTarget,
    stackless_bytecode::{Bytecode, Operation},
    stackless_control_flow_graph::StacklessControlFlowGraph,
};
use num::ToPrimitive;
use std::collections::{BTreeMap, BTreeSet, HashMap};

use self::{
    cfg_utils::{build_label_to_block_map, collect_label_offsets},
    dataflow::{analyze_contains_function, ContainsTransfer},
};

const VIEW_FUNCTION_ATTRIBUTE: &str = "view";

#[derive(Default)]
pub struct ContainsInTable {}

impl StacklessBytecodeChecker for ContainsInTable {
    fn get_name(&self) -> String {
        "contains_in_table".to_string()
    }

    fn check(&self, target: &FunctionTarget) {
        if target.func_env.visibility() != Visibility::Public
            || has_function_attribute(target, VIEW_FUNCTION_ATTRIBUTE)
        {
            return;
        }

        let code = target.get_bytecode();
        let cfg = StacklessControlFlowGraph::new_forward(code);

        let analyzer = TempEquivalenceAnalyzer;
        let equiv_states = analyzer.state_at_each_instruction(code, &cfg);

        let (contains_calls, contains_index) = collect_contains_calls(target, code);
        let negated_map = collect_negated_contains(code);
        let label_offsets = collect_label_offsets(code);
        let label_to_block = build_label_to_block_map(&label_offsets, &cfg);

        let transfer = ContainsTransfer::new(
            target,
            &contains_calls,
            &contains_index,
            &negated_map,
            &equiv_states,
        );

        let state_map =
            analyze_contains_function(&transfer, code, &cfg, &label_to_block, &equiv_states);

        let per_instruction =
            transfer.state_per_instruction_with_default(state_map, code, &cfg, |pre, _post| {
                pre.clone()
            });

        for (offset, instr) in code.iter().enumerate() {
            let Some(code_offset) = offset.to_u16() else {
                continue;
            };

            let Bytecode::Call(attr_id, _, Operation::Function(module_id, function_id, _), srcs, _) =
                instr
            else {
                continue;
            };

            let Some(alias_state) = equiv_states.get(&code_offset) else {
                continue;
            };

            let Some(state) = per_instruction.get(&code_offset) else {
                continue;
            };

            if srcs.len() < 2 {
                continue;
            }

            let function_name = get_function_name(target, *module_id, *function_id);
            let table_temp = srcs[0];
            let key_temp = srcs[1];

            if (is_table_function(&function_name, "borrow")
                || is_table_function(&function_name, "borrow_mut"))
                && !knowledge_matches(
                    &state.known_present,
                    &contains_calls,
                    alias_state,
                    table_temp,
                    key_temp,
                )
            {
                let loc = target.get_bytecode_loc(*attr_id);
                self.report(
                    target.global_env(),
                    &loc,
                    "table::borrow called without checking if key exists. Consider using table::contains first to avoid runtime errors.",
                );
            } else if is_table_function(&function_name, "add")
                && !knowledge_matches(
                    &state.known_absent,
                    &contains_calls,
                    alias_state,
                    table_temp,
                    key_temp,
                )
            {
                let loc = target.get_bytecode_loc(*attr_id);
                self.report(
                    target.global_env(),
                    &loc,
                    "table::add called without ensuring key doesn't exist. This may fail if key already exists. Consider using table::upsert or check with table::contains first.",
                );
            }
        }
    }
}

#[derive(Debug, Clone)]
pub(super) struct ContainsCall {
    result: Option<TempIndex>,
    table: TempIndex,
    key: TempIndex,
}

fn collect_contains_calls(
    target: &FunctionTarget,
    code: &[Bytecode],
) -> (Vec<ContainsCall>, BTreeMap<TempIndex, Vec<usize>>) {
    let mut calls = Vec::new();
    let mut index: BTreeMap<TempIndex, Vec<usize>> = BTreeMap::new();

    for instr in code.iter() {
        let Bytecode::Call(_, dests, Operation::Function(module_id, function_id, _), srcs, _) =
            instr
        else {
            continue;
        };
        if srcs.len() < 2 {
            continue;
        }
        let function_name = get_function_name(target, *module_id, *function_id);
        if !is_table_function(&function_name, "contains") {
            continue;
        }
        let call = ContainsCall {
            result: dests.first().copied(),
            table: srcs[0],
            key: srcs[1],
        };
        if let Some(result) = call.result {
            index.entry(result).or_default().push(calls.len());
        }
        calls.push(call);
    }

    (calls, index)
}

fn collect_negated_contains(code: &[Bytecode]) -> HashMap<TempIndex, TempIndex> {
    let mut map = HashMap::new();
    for instr in code {
        if let Bytecode::Call(_, dests, Operation::Not, srcs, _) = instr {
            if let (Some(dest), Some(src)) = (dests.first(), srcs.first()) {
                map.insert(*dest, *src);
            }
        }
    }
    map
}

fn knowledge_matches(
    knowledge: &BTreeSet<usize>,
    contains_calls: &[ContainsCall],
    alias_state: &TempEquivalenceState,
    table_temp: TempIndex,
    key_temp: TempIndex,
) -> bool {
    knowledge.iter().any(|idx| {
        let call = &contains_calls[*idx];
        alias_state.are_equivalent(call.table, table_temp)
            && alias_state.are_equivalent(call.key, key_temp)
    })
}

fn get_function_name(target: &FunctionTarget, module_id: ModuleId, function_id: FunId) -> String {
    let global_env = target.global_env();
    let module_env = global_env.get_module(module_id);
    let function_env = module_env.get_function(function_id);
    format!(
        "{}::{}",
        module_env.get_full_name_str(),
        function_env.get_name_str()
    )
}

fn is_table_function(full_name: &str, operation: &str) -> bool {
    let patterns = [
        format!("table::{}", operation),
        format!("aptos_std::table::{}", operation),
        format!("std::table::{}", operation),
    ];
    patterns.iter().any(|pattern| full_name.ends_with(pattern))
}

fn has_function_attribute(target: &FunctionTarget, attr_name: &str) -> bool {
    let symbol_pool = target.func_env.symbol_pool();
    target.func_env.get_attributes().iter().any(|attribute| {
        matches!(
            attribute,
            Attribute::Apply(_, name, _) if symbol_pool.string(*name).as_str() == attr_name
        )
    })
}
