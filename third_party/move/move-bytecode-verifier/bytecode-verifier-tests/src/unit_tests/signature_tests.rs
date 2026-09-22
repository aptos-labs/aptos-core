// Parts of the file are Copyright (c) The Diem Core Contributors
// Parts of the file are Copyright (c) The Move Contributors
// Parts of the file are Copyright (c) Aptos Foundation
// All Aptos Foundation code and content is licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use move_binary_format::{
    errors::VMResult,
    file_format::{Bytecode::*, CompiledModule, SignatureToken::*, Visibility::Public, *},
};
use move_bytecode_verifier::{verify_module, verify_module_with_config_for_test, VerifierConfig};
use move_core_types::{
    ability::AbilitySet, account_address::AccountAddress, identifier::Identifier,
    vm_status::StatusCode,
};

#[test]
fn no_verify_locals_good() {
    let compiled_module_good = CompiledModule {
        version: move_binary_format::file_format_common::VERSION_MAX,

        module_handles: vec![ModuleHandle {
            address: AddressIdentifierIndex(0),
            name: IdentifierIndex(0),
        }],
        self_module_handle_idx: ModuleHandleIndex(0),
        struct_handles: vec![],
        signatures: vec![
            Signature(vec![Address]),
            Signature(vec![U64]),
            Signature(vec![]),
        ],
        function_handles: vec![
            FunctionHandle {
                module: ModuleHandleIndex(0),
                name: IdentifierIndex(1),
                return_: SignatureIndex(2),
                parameters: SignatureIndex(0),
                type_parameters: vec![],
                access_specifiers: None,
                attributes: vec![],
            },
            FunctionHandle {
                module: ModuleHandleIndex(0),
                name: IdentifierIndex(2),
                return_: SignatureIndex(2),
                parameters: SignatureIndex(1),
                type_parameters: vec![],
                access_specifiers: None,
                attributes: vec![],
            },
        ],
        field_handles: vec![],
        friend_decls: vec![],
        struct_def_instantiations: vec![],
        function_instantiations: vec![],
        field_instantiations: vec![],
        identifiers: vec![
            Identifier::new("Bad").unwrap(),
            Identifier::new("blah").unwrap(),
            Identifier::new("foo").unwrap(),
        ],
        address_identifiers: vec![AccountAddress::new([0; AccountAddress::LENGTH])],
        constant_pool: vec![],
        metadata: vec![],
        struct_defs: vec![],
        function_defs: vec![
            FunctionDefinition {
                function: FunctionHandleIndex(0),
                visibility: Visibility::Public,
                is_entry: false,
                acquires_global_resources: vec![],
                code: Some(CodeUnit {
                    locals: SignatureIndex(0),
                    code: vec![Ret],
                }),
            },
            FunctionDefinition {
                function: FunctionHandleIndex(1),
                visibility: Visibility::Public,
                is_entry: false,
                acquires_global_resources: vec![],
                code: Some(CodeUnit {
                    locals: SignatureIndex(1),
                    code: vec![Ret],
                }),
            },
        ],
        struct_variant_handles: vec![],
        struct_variant_instantiations: vec![],
        variant_field_handles: vec![],
        variant_field_instantiations: vec![],
    };
    assert!(verify_module(&compiled_module_good).is_ok());
}

