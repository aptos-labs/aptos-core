// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use move_binary_format::file_format::{
    AbilitySet, AddressIdentifierIndex, Bytecode::*, CodeUnit, CompiledModule, FieldDefinition,
    FunctionDefinition, FunctionHandle, FunctionHandleIndex, IdentifierIndex, ModuleHandle,
    ModuleHandleIndex, Signature, SignatureIndex, SignatureToken, SignatureToken::*,
    StructDefinition, StructFieldInformation, StructHandle, StructHandleIndex, StructTypeParameter,
    TypeSignature,
};
use move_core_types::{
    account_address::AccountAddress,
    identifier::{IdentStr, Identifier},
    language_storage::{StructTag, TypeTag},
    vm_status::StatusCode,
};
use move_vm_runtime::move_vm::MoveVM;
use move_vm_test_utils::{gas_schedule::GasStatus, InMemoryStorage};

#[test]
fn instantiation_err() {
    let addr = AccountAddress::from_hex_literal("0xcafe").unwrap();

    let mut big_ty = SignatureToken::TypeParameter(0);

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
    };

    move_bytecode_verifier::verify_module(&cm).expect("verify failed");
    let vm = MoveVM::new(vec![]).unwrap();

    let storage: InMemoryStorage = InMemoryStorage::new();
    let mut session = vm.new_session(&storage);
    let mut mod_bytes = vec![];
    cm.serialize(&mut mod_bytes).unwrap();

    session
        .publish_module(mod_bytes, addr, &mut GasStatus::new_unmetered())
        .expect("Module must publish");

    let mut ty_arg = TypeTag::U128;
    for _ in 0..4 {
        // ty_arg = TypeTag::Vector(Box::new(ty_arg));
        ty_arg = TypeTag::Struct(Box::new(StructTag {
            address: addr,
            module: Identifier::new("m").unwrap(),
            name: Identifier::new("s").unwrap(),
            type_params: vec![ty_arg; N],
        }));
    }

    let err = session.execute_entry_function(
        &cm.self_id(),
        IdentStr::new("f").unwrap(),
        vec![ty_arg],
        Vec::<Vec<u8>>::new(),
        &mut GasStatus::new_unmetered(),
    );
    assert!(err.is_err(), "Instantiation must fail at runtime");
    assert_eq!(
        err.err().unwrap().major_status(),
        StatusCode::VERIFICATION_ERROR
    );
}
