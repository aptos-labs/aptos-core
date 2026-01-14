// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use move_binary_format::file_format::{
    AddressIdentifierIndex, Bytecode::*, CodeUnit, CompiledModule, FieldDefinition,
    FunctionDefinition, FunctionHandle, FunctionHandleIndex, IdentifierIndex, ModuleHandle,
    ModuleHandleIndex, Signature, SignatureIndex, SignatureToken::*, StructDefinition,
    StructFieldInformation, StructHandle, StructHandleIndex, StructTypeParameter, TypeSignature,
};
use move_core_types::{
    ability::AbilitySet,
    account_address::AccountAddress,
    ident_str,
    identifier::Identifier,
    language_storage::{ModuleId, StructTag, TypeTag},
    vm_status::StatusCode,
};
use move_vm_runtime::{
    config::VMConfig,
    dispatch_loader,
    module_traversal::{TraversalContext, TraversalStorage},
    AsUnsyncCodeStorage, InstantiatedFunctionLoader, LegacyLoaderConfig, ModuleStorage,
    RuntimeEnvironment, StagingModuleStorage,
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
            attributes: vec![],
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
        ..VMConfig::default_for_test()
    };
    let runtime_environment = RuntimeEnvironment::new_with_config(vec![], vm_config);
    let storage: InMemoryStorage =
        InMemoryStorage::new_with_runtime_environment(runtime_environment);

    let mut mod_bytes = vec![];
    cm.serialize(&mut mod_bytes).unwrap();

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

    let module_storage = storage.as_unsync_code_storage();

    // Publish (must succeed!) and then load the function.
    let new_module_storage =
        StagingModuleStorage::create(&addr, &module_storage, vec![mod_bytes.into()])
            .expect("Module must publish");
    load_function(&new_module_storage, &cm.self_id(), &[ty_arg])
}

fn load_function(module_storage: &impl ModuleStorage, module_id: &ModuleId, ty_args: &[TypeTag]) {
    let traversal_storage = TraversalStorage::new();
    let res = dispatch_loader!(module_storage, loader, {
        loader.load_instantiated_function(
            &LegacyLoaderConfig::unmetered(),
            &mut UnmeteredGasMeter,
            &mut TraversalContext::new(&traversal_storage),
            module_id,
            ident_str!("f"),
            ty_args,
        )
    });
    assert!(
        res.is_err(),
        "Instantiation must fail at load time when converting from type tag to type "
    );
    assert_eq!(
        res.err().unwrap().major_status(),
        StatusCode::TOO_MANY_TYPE_NODES
    );
}
