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
    language_storage::{ModuleId, StructTag, TypeTag},
    vm_status::StatusCode,
};
use move_vm_runtime::{
    config::VMConfig, move_vm::MoveVM, session::Session, IntoUnsyncCodeStorage,
    LocalModuleBytesStorage, ModuleStorage, TemporaryModuleStorage, UnreachableCodeStorage,
};
use move_vm_test_utils::InMemoryStorage;
use move_vm_types::gas::UnmeteredGasMeter;

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
    let mut mod_bytes = vec![];
    cm.serialize(&mut mod_bytes).unwrap();

    let vm_config = VMConfig {
        paranoid_type_checks: false,
        ..VMConfig::default()
    };
    let vm = MoveVM::new_with_config(vec![], vm_config);

    let resource_storage: InMemoryStorage = InMemoryStorage::new();
    let module_storage =
        LocalModuleBytesStorage::empty().into_unsync_code_storage(vm.runtime_environment());

    // Prepare type arguments.
    let mut ty_arg = TypeTag::U128;
    for _ in 0..4 {
        ty_arg = TypeTag::Struct(Box::new(StructTag {
            address: addr,
            module: Identifier::new("m").unwrap(),
            name: Identifier::new("s").unwrap(),
            type_args: vec![ty_arg; N],
        }));
    }

    // Publish (must succeed!) and the load the function.
    let mut session = vm.new_session(&resource_storage);
    if vm.vm_config().use_loader_v2 {
        let module_storage =
            TemporaryModuleStorage::new(&addr, vm.runtime_environment(), &module_storage, vec![
                mod_bytes.into(),
            ])
            .expect("Module must publish");
        load_function(&mut session, &module_storage, &cm.self_id(), &[ty_arg])
    } else {
        #[allow(deprecated)]
        session
            .publish_module(mod_bytes, addr, &mut UnmeteredGasMeter)
            .expect("Module must publish");
        load_function(&mut session, &UnreachableCodeStorage, &cm.self_id(), &[
            ty_arg,
        ])
    }
}

fn load_function(
    session: &mut Session,
    module_storage: &impl ModuleStorage,
    module_id: &ModuleId,
    ty_args: &[TypeTag],
) {
    let res = session.load_function(module_storage, module_id, ident_str!("f"), ty_args);
    assert!(
        res.is_err(),
        "Instantiation must fail at load time when converting from type tag to type "
    );
    assert_eq!(
        res.err().unwrap().major_status(),
        StatusCode::TOO_MANY_TYPE_NODES
    );
}
