// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use std::convert::TryFrom;

use crate::{compatibility::Compatibility, file_format::*, normalized};
use move_core_types::{account_address::AccountAddress, identifier::Identifier};

fn mk_module(vis: u8) -> normalized::Module {
    let (visibility, is_entry) = if vis == Visibility::DEPRECATED_SCRIPT {
        (Visibility::Public, true)
    } else {
        (Visibility::try_from(vis).unwrap(), false)
    };
    let m = CompiledModule {
        version: crate::file_format_common::VERSION_4,
        module_handles: vec![
            // only self module
            ModuleHandle {
                address: AddressIdentifierIndex(0),
                name: IdentifierIndex(0),
            },
        ],
        self_module_handle_idx: ModuleHandleIndex(0),
        identifiers: vec![
            Identifier::new("M").unwrap(),  // Module name
            Identifier::new("fn").unwrap(), // Function name
        ],
        address_identifiers: vec![
            AccountAddress::ZERO, // Module address
        ],
        function_handles: vec![
            // fun fn()
            FunctionHandle {
                module: ModuleHandleIndex(0),
                name: IdentifierIndex(1),
                parameters: SignatureIndex(0),
                return_: SignatureIndex(0),
                type_parameters: vec![],
            },
        ],
        function_defs: vec![
            // public(script) fun fn() { return; }
            FunctionDefinition {
                function: FunctionHandleIndex(0),
                visibility,
                is_entry,
                acquires_global_resources: vec![],
                code: Some(CodeUnit {
                    locals: SignatureIndex(0),
                    code: vec![Bytecode::Ret],
                }),
            },
        ],
        signatures: vec![
            Signature(vec![]), // void
        ],
        struct_defs: vec![],
        struct_handles: vec![],
        constant_pool: vec![],
        metadata: vec![],
        field_handles: vec![],
        friend_decls: vec![],
        struct_def_instantiations: vec![],
        function_instantiations: vec![],
        field_instantiations: vec![],
    };
    normalized::Module::new(&m)
}

#[test]
fn deprecated_unchanged_script_visibility() {
    let script_module = mk_module(Visibility::DEPRECATED_SCRIPT);
    assert!(Compatibility::full_check()
        .check(&script_module, &script_module)
        .is_ok(),);
}

#[test]
fn deprecated_remove_script_visibility() {
    let script_module = mk_module(Visibility::DEPRECATED_SCRIPT);
    // script -> private, not allowed
    let private_module = mk_module(Visibility::Private as u8);
    assert!(Compatibility::full_check()
        .check(&script_module, &private_module)
        .is_err());
    // script -> public, not allowed
    let public_module = mk_module(Visibility::Public as u8);
    assert!(Compatibility::full_check()
        .check(&script_module, &public_module)
        .is_err());
    // script -> friend, not allowed
    let friend_module = mk_module(Visibility::Friend as u8);
    assert!(Compatibility::full_check()
        .check(&script_module, &friend_module)
        .is_err());
}

#[test]
fn deprecated_add_script_visibility() {
    let script_module = mk_module(Visibility::DEPRECATED_SCRIPT);
    // private -> script, allowed
    let private_module = mk_module(Visibility::Private as u8);
    assert!(Compatibility::full_check()
        .check(&private_module, &script_module)
        .is_ok());
    // public -> script, not allowed
    let public_module = mk_module(Visibility::Public as u8);
    assert!(Compatibility::full_check()
        .check(&public_module, &script_module)
        .is_err());
    // friend -> script, not allowed
    let friend_module = mk_module(Visibility::Friend as u8);
    assert!(Compatibility::full_check()
        .check(&friend_module, &script_module)
        .is_err());
}
