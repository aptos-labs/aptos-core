// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use move_binary_format::file_format::{
    AbilitySet, AddressIdentifierIndex, Bytecode::*, CodeUnit, CompiledModule, FieldDefinition,
    FunctionDefinition, FunctionHandle, FunctionHandleIndex, IdentifierIndex, ModuleHandle,
    ModuleHandleIndex, Signature, SignatureIndex, SignatureToken::*, StructDefinition,
    StructFieldInformation, StructHandle, StructHandleIndex, StructTypeParameter, TypeSignature,
};
use move_core_types::{
    account_address::AccountAddress,
    ident_str,
    identifier::Identifier,
    language_storage::{StructTag, TypeTag},
    vm_status::StatusCode,
};
use move_vm_runtime::{config::VMConfig, move_vm::MoveVM, TestModuleStorage};
use move_vm_test_utils::InMemoryStorage;

#[test]
fn instantiation_err() {
    let addr = AccountAddress::from_hex_literal("0xcafe").unwrap();

    let mut big_ty = TypeParameter(0);

    const N: usize = 7;
    for _ in 0..2 {
        let mut ty_args = vec![];
        for _ in 0..N {
            ty_args.push(big_ty.clone());
        }
        big_ty = StructInstantiation(StructHandleIndex(0), ty_args);
    }

    let cm = CompiledModule {
        version: 6,
        self_module_handle_idx: ModuleHandleIndex(0),
        module_handles: vec![ModuleHandle {
            address: AddressIdentifierIndex(0),
            name: IdentifierIndex(0),
        }],
        struct_handles: vec![StructHandle {
            module: ModuleHandleIndex(0),
            name: IdentifierIndex(1),
            abilities: AbilitySet::ALL,
            type_parameters: vec![
                StructTypeParameter {
                    constraints: AbilitySet::EMPTY,
                    is_phantom: false,
                };
                N
            ],
        }],
        function_handles: vec![FunctionHandle {
            module: ModuleHandleIndex(0),
            name: IdentifierIndex(2),
            parameters: SignatureIndex(0),
            return_: SignatureIndex(0),
            type_parameters: vec![AbilitySet::PRIMITIVES],
            access_specifiers: None,
        }],
        field_handles: vec![],
        friend_decls: vec![],
        struct_def_instantiations: vec![],
        function_instantiations: vec![],
        field_instantiations: vec![],
        signatures: vec![Signature(vec![]), Signature(vec![big_ty])],
        identifiers: vec![
            Identifier::new("m").unwrap(),
            Identifier::new("s").unwrap(),
            Identifier::new("f").unwrap(),
            Identifier::new("field").unwrap(),
        ],
        address_identifiers: vec![addr],
        constant_pool: vec![],
        metadata: vec![],
        struct_defs: vec![StructDefinition {
            struct_handle: StructHandleIndex(0),
            field_information: StructFieldInformation::Declared(vec![FieldDefinition {
                name: IdentifierIndex(0),
                signature: TypeSignature(U8),
            }]),
        }],
        function_defs: vec![FunctionDefinition {
            function: FunctionHandleIndex(0),
            visibility: move_binary_format::file_format::Visibility::Public,
            is_entry: true,
            acquires_global_resources: vec![],
            code: Some(CodeUnit {
                locals: SignatureIndex(1),
                code: vec![
                    VecPack(SignatureIndex(1), 0),
                    // LdU8(0),
                    Pop,
                    Branch(0),
                ],
            }),
        }],
        // TODO(#13806): followup on whether we need specific tests for variants here
        struct_variant_handles: vec![],
        struct_variant_instantiations: vec![],
        variant_field_handles: vec![],
        variant_field_instantiations: vec![],
    };

    move_bytecode_verifier::verify_module(&cm).expect("verify failed");

    let vm_config = VMConfig {
        paranoid_type_checks: false,
        ..VMConfig::default()
    };
    let vm = MoveVM::new_with_config(vec![], vm_config);

    let mut resource_storage: InMemoryStorage = InMemoryStorage::new();
    let module_storage = TestModuleStorage::empty(&vm.vm_config().deserializer_config);

    // Verify we can publish this module.
    {
        let mut session = vm.new_session(&resource_storage);
        session
            .verify_module_bundle_before_publishing(&[cm.clone()], cm.self_addr(), &module_storage)
            .expect("Module must publish");
        drop(session);

        // Add it to module storage and restart the session.
        let mut mod_bytes = vec![];
        cm.serialize(&mut mod_bytes).unwrap();
        resource_storage.publish_or_overwrite_module(cm.self_id(), mod_bytes.clone());
        module_storage.add_module_bytes(cm.self_addr(), cm.self_name(), mod_bytes.into());
    }

    let mut session = vm.new_session(&resource_storage);
    let mut ty_arg = TypeTag::U128;
    for _ in 0..4 {
        ty_arg = TypeTag::Struct(Box::new(StructTag {
            address: addr,
            module: Identifier::new("m").unwrap(),
            name: Identifier::new("s").unwrap(),
            type_args: vec![ty_arg; N],
        }));
    }

    let res = session.load_function(&module_storage, &cm.self_id(), ident_str!("f"), &[ty_arg]);
    assert!(
        res.is_err(),
        "Instantiation must fail at load time when converting from type tag to type "
    );
    assert_eq!(
        res.err().unwrap().major_status(),
        StatusCode::TOO_MANY_TYPE_NODES
    );
}
