// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    attr_derivation::{
        attr_params, find_attr_slice, new_attr, new_native_fun, new_simple_type, new_var,
    },
    parser::ast::{Definition, FunctionName, ModuleDefinition, ModuleMember, Visibility},
    shared::{CompilationEnv, NamedAddressMap},
};
use move_ir_types::location::sp;
use move_symbol_pool::Symbol;
use std::collections::BTreeSet;

const CONTRACT_ATTR: &str = "contract";
const CALLABLE_ATTR: &str = "callable";
const EXTERNAL_ATTR: &str = "external";

// The following appear in test code under /evm/.
const ACTOR_ATTR: &str = "actor";
const INIT_ATTR: &str = "init";
const MESSAGE_ATTR: &str = "message";
const ABI_STRUCT_ATTR: &str = "abi_struct";
const CREATE_ATTR: &str = "create";
const DECODE_ATTR: &str = "decode";
const DELETE_ATTR: &str = "delete";
const ENCODE_ATTR: &str = "encode";
const ENCODE_PACKED_ATTR: &str = "encode_packed";
const EVENT_ATTR: &str = "event";
const EVM_ARITH_ATTR: &str = "evm_arith";
const EVM_TEST_ATTR: &str = "evm_test";
const FALLBACK_ATTR: &str = "fallback";
const INTERFACE_ATTR: &str = "interface";
const INTERFACE_ID_ATTR: &str = "interface_id";
const SELECTOR_ATTR: &str = "selector";
const STATE_ATTR: &str = "state";
const STORAGE_ATTR: &str = "storage";

const EVM_CONTRACT_ATTR: &str = "evm_contract";
const PAYABLE_ATTR: &str = "payable";
const RECEIVE_ATTR: &str = "receive";
const VIEW_ATTR: &str = "view";
const PURE_ATTR: &str = "pure";

pub(crate) fn add_attributes_for_evm(known_attributes: &mut BTreeSet<String>) {
    known_attributes.insert(CALLABLE_ATTR.to_string());
    known_attributes.insert(CONTRACT_ATTR.to_string());
    known_attributes.insert(EXTERNAL_ATTR.to_string());

    known_attributes.insert(ABI_STRUCT_ATTR.to_string());
    known_attributes.insert(ACTOR_ATTR.to_string());
    known_attributes.insert(CREATE_ATTR.to_string());
    known_attributes.insert(DECODE_ATTR.to_string());
    known_attributes.insert(DELETE_ATTR.to_string());
    known_attributes.insert(ENCODE_ATTR.to_string());
    known_attributes.insert(ENCODE_PACKED_ATTR.to_string());
    known_attributes.insert(EVENT_ATTR.to_string());
    known_attributes.insert(EVM_ARITH_ATTR.to_string());
    known_attributes.insert(EVM_TEST_ATTR.to_string());
    known_attributes.insert(FALLBACK_ATTR.to_string());
    known_attributes.insert(INIT_ATTR.to_string());
    known_attributes.insert(INTERFACE_ATTR.to_string());
    known_attributes.insert(INTERFACE_ID_ATTR.to_string());
    known_attributes.insert(MESSAGE_ATTR.to_string());
    known_attributes.insert(SELECTOR_ATTR.to_string());
    known_attributes.insert(STATE_ATTR.to_string());
    known_attributes.insert(STORAGE_ATTR.to_string());

    known_attributes.insert(EVM_CONTRACT_ATTR.to_string());
    known_attributes.insert(PAYABLE_ATTR.to_string());
    known_attributes.insert(RECEIVE_ATTR.to_string());
    known_attributes.insert(VIEW_ATTR.to_string());
    known_attributes.insert(PURE_ATTR.to_string());
}

pub(crate) fn derive_for_evm(
    _env: &mut CompilationEnv,
    _address_map: &NamedAddressMap,
    def: &mut Definition,
) {
    if let Definition::Module(mod_def) = def {
        derive_module_for_evm(mod_def)
    }
}

fn derive_module_for_evm(mod_def: &mut ModuleDefinition) {
    if find_attr_slice(&mod_def.attributes, CONTRACT_ATTR).is_none() {
        // Not an EVM contract module
        return;
    }
    let mut new_funs = vec![];
    for mem in &mod_def.members {
        if let ModuleMember::Function(fun_def) = mem {
            if let Some(attr) = find_attr_slice(&fun_def.attributes, CALLABLE_ATTR) {
                // Generate a `call_<name>(contract: address, <args>)` native function for
                // cross contract calls.
                let loc = attr.loc;
                let call_name =
                    FunctionName(sp(loc, Symbol::from(format!("call_{}", fun_def.name))));
                let mut sign = fun_def.signature.clone();
                sign.parameters.insert(
                    0,
                    (
                        new_var(loc, "_target"),
                        new_simple_type(loc, "address", vec![]),
                    ),
                );
                // Create new #[external(params)] attribute, taking over parameters given to
                // #[callable].
                let attrs = sp(loc, vec![new_attr(
                    loc,
                    EXTERNAL_ATTR,
                    attr_params(attr).into_iter().cloned().collect(),
                )]);
                new_funs.push(new_native_fun(
                    loc,
                    call_name,
                    attrs,
                    Visibility::Public(loc),
                    None,
                    sign,
                ));
            }
        }
    }
    for fun_def in new_funs {
        mod_def.members.push(ModuleMember::Function(fun_def))
    }
}