#[test]
fn big_signature_test() {
    const N_TYPE_PARAMS: usize = 5;
    const INSTANTIATION_DEPTH: usize = 3;
    const VECTOR_DEPTH: usize = 250;
    let mut st = SignatureToken::U8;
    for _ in 0..VECTOR_DEPTH {
        st = SignatureToken::Vector(Box::new(st));
    }
    for _ in 0..INSTANTIATION_DEPTH {
        let type_params = vec![st; N_TYPE_PARAMS];
        st = SignatureToken::StructInstantiation(StructHandleIndex(0), type_params);
    }

    const N_READPOP: u16 = 7500;

    let mut code = vec![];
    // 1. ImmBorrowLoc: ... ref
    // 2. ReadRef:      ... value
    // 3. Pop:          ...
    for _ in 0..N_READPOP {
        code.push(Bytecode::ImmBorrowLoc(0));
        code.push(Bytecode::ReadRef);
        code.push(Bytecode::Pop);
    }
    code.push(Bytecode::Ret);

    let type_param_constraints = StructTypeParameter {
        constraints: AbilitySet::EMPTY,
        is_phantom: false,
    };

    let module = CompiledModule {
        version: 5,
        self_module_handle_idx: ModuleHandleIndex(0),
        module_handles: vec![ModuleHandle {
            address: AddressIdentifierIndex(0),
            name: IdentifierIndex(0),
        }],
        struct_handles: vec![StructHandle {
            module: ModuleHandleIndex(0),
            name: IdentifierIndex(1),
            abilities: AbilitySet::ALL,
            type_parameters: vec![type_param_constraints; N_TYPE_PARAMS],
        }],
        function_handles: vec![FunctionHandle {
            module: ModuleHandleIndex(0),
            name: IdentifierIndex(0),
            parameters: SignatureIndex(1),
            return_: SignatureIndex(0),
            type_parameters: vec![],
            access_specifiers: None,
            attributes: vec![],
        }],
        field_handles: vec![],
        friend_decls: vec![],
        struct_def_instantiations: vec![],
        function_instantiations: vec![],
        field_instantiations: vec![],
        signatures: vec![Signature(vec![]), Signature(vec![st])],
        identifiers: vec![
            Identifier::new("f").unwrap(),
            Identifier::new("generic_struct").unwrap(),
        ],
        address_identifiers: vec![AccountAddress::ONE],
        constant_pool: vec![],
        metadata: vec![],
        struct_defs: vec![StructDefinition {
            struct_handle: StructHandleIndex(0),
            field_information: StructFieldInformation::Native,
        }],
        function_defs: vec![FunctionDefinition {
            function: FunctionHandleIndex(0),
            visibility: Public,
            is_entry: true,
            acquires_global_resources: vec![],
            code: Some(CodeUnit {
                locals: SignatureIndex(0),
                code,
            }),
        }],
        struct_variant_handles: vec![],
        struct_variant_instantiations: vec![],
        variant_field_handles: vec![],
        variant_field_instantiations: vec![],
    };

    // save module and verify that it can ser/de
    let mut mvbytes = vec![];
    module.serialize(&mut mvbytes).unwrap();
    let module = CompiledModule::deserialize(&mvbytes).unwrap();

    let res = verify_module_with_config_for_test(
        "big_signature_test",
        &VerifierConfig::production(),
        &module,
    )
    .unwrap_err();
    assert_eq!(res.major_status(), StatusCode::TOO_MANY_TYPE_NODES);
}

/// Builds a module whose only declared function takes `parameter`.
fn module_with_function_type_parameter(parameter: SignatureToken) -> CompiledModule {
    let mut module = empty_module();
    module.identifiers.push(Identifier::new("f").unwrap());
    module.signatures.push(Signature(vec![parameter]));
    module.function_handles.push(FunctionHandle {
        module: ModuleHandleIndex(0),
        name: IdentifierIndex(1),
        return_: SignatureIndex(0),
        parameters: SignatureIndex(1),
        type_parameters: vec![],
        access_specifiers: None,
        attributes: vec![],
    });
    module.function_defs.push(FunctionDefinition {
        function: FunctionHandleIndex(0),
        visibility: Public,
        is_entry: false,
        acquires_global_resources: vec![],
        code: Some(CodeUnit {
            locals: SignatureIndex(0),
            code: vec![Ret],
        }),
    });
    module
}

fn verify_function_type_abilities(module: &CompiledModule) -> VMResult<()> {
    verify_module_with_config_for_test(
        "function_type_abilities",
        &VerifierConfig::production(),
        module,
    )
}

/// Rejects `key`, which no function value can carry, in top-level and nested function types.
#[test]
fn function_type_abilities_are_bounded_by_public_functions() {
    let excess = AbilitySet::ALL;
    assert!(!excess.is_subset(AbilitySet::PUBLIC_FUNCTIONS));

    for parameter in [
        Function(vec![], vec![], excess),
        Vector(Box::new(Function(vec![], vec![], excess))),
        Function(
            vec![Function(vec![], vec![], excess)],
            vec![],
            AbilitySet::PUBLIC_FUNCTIONS,
        ),
    ] {
        let module = module_with_function_type_parameter(parameter);
        let err = verify_function_type_abilities(&module).unwrap_err();
        assert_eq!(err.major_status(), StatusCode::CONSTRAINT_NOT_SATISFIED);
    }
}

/// Accepts representative valid function ability sets. Top-level parameters require `drop` because
/// the helper function returns without consuming them, so `EMPTY` is tested in a nested position.
#[test]
fn legal_function_type_abilities_still_verify() {
    for parameter in [
        Function(vec![], vec![], AbilitySet::FUNCTIONS),
        Function(vec![], vec![], AbilitySet::PRIVATE_FUNCTIONS),
        Function(vec![], vec![], AbilitySet::PUBLIC_FUNCTIONS),
        Function(
            vec![Function(vec![], vec![], AbilitySet::EMPTY)],
            vec![],
            AbilitySet::PUBLIC_FUNCTIONS,
        ),
        Vector(Box::new(Function(
            vec![],
            vec![],
            AbilitySet::PUBLIC_FUNCTIONS,
        ))),
    ] {
        let module = module_with_function_type_parameter(parameter);
        verify_function_type_abilities(&module).unwrap();
    }
}

