// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Note[Orderless]: Done
use crate::MoveHarness;
use aptos_types::{move_utils::MemberId, transaction::ExecutionStatus};
use move_binary_format::{
    file_format::{
        AddressIdentifierIndex, Bytecode::*, CodeUnit, Constant, ConstantPoolIndex,
        FieldDefinition, FunctionDefinition, FunctionHandle, FunctionHandleIndex, IdentifierIndex,
        ModuleHandle, ModuleHandleIndex, Signature, SignatureIndex, SignatureToken,
        StructDefInstantiation, StructDefInstantiationIndex, StructDefinition,
        StructDefinitionIndex, StructFieldInformation, StructHandle, StructHandleIndex,
        StructTypeParameter, TypeSignature,
    },
    CompiledModule,
};
use move_core_types::{ability::AbilitySet, identifier::Identifier, vm_status::StatusCode};
use rstest::rstest;

#[rstest(
    stateless_account,
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(true, false, false),
    case(true, true, false),
    case(true, true, true),
    case(false, false, false),
    case(false, true, false),
    case(false, true, true)
)]
fn access_path_panic(
    stateless_account: bool,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    // github.com/aptos-labs/aptos-core/security/advisories/GHSA-rpw2-84hq-48jj
    let mut ty = SignatureToken::Bool;
    for _ in 0..18 {
        ty = SignatureToken::StructInstantiation(StructHandleIndex(0), vec![ty]);
    }

    let mut h = MoveHarness::new_with_flags(use_txn_payload_v2_format, use_orderless_transactions);
    let acc = h.new_account_with_key_pair(if stateless_account { None } else { Some(0) });
    let addr = *acc.address();

    let cm = CompiledModule {
        version: 6,
        self_module_handle_idx: ModuleHandleIndex(0),
        module_handles: vec![ModuleHandle {
            address: AddressIdentifierIndex(0),
            name: IdentifierIndex(0),
        }],
        struct_handles: vec![StructHandle {
            module: ModuleHandleIndex(0),
            name: IdentifierIndex(0),
            abilities: AbilitySet::ALL,
            type_parameters: vec![StructTypeParameter {
                constraints: AbilitySet::EMPTY,
                is_phantom: true,
            }],
        }],
        function_handles: vec![FunctionHandle {
            module: ModuleHandleIndex(0),
            name: IdentifierIndex(1),
            parameters: SignatureIndex(0),
            return_: SignatureIndex(0),
            type_parameters: vec![],
            access_specifiers: None,
            attributes: vec![],
        }],
        field_handles: vec![],
        friend_decls: vec![],
        struct_def_instantiations: vec![StructDefInstantiation {
            def: StructDefinitionIndex(0),
            type_parameters: SignatureIndex(1),
        }],
        function_instantiations: vec![],
        field_instantiations: vec![],
        signatures: vec![Signature(vec![]), Signature(vec![ty])],
        identifiers: vec![Identifier::new("M").unwrap(), Identifier::new("f").unwrap()],
        address_identifiers: vec![addr],
        constant_pool: vec![Constant {
            type_: SignatureToken::Address,
            data: bcs::to_bytes(&addr).unwrap(),
        }],
        metadata: vec![],
        struct_defs: vec![StructDefinition {
            struct_handle: StructHandleIndex(0),
            field_information: StructFieldInformation::Declared(vec![FieldDefinition {
                name: IdentifierIndex(0),
                signature: TypeSignature(SignatureToken::Bool),
            }]),
        }],
        function_defs: vec![FunctionDefinition {
            function: FunctionHandleIndex(0),
            visibility: move_binary_format::file_format::Visibility::Public,
            is_entry: true,
            acquires_global_resources: vec![],
            code: Some(CodeUnit {
                locals: SignatureIndex(0),
                code: vec![
                    LdConst(ConstantPoolIndex(0)),
                    ExistsGeneric(StructDefInstantiationIndex(0)),
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

    let mut module_bytes = vec![];
    cm.serialize(&mut module_bytes).unwrap();

    h.executor.add_module(&cm.self_id(), module_bytes);

    let res = h.run_entry_function(
        &acc,
        MemberId {
            module_id: cm.self_id(),
            member_id: Identifier::new("f").unwrap(),
        },
        vec![],
        Vec::<Vec<u8>>::new(),
    );

    assert_eq!(
        res.status().unwrap(),
        ExecutionStatus::MiscellaneousError(Some(StatusCode::VALUE_SERIALIZATION_ERROR))
    );
}
