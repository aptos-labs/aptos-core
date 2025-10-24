// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0
//! Zero-Address Detector
//!
//! Warns when exposed functions use address-typed parameters without first proving
//! they are not the zero address. Relies on the temp equivalence analyzer to follow
//! aliases and on a lightweight dataflow to propagate non-zero knowledge across
//! branches.

mod cfg_utils;
mod dataflow;

use self::{
    cfg_utils::{build_label_to_block_map, collect_label_offsets},
    dataflow::{analyze_zero_function, ZeroTransfer},
};
use crate::temp_equivalence_analyzer::{TempEquivalenceAnalyzer, TempEquivalenceState};
use move_binary_format::file_format::CodeOffset;
use move_compiler_v2::external_checks::StacklessBytecodeChecker;
use move_model::{
    ast::{Attribute, TempIndex},
    model::{FunId, ModuleId},
    ty::{PrimitiveType, Type},
};
use move_stackless_bytecode::{
    dataflow_analysis::DataflowAnalysis,
    function_target::FunctionTarget,
    stackless_bytecode::{Bytecode, Operation},
    stackless_control_flow_graph::StacklessControlFlowGraph,
};
use num::ToPrimitive;
use std::collections::{BTreeMap, BTreeSet, HashMap};

const DANGEROUS_PARAM_NAMES: &[&str] = &[
    "recipient",
    "to",
    "destination",
    "dest",
    "owner",
    "admin",
    "authority",
    "signer",
    "delegate",
    "operator",
    "controller",
    "beneficiary",
    "payee",
];

const DANGEROUS_FUNCTION_NAMES: &[&str] = &[
    "transfer",
    "send",
    "deposit",
    "withdraw",
    "grant",
    "authorize",
    "approve",
    "delegate",
    "create_account",
    "register",
    "set_owner",
    "set_admin",
    "add_admin",
    "mint",
    "burn",
    "issue",
];

const VIEW_FUNCTION_ATTRIBUTE: &str = "view";

#[derive(Default)]
pub struct ZeroAddress;

impl StacklessBytecodeChecker for ZeroAddress {
    fn get_name(&self) -> String {
        "zero_address".to_string()
    }