/// Allows excess function abilities when the verifier check is disabled.
#[test]
fn excess_function_type_abilities_verify_when_the_check_is_disabled() {
    let module = module_with_function_type_parameter(Function(vec![], vec![], AbilitySet::ALL));
    let config = VerifierConfig {
        check_function_type_abilities: false,
        ..VerifierConfig::production()
    };
    verify_module_with_config_for_test("function_type_abilities", &config, &module).unwrap();
}

#[test]
fn vec_pack_two_type_parameters() {
    // Create a module with vec_pack that has 2 type parameters instead of 1
    let module = CompiledModule {
        version: move_binary_format::file_format_common::VERSION_MAX,
        module_handles: vec![ModuleHandle {
            address: AddressIdentifierIndex(0),
            name: IdentifierIndex(0),
        }],
        self_module_handle_idx: ModuleHandleIndex(0),
        struct_handles: vec![],
        signatures: vec![
            Signature(vec![]),          // Empty signature for return type and locals
            Signature(vec![U64, Bool]), // Signature with 2 type parameters (invalid for vec_pack)
        ],
        function_handles: vec![FunctionHandle {
            module: ModuleHandleIndex(0),
            name: IdentifierIndex(1),
            return_: SignatureIndex(0),
            parameters: SignatureIndex(0),
            type_parameters: vec![],
            access_specifiers: None,
            attributes: vec![],
        }],
        field_handles: vec![],
        friend_decls: vec![],
        struct_def_instantiations: vec![],
        function_instantiations: vec![],
        field_instantiations: vec![],
        identifiers: vec![Identifier::new("M").unwrap(), Identifier::new("f").unwrap()],
        address_identifiers: vec![AccountAddress::new([0; AccountAddress::LENGTH])],
        constant_pool: vec![],
        metadata: vec![],
        struct_defs: vec![],
        function_defs: vec![FunctionDefinition {
            function: FunctionHandleIndex(0),
            visibility: Visibility::Public,
            is_entry: false,
            acquires_global_resources: vec![],
            code: Some(CodeUnit {
                locals: SignatureIndex(0),
                code: vec![
                    VecPack(SignatureIndex(1), 0), // vec_pack with 2 type parameters
                    Pop,
                    Ret,
                ],
            }),
        }],
        struct_variant_handles: vec![],
        struct_variant_instantiations: vec![],
        variant_field_handles: vec![],
        variant_field_instantiations: vec![],
    };

    let result = verify_module(&module);
    assert_eq!(
        result.unwrap_err().major_status(),
        StatusCode::NUMBER_OF_TYPE_ARGUMENTS_MISMATCH,
    );
}

#[test]
fn vec_pack_zero_type_parameters() {
    // Create a module with vec_pack that has 0 type parameters instead of 1
    let module = CompiledModule {
        version: move_binary_format::file_format_common::VERSION_MAX,
        module_handles: vec![ModuleHandle {
            address: AddressIdentifierIndex(0),
            name: IdentifierIndex(0),
        }],
        self_module_handle_idx: ModuleHandleIndex(0),
        struct_handles: vec![],
        signatures: vec![
            Signature(vec![]), // Empty signature for return type, locals, and vec_pack type args
        ],
        function_handles: vec![FunctionHandle {
            module: ModuleHandleIndex(0),
            name: IdentifierIndex(1),
            return_: SignatureIndex(0),
            parameters: SignatureIndex(0),
            type_parameters: vec![],
            access_specifiers: None,
            attributes: vec![],
        }],
        field_handles: vec![],
        friend_decls: vec![],
        struct_def_instantiations: vec![],
        function_instantiations: vec![],
        field_instantiations: vec![],
        identifiers: vec![Identifier::new("M").unwrap(), Identifier::new("f").unwrap()],
        address_identifiers: vec![AccountAddress::new([0; AccountAddress::LENGTH])],
        constant_pool: vec![],
        metadata: vec![],
        struct_defs: vec![],
        function_defs: vec![FunctionDefinition {
            function: FunctionHandleIndex(0),
            visibility: Visibility::Public,
            is_entry: false,
            acquires_global_resources: vec![],
            code: Some(CodeUnit {
                locals: SignatureIndex(0),
                code: vec![
                    VecPack(SignatureIndex(0), 0), // vec_pack with 0 type parameters
                    Pop,
                    Ret,
                ],
            }),
        }],
        struct_variant_handles: vec![],
        struct_variant_instantiations: vec![],
        variant_field_handles: vec![],
        variant_field_instantiations: vec![],
    };

    let result = verify_module(&module);
    assert_eq!(
        result.unwrap_err().major_status(),
        StatusCode::NUMBER_OF_TYPE_ARGUMENTS_MISMATCH,
    );
}
