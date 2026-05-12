// Parts of the file are Copyright (c) The Diem Core Contributors
// Parts of the file are Copyright (c) The Move Contributors
// Parts of the file are Copyright (c) Aptos Foundation
// All Aptos Foundation code and content is licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{compatibility::Compatibility, file_format::*};
use move_core_types::{account_address::AccountAddress, identifier::Identifier};
use std::convert::TryFrom;

#[allow(deprecated)]
fn mk_module(vis: u8) -> CompiledModule {
    let (visibility, is_entry) = if vis == Visibility::DEPRECATED_SCRIPT {
        (Visibility::Public, true)
    } else {
        (Visibility::try_from(vis).unwrap(), false)
    };
    mk_module_with_entry(visibility, is_entry, crate::file_format_common::VERSION_4)
}

fn mk_module_with_entry(visibility: Visibility, is_entry: bool, version: u32) -> CompiledModule {
    CompiledModule {
        version,
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
                access_specifiers: None,
                attributes: vec![],
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
        struct_variant_handles: vec![],
        struct_variant_instantiations: vec![],
        variant_field_handles: vec![],
        variant_field_instantiations: vec![],
    }
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

#[test]
fn friend_entry_to_private_entry() {
    // `public(friend) entry fun f()` -> `entry fun f()`.
    // The transaction-callable surface is preserved by `entry`; friend visibility is
    // independently demoted to private. This must be allowed when `check_friend_linking`
    // is off (i.e. friend is treated as private) AND the flag is on.
    let version = crate::file_format_common::VERSION_7;
    let friend_entry = mk_module_with_entry(Visibility::Friend, true, version);
    let private_entry = mk_module_with_entry(Visibility::Private, true, version);

    // Flag on, friend linking off (production post-rollout state): allowed.
    let allow = Compatibility::new(
        true,  // check_struct_layout
        false, // check_friend_linking (TREAT_FRIEND_AS_PRIVATE on)
        true,  // treat_entry_as_public
        false, // function_type_compat_bug
        true,  // allow_friend_entry_visibility_downgrade
    );
    assert!(allow.check(&friend_entry, &private_entry).is_ok());

    // Flag off, friend linking off (production pre-rollout state): rejected — pins down
    // that the gate matters and the legacy behavior is preserved before activation.
    let pre_rollout = Compatibility::new(true, false, true, false, false);
    assert!(pre_rollout.check(&friend_entry, &private_entry).is_err());

    // Strict mode (check_friend_linking=true): rejected even with the flag on.
    // Friend linking is a contract; we don't relax it here.
    let strict = Compatibility::new(true, true, true, false, true);
    assert!(strict.check(&friend_entry, &private_entry).is_err());
}

#[test]
fn friend_entry_to_non_entry_still_rejected() {
    // `public(friend) entry fun f()` -> `fun f()` (entry removed): always rejected,
    // because `is_entry_compatible` requires entry to remain entry. Confirms that the
    // visibility relaxation does not bypass the entry check.
    let version = crate::file_format_common::VERSION_7;
    let friend_entry = mk_module_with_entry(Visibility::Friend, true, version);
    let private_non_entry = mk_module_with_entry(Visibility::Private, false, version);

    let allow = Compatibility::new(true, false, true, false, true);
    assert!(allow.check(&friend_entry, &private_non_entry).is_err());
}

#[test]
fn public_entry_to_private_entry_still_rejected() {
    // `public entry fun f()` -> `entry fun f()`: always rejected because public must
    // remain public — pins down that we did not over-relax the `Public` arm.
    let version = crate::file_format_common::VERSION_7;
    let public_entry = mk_module_with_entry(Visibility::Public, true, version);
    let private_entry = mk_module_with_entry(Visibility::Private, true, version);

    let allow = Compatibility::new(true, false, true, false, true);
    assert!(allow.check(&public_entry, &private_entry).is_err());
}
