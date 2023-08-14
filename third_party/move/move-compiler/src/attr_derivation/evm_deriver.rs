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

pub(crate) fn add_attributes_for_evm(attributes: &mut BTreeSet<String>) {
    const ALL_ATTRIBUTE_NAMES: [&str; 26] = [
        CALLABLE_ATTR,
        CONTRACT_ATTR,
        EXTERNAL_ATTR,
        ABI_STRUCT_ATTR,
        ACTOR_ATTR,
        CREATE_ATTR,
        DECODE_ATTR,
        DELETE_ATTR,
        ENCODE_ATTR,
        ENCODE_PACKED_ATTR,
        EVENT_ATTR,
        EVM_ARITH_ATTR,
        EVM_TEST_ATTR,
        FALLBACK_ATTR,
        INIT_ATTR,
        INTERFACE_ATTR,
        INTERFACE_ID_ATTR,
        MESSAGE_ATTR,
        SELECTOR_ATTR,
        STATE_ATTR,
        STORAGE_ATTR,
        EVM_CONTRACT_ATTR,
        PAYABLE_ATTR,
        RECEIVE_ATTR,
        VIEW_ATTR,
        PURE_ATTR,
    ];
    ALL_ATTRIBUTE_NAMES.into_iter().for_each(|elt| {
        attributes.insert(elt.to_string());
    });
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
