// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::tests::execute_function_with_single_storage_for_test;
use claims::assert_err;
use move_binary_format::{
    errors::VMResult,
    file_format::{
        AccessKind, AccessSpecifier, AddressIdentifierIndex, AddressSpecifier, Bytecode, CodeUnit,
        CompiledModule, FieldDefinition, FunctionDefinition, FunctionHandle, FunctionHandleIndex,
        IdentifierIndex, ModuleHandle, ModuleHandleIndex, ResourceSpecifier, Signature,
        SignatureIndex, SignatureToken, StructDefinition, StructFieldInformation, StructHandle,
        StructHandleIndex, TableIndex, TypeSignature, Visibility,
    },
};
use move_core_types::{
    ability::AbilitySet, account_address::AccountAddress, identifier::Identifier,
    language_storage::TypeTag, value::MoveValue, vm_status::StatusCode,
};
use move_vm_test_utils::InMemoryStorage;

fn make_module_with_function(
    parameters: Signature,
    return_: Signature,
    type_parameters: Vec<AbilitySet>,
    access_specifiers: Vec<AccessSpecifier>,
) -> (CompiledModule, Identifier) {
    let function_name = Identifier::new("foo").unwrap();
    let mut signatures = vec![Signature(vec![])];
    let parameters_idx = match signatures
        .iter()
        .enumerate()
        .find(|(_, s)| *s == &parameters)
    {
        Some((idx, _)) => SignatureIndex(idx as TableIndex),
        None => {
            signatures.push(parameters);
            SignatureIndex((signatures.len() - 1) as TableIndex)
        },
    };
    let return_idx = match signatures.iter().enumerate().find(|(_, s)| *s == &return_) {
        Some((idx, _)) => SignatureIndex(idx as TableIndex),
        None => {
            signatures.push(return_);
            SignatureIndex((signatures.len() - 1) as TableIndex)
        },
    };
    let module = CompiledModule {
        version: move_binary_format::file_format_common::VERSION_MAX,
        self_module_handle_idx: ModuleHandleIndex(0),
        module_handles: vec![ModuleHandle {
            address: AddressIdentifierIndex(0),
            name: IdentifierIndex(0),
        }],
        struct_handles: vec![StructHandle {
            module: ModuleHandleIndex(0),
            name: IdentifierIndex(1),
            abilities: AbilitySet::EMPTY,
            type_parameters: vec![],
        }],
        function_handles: vec![FunctionHandle {
            module: ModuleHandleIndex(0),
            name: IdentifierIndex(2),
            parameters: parameters_idx,
            return_: return_idx,
            type_parameters,
            access_specifiers: Some(access_specifiers),
            attributes: vec![],
        }],
        field_handles: vec![],
        friend_decls: vec![],

        struct_def_instantiations: vec![],
        function_instantiations: vec![],
        field_instantiations: vec![],

        signatures,

        identifiers: vec![
            Identifier::new("M").unwrap(),
            Identifier::new("X").unwrap(),
            function_name.clone(),
        ],
        address_identifiers: vec![AccountAddress::random()],
        constant_pool: vec![],
        metadata: vec![],

        struct_defs: vec![StructDefinition {
            struct_handle: StructHandleIndex(0),
            field_information: StructFieldInformation::Declared(vec![FieldDefinition {
                name: IdentifierIndex(1),
                signature: TypeSignature(SignatureToken::Bool),
            }]),
        }],
        function_defs: vec![FunctionDefinition {
            function: FunctionHandleIndex(0),
            visibility: Visibility::Public,
            is_entry: false,
            acquires_global_resources: vec![],
            code: Some(CodeUnit {
                locals: SignatureIndex(0),
                code: vec![Bytecode::LdU64(0), Bytecode::Abort],
            }),
        }],
        struct_variant_handles: vec![],
        struct_variant_instantiations: vec![],
        variant_field_handles: vec![],
        variant_field_instantiations: vec![],
    };
    (module, function_name)
}

fn load_and_call_function(
    module: CompiledModule,
    function_name: Identifier,
    non_signer_args: Vec<Vec<u8>>,
    ty_args: Vec<TypeTag>,
    signers: Vec<AccountAddress>,
) -> VMResult<()> {
    let mut storage = InMemoryStorage::new();

    let module_id = module.self_id();
    let mut module_blob = vec![];
    module.serialize(&mut module_blob).unwrap();

    storage.add_module_bytes(module_id.address(), module_id.name(), module_blob.into());

    execute_function_with_single_storage_for_test(
        &storage,
        &module_id,
        function_name.as_ident_str(),
        &ty_args,
        signers
            .into_iter()
            .map(|s| MoveValue::Signer(s).simple_serialize().unwrap())
            .chain(non_signer_args)
            .collect(),
    )?;
    Ok(())
}

