// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module implements a stackless-bytecode check that warns about address parameters not compared to
//! the zero address.
//! Using an address parameter without checking not being the zero address could lead to loss of funds,
//! loss of ownership and other potential issues.
//! This check has false positives, for example:
//! ```move
//!     public fun example_unchecked_param(user: address) {
//!        if (user == @0x1) {
//!            do_something()
//!        };
//!    }
//! ```
//! Will warn, even when the address is known to not be @0x0

use move_compiler_v2::external_checks::StacklessBytecodeChecker;
use move_model::{
    ast::Address,
    model::Parameter,
    ty::{PrimitiveType, Type},
};
use move_stackless_bytecode::{
    function_target::FunctionTarget,
    stackless_bytecode::{Bytecode, Constant, Operation},
};
use std::collections::{BTreeMap, HashSet};

#[derive(Debug, Clone, PartialEq, Copy)]
enum SanitizationState {
    Unsanitized,
    Sanitized,
    Derived,
}

struct CheckerState {
    sanitization_state: BTreeMap<usize, SanitizationState>,
    var_to_param: BTreeMap<usize, usize>,
    reported_params: HashSet<usize>,
}

impl CheckerState {
    fn new(address_params: &BTreeMap<usize, &Parameter>) -> Self {
        let sanitization_state = address_params
            .keys()
            .map(|&idx| (idx, SanitizationState::Unsanitized))
            .collect();
        let var_to_param = address_params.keys().map(|&idx| (idx, idx)).collect();

        Self {
            sanitization_state,
            var_to_param,
            reported_params: HashSet::new(),
        }
    }
}

#[derive(Default)]
pub struct ZeroAddress {}

impl StacklessBytecodeChecker for ZeroAddress {
    fn get_name(&self) -> String {
        "zero_address".to_string()
    }

    fn check(&self, target: &FunctionTarget) {
        if !target.is_exposed() {
            return;
        }

        let fn_params = target.func_env.get_parameters();
        let address_params: BTreeMap<usize, &Parameter> = (0..target.get_parameter_count())
            .zip(fn_params.iter())
            .filter(|(idx, _param)| {
                matches!(
                    target.get_local_type(*idx),
                    Type::Primitive(PrimitiveType::Address)
                )
            })
            .collect();

        if address_params.is_empty() {
            return;
        }

        let mut state = CheckerState::new(&address_params);

        self.build_sanitization_state(target, &mut state);
        self.check_usage_violations(target, &mut state, &address_params);
    }
}

impl ZeroAddress {
    fn build_sanitization_state(&self, target: &FunctionTarget, state: &mut CheckerState) {
        let code = target.get_bytecode();

        for instr in code.iter() {
            self.process_zero_checks(instr, code, state);
            self.update_variable_tracking(instr, state);
        }
    }

    fn process_zero_checks(&self, instr: &Bytecode, code: &[Bytecode], state: &mut CheckerState) {
        if let Bytecode::Call(_, _, Operation::Eq | Operation::Neq, args, _) = instr {
            if let Some(checked_var) = self.is_zero_address_check(code, args) {
                if let Some(&original_param) = state.var_to_param.get(&checked_var) {
                    state
                        .sanitization_state
                        .insert(original_param, SanitizationState::Sanitized);
                }
            }
        }
    }

    fn update_variable_tracking(&self, instr: &Bytecode, state: &mut CheckerState) {
        match instr {
            Bytecode::Assign(_, dest, src, _) => {
                self.handle_assign(*dest, *src, state);
            },
            _ => {
                self.handle_general_instruction(instr, state);
            },
        }
    }

    fn handle_assign(&self, dest: usize, src: usize, state: &mut CheckerState) {
        if let Some(&original_param) = state.var_to_param.get(&src) {
            state.var_to_param.insert(dest, original_param);
            if let Some(&param_state) = state.sanitization_state.get(&original_param) {
                let new_state = match param_state {
                    SanitizationState::Unsanitized => SanitizationState::Derived,
                    x => x,
                };
                state.sanitization_state.insert(dest, new_state);
            }
        }
    }

    fn handle_general_instruction(&self, instr: &Bytecode, state: &mut CheckerState) {
        for dest in instr.dests() {
            for src in instr.sources() {
                if let Some(&original_param) = state.var_to_param.get(&src) {
                    state.var_to_param.insert(dest, original_param);
                    if let Some(&param_state) = state.sanitization_state.get(&original_param) {
                        let new_state = match param_state {
                            SanitizationState::Unsanitized => SanitizationState::Derived,
                            x => x,
                        };
                        state.sanitization_state.insert(dest, new_state);
                    }
                    break;
                }
            }
        }
    }