    fn check(&self, target: &FunctionTarget) {
        if target.func_env.is_test_only() {
            return;
        }

        if !target.func_env.is_entry() {
            return;
        }

        if has_function_attribute(target, VIEW_FUNCTION_ATTRIBUTE) {
            return;
        }

        let address_params = collect_address_parameters_with_names(target);
        if address_params.is_empty() {
            return;
        }

        let code = target.get_bytecode();
        let cfg = StacklessControlFlowGraph::new_forward(code);

        let analyzer = TempEquivalenceAnalyzer;
        let equiv_states = analyzer.state_at_each_instruction(code, &cfg);

        let comparisons = collect_zero_comparisons(code, &equiv_states);
        let negated_map = collect_negated_temps(code);
        let label_offsets = collect_label_offsets(code);
        let label_to_block = build_label_to_block_map(&label_offsets, &cfg);

        let param_indices: Vec<TempIndex> =
            address_params.iter().map(|(idx, _name)| *idx).collect();
        let transfer = ZeroTransfer::new(&param_indices, &comparisons, &negated_map, &equiv_states);

        let state_map =
            analyze_zero_function(&transfer, code, &cfg, &label_to_block, &equiv_states);

        let per_instruction =
            transfer.state_per_instruction_with_default(state_map, code, &cfg, |pre, _post| {
                pre.clone()
            });

        let mut reported: BTreeSet<TempIndex> = BTreeSet::new();

        for (offset, instr) in code.iter().enumerate() {
            let Some(code_offset) = offset.to_u16() else {
                continue;
            };

            let Bytecode::Call(attr_id, _, Operation::Function(module_id, function_id, _), srcs, _) =
                instr
            else {
                continue;
            };

            let function_name = get_function_name(target, *module_id, *function_id);

            let Some((alias_state, state)) = equiv_states
                .get(&code_offset)
                .and_then(|a| per_instruction.get(&code_offset).map(|s| (a, s)))
            else {
                continue;
            };

            // Check each address parameter
            for (param_temp, param_name) in &address_params {
                if reported.contains(param_temp) {
                    continue;
                }

                // Check if param flows into this call
                if !srcs
                    .iter()
                    .any(|src| alias_state.are_equivalent(*src, *param_temp))
                {
                    continue;
                }

                // Check if already proven non-zero
                if state.is_non_zero(*param_temp) {
                    continue;
                }

                // Heuristic: check if this looks dangerous
                if is_dangerous_use(param_name, &function_name) {
                    let loc = target.get_bytecode_loc(*attr_id);
                    self.report(
                        target.global_env(),
                        &loc,
                        &format!(
                            "{} may be zero address when calling {}",
                            param_name, function_name
                        ),
                    );
                    reported.insert(*param_temp);
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
struct ZeroComparison {
    address: TempIndex,
    op: ZeroComparisonOp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ZeroComparisonOp {
    EqZero,
    NeqZero,
}

fn collect_address_parameters_with_names(target: &FunctionTarget) -> Vec<(TempIndex, String)> {
    target
        .get_parameters()
        .filter(|&idx| is_address_type(target.get_local_type(idx)))
        .map(|idx| {
            let name = target.get_local_name_for_error_message(idx);
            (idx, name)
        })
        .collect()
}

fn is_address_type(ty: &Type) -> bool {
    match ty {
        Type::Primitive(PrimitiveType::Address) => true,
        Type::Reference(_, inner) => is_address_type(inner),
        _ => false,
    }
}

fn collect_zero_comparisons(
    code: &[Bytecode],
    equiv_states: &BTreeMap<CodeOffset, TempEquivalenceState>,
) -> BTreeMap<TempIndex, ZeroComparison> {
    let mut map = BTreeMap::new();

    for (offset, instr) in code.iter().enumerate() {
        let Some(code_offset) = offset.to_u16() else {
            continue;
        };

        let Bytecode::Call(_, dests, operation, srcs, _) = instr else {
            continue;
        };

        if dests.is_empty() || srcs.len() < 2 {
            continue;
        };

        let op = match operation {
            Operation::Eq => ZeroComparisonOp::EqZero,
            Operation::Neq => ZeroComparisonOp::NeqZero,
            _ => continue,
        };

        let Some(alias_state) = equiv_states.get(&code_offset) else {
            continue;
        };

        let lhs_zero = is_zero_address(alias_state, srcs[0]);
        let rhs_zero = is_zero_address(alias_state, srcs[1]);
        let lhs_non_zero = is_non_zero_address_constant(alias_state, srcs[0]);
        let rhs_non_zero = is_non_zero_address_constant(alias_state, srcs[1]);

        let (address_temp, effective_op) = match (lhs_zero, rhs_zero, lhs_non_zero, rhs_non_zero) {
            // Explicit @0x0 comparison
            (true, false, _, _) => (srcs[1], op),
            (false, true, _, _) => (srcs[0], op),

            // Non-zero constant comparison
            (false, false, true, false) => match op {
                ZeroComparisonOp::EqZero => (srcs[1], ZeroComparisonOp::NeqZero),
                ZeroComparisonOp::NeqZero => (srcs[1], ZeroComparisonOp::EqZero),
            },
            (false, false, false, true) => match op {
                ZeroComparisonOp::EqZero => (srcs[0], ZeroComparisonOp::NeqZero),
                ZeroComparisonOp::NeqZero => (srcs[0], ZeroComparisonOp::EqZero),
            },

            _ => continue,
        };

        let dest = dests[0];
        map.insert(dest, ZeroComparison {
            address: address_temp,
            op: effective_op,
        });
    }

    map
}

fn is_non_zero_address_constant(alias_state: &TempEquivalenceState, temp: TempIndex) -> bool {
    alias_state.equivalence_class(temp).iter().any(|member| {
        alias_state
            .constant_for(*member)
            .is_some_and(|value| !value.is_zero_address())
    })
}

fn is_zero_address(alias_state: &TempEquivalenceState, temp: TempIndex) -> bool {
    alias_state.equivalence_class(temp).iter().any(|member| {
        alias_state
            .constant_for(*member)
            .is_some_and(|value| value.is_zero_address())
    })
}

fn collect_negated_temps(code: &[Bytecode]) -> HashMap<TempIndex, TempIndex> {
    code.iter()
        .filter_map(|instr| match instr {
            Bytecode::Call(_, dests, Operation::Not, srcs, _) => {
                Some((*dests.first()?, *srcs.first()?))
            },
            _ => None,
        })
        .collect()
}

fn is_dangerous_use(param_name: &str, function_name: &str) -> bool {
    let param_lower = param_name.to_lowercase();
    let func_lower = function_name.to_lowercase();

    DANGEROUS_PARAM_NAMES
        .iter()
        .any(|&name| param_lower.contains(name))
        || DANGEROUS_FUNCTION_NAMES
            .iter()
            .any(|&name| func_lower.contains(name))
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

fn has_function_attribute(target: &FunctionTarget, attr_name: &str) -> bool {
    let symbol_pool = target.func_env.symbol_pool();
    target.func_env.get_attributes().iter().any(|attribute| {
        matches!(
            attribute,
            Attribute::Apply(_, name, _) if symbol_pool.string(*name).as_str() == attr_name
        )
    })
}