#[test]
fn rac_declared_at_ok() {
    let (module, function_name) =
        make_module_with_function(Signature::default(), Signature::default(), vec![], vec![
            AccessSpecifier {
                kind: AccessKind::Reads,
                negated: false,
                resource: ResourceSpecifier::DeclaredAtAddress(AddressIdentifierIndex::new(0)),
                address: AddressSpecifier::Any,
            },
        ]);
    let err = assert_err!(load_and_call_function(
        module,
        function_name,
        vec![],
        vec![],
        vec![]
    ));
    // aborted means function executed
    assert_eq!(err.major_status(), StatusCode::ABORTED)
}

#[test]
fn rac_declared_at_fail() {
    let (module, function_name) =
        make_module_with_function(Signature::default(), Signature::default(), vec![], vec![
            AccessSpecifier {
                kind: AccessKind::Reads,
                negated: false,
                resource: ResourceSpecifier::DeclaredAtAddress(AddressIdentifierIndex::new(1)),
                address: AddressSpecifier::Any,
            },
        ]);
    let err = assert_err!(load_and_call_function(
        module,
        function_name,
        vec![],
        vec![],
        vec![]
    ));
    // bounds checker error surfaces as serialization error
    assert_eq!(
        err.major_status(),
        StatusCode::UNEXPECTED_DESERIALIZATION_ERROR,
        "{:?}",
        err
    )
}

#[test]
fn rac_declared_in_module_fail() {
    let (module, function_name) =
        make_module_with_function(Signature::default(), Signature::default(), vec![], vec![
            AccessSpecifier {
                kind: AccessKind::Reads,
                negated: false,
                resource: ResourceSpecifier::DeclaredInModule(ModuleHandleIndex::new(1)),
                address: AddressSpecifier::Any,
            },
        ]);
    let err = assert_err!(load_and_call_function(
        module,
        function_name,
        vec![],
        vec![],
        vec![]
    ));
    // bounds checker error surfaces as serialization error
    assert_eq!(
        err.major_status(),
        StatusCode::UNEXPECTED_DESERIALIZATION_ERROR,
        "{:?}",
        err
    )
}

#[test]
fn rac_resource_ok() {
    let (module, function_name) =
        make_module_with_function(Signature::default(), Signature::default(), vec![], vec![
            AccessSpecifier {
                kind: AccessKind::Reads,
                negated: false,
                resource: ResourceSpecifier::Resource(StructHandleIndex::new(0)),
                address: AddressSpecifier::Any,
            },
        ]);
    let err = assert_err!(load_and_call_function(
        module,
        function_name,
        vec![],
        vec![],
        vec![]
    ));
    assert_eq!(err.major_status(), StatusCode::ABORTED, "{:?}", err)
}

#[test]
fn rac_resource_fail() {
    let (module, function_name) =
        make_module_with_function(Signature::default(), Signature::default(), vec![], vec![
            AccessSpecifier {
                kind: AccessKind::Reads,
                negated: false,
                resource: ResourceSpecifier::Resource(StructHandleIndex::new(1)),
                address: AddressSpecifier::Any,
            },
        ]);
    let err = assert_err!(load_and_call_function(
        module,
        function_name,
        vec![],
        vec![],
        vec![]
    ));
    assert_eq!(
        err.major_status(),
        StatusCode::UNEXPECTED_DESERIALIZATION_ERROR,
        "{:?}",
        err
    )
}

#[test]
fn rac_resource_instantiation_fail() {
    let (module, function_name) =
        make_module_with_function(Signature::default(), Signature::default(), vec![], vec![
            AccessSpecifier {
                kind: AccessKind::Reads,
                negated: false,
                resource: ResourceSpecifier::ResourceInstantiation(
                    StructHandleIndex::new(0),
                    SignatureIndex::new(2),
                ),
                address: AddressSpecifier::Any,
            },
        ]);
    let err = assert_err!(load_and_call_function(
        module,
        function_name,
        vec![],
        vec![],
        vec![]
    ));
    assert_eq!(
        err.major_status(),
        StatusCode::UNEXPECTED_DESERIALIZATION_ERROR,
        "{:?}",
        err
    )
}