    fn check_usage_violations(
        &self,
        target: &FunctionTarget,
        state: &mut CheckerState,
        address_params: &BTreeMap<usize, &Parameter>,
    ) {
        let code = target.get_bytecode();
        let fn_params = target.func_env.get_parameters();

        for instr in code.iter() {
            self.check_instruction_usage(instr, state, &fn_params, target, address_params);

            if instr.is_exit() {
                self.check_exit_usage(instr, state, &fn_params, target, address_params);
            }
        }
    }

    fn check_instruction_usage(
        &self,
        instr: &Bytecode,
        state: &mut CheckerState,
        fn_params: &[Parameter],
        target: &FunctionTarget,
        address_params: &BTreeMap<usize, &Parameter>,
    ) {
        let should_check = match instr {
            Bytecode::Call(_, _, Operation::Eq | Operation::Neq, args, _) => self
                .is_zero_address_check(target.get_bytecode(), args)
                .is_none(),
            Bytecode::Call(..) | Bytecode::Assign(..) => true,
            _ => false,
        };

        if should_check {
            self.check_unsanitized_usage(
                instr,
                &state.var_to_param,
                &state.sanitization_state,
                &mut state.reported_params,
                fn_params,
                target,
                address_params,
            );
        }
    }

    fn check_exit_usage(
        &self,
        instr: &Bytecode,
        state: &mut CheckerState,
        fn_params: &[Parameter],
        target: &FunctionTarget,
        address_params: &BTreeMap<usize, &Parameter>,
    ) {
        for src in instr.sources() {
            let original_param = state
                .var_to_param
                .get(&src)
                .copied()
                .or_else(|| self.find_original_param(&state.var_to_param, src, address_params));

            if original_param.is_none() {
                return;
            }
            let param_idx = original_param.unwrap();
            if state.reported_params.contains(&param_idx) {
                return;
            }
            if let Some(&SanitizationState::Unsanitized) = state.sanitization_state.get(&param_idx)
            {
                let param_name = fn_params[param_idx]
                    .0
                    .display(target.func_env.symbol_pool());
                self.report(
                    target.global_env(),
                    &fn_params[param_idx].2,
                    &format!(
                        "Address parameter '{}' used without zero-address validation",
                        param_name
                    ),
                );
                state.reported_params.insert(param_idx);
            }
        }
    }

    fn check_unsanitized_usage(
        &self,
        instr: &Bytecode,
        var_to_param: &BTreeMap<usize, usize>,
        sanitization_state: &BTreeMap<usize, SanitizationState>,
        reported_params: &mut HashSet<usize>,
        fn_params: &[Parameter],
        target: &FunctionTarget,
        address_params: &BTreeMap<usize, &Parameter>,
    ) {
        for src in instr.sources() {
            let original_param = var_to_param
                .get(&src)
                .copied()
                .or_else(|| self.find_original_param(var_to_param, src, address_params));

            if let Some(param_idx) = original_param {
                if !reported_params.contains(&param_idx) {
                    if let Some(&SanitizationState::Unsanitized) =
                        sanitization_state.get(&param_idx)
                    {
                        let param_name = fn_params[param_idx]
                            .0
                            .display(target.func_env.symbol_pool());
                        self.report(
                            target.global_env(),
                            &fn_params[param_idx].2,
                            &format!(
                                "Address parameter '{}' used without zero-address validation",
                                param_name
                            ),
                        );
                        reported_params.insert(param_idx);
                    }
                }
            }
        }
    }

    fn is_zero_address_check(&self, code: &[Bytecode], args: &[usize]) -> Option<usize> {
        if args.len() != 2 {
            return None;
        }

        // Look for the instruction that loaded the zero constant
        for instr in code {
            match instr {
                Bytecode::Load(_, dest, Constant::Address(Address::Numerical(addr))) => {
                    if **addr == [0u8; 32] && args.contains(dest) {
                        return args.iter().find(|&&x| x != *dest).copied();
                    }
                },
                _ => continue,
            }
        }

        None
    }

    fn find_original_param(
        &self,
        var_to_param: &BTreeMap<usize, usize>,
        var: usize,
        address_params: &BTreeMap<usize, &Parameter>,
    ) -> Option<usize> {
        if let Some(&param_idx) = var_to_param.get(&var) {
            if address_params.contains_key(&param_idx) {
                return Some(param_idx);
            }
        }
        None
    }
}
