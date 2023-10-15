// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::{Context, Result};
use aptos_api_types::MoveModule;
use aptos_framework::{BuildOptions, BuiltPackage};
use aptos_types::account_address::AccountAddress;
use move_binary_format::file_format::AbilitySet;
use move_core_types::{
    identifier::Identifier,
    language_storage::{StructTag, TypeTag},
};
use move_resource_viewer::{AnnotatedMoveStruct, AnnotatedMoveValue};
use std::{collections::BTreeMap, path::PathBuf};

// This function builds a Move struct for the sake of testing. The output loosely
// matches the Tontine struct from banool/aptos-tontine:
// https://github.com/banool/aptos-tontine/blob/923e5bef79841b9f068d1f4c4208bda891516d91/move/sources/tontine.move#L210
pub fn build_tontine() -> AnnotatedMoveStruct {
    let tontine_config = AnnotatedMoveValue::Struct(annotated_move_struct(
        type_struct("0x789", "tontine", "Config"),
        vec![
            (
                identifier("description"),
                AnnotatedMoveValue::Struct(annotated_move_struct(
                    type_struct("0x1", "string", "String"),
                    vec![(
                        identifier("bytes"),
                        // The string "My Tontine" as UTF-8 bytes
                        AnnotatedMoveValue::Bytes(vec![
                            0x4D, 0x79, 0x20, 0x54, 0x6F, 0x6E, 0x74, 0x69, 0x6E, 0x65,
                        ]),
                    )],
                )),
            ),
            (
                identifier("per_member_amount_octa"),
                AnnotatedMoveValue::U64(100000),
            ),
            (
                identifier("delegation_pool"),
                AnnotatedMoveValue::Struct(annotated_move_struct(
                    type_struct("0x1", "option", "Option"),
                    vec![(
                        identifier("vec"),
                        // The string "My Tontine" as UTF-8 bytes
                        AnnotatedMoveValue::Vector(TypeTag::Address, vec![
                            AnnotatedMoveValue::Address(address("0x123")),
                        ]),
                    )],
                )),
            ),
        ],
    ));

    let member_data = AnnotatedMoveValue::Struct(annotated_move_struct(
        type_struct("0x1", "simple_map", "SimpleMap"),
        vec![(
            identifier("data"),
            AnnotatedMoveValue::Vector(
                TypeTag::Struct(Box::new(StructTag {
                    address: address("0x1"),
                    module: identifier("simple_map"),
                    name: identifier("Element"),
                    type_params: vec![
                        TypeTag::Address,
                        TypeTag::Struct(Box::new(StructTag {
                            address: address("0x789"),
                            module: identifier("tontine"),
                            name: identifier("MemberData"),
                            type_params: vec![],
                        })),
                    ],
                })),
                vec![AnnotatedMoveValue::Struct(annotated_move_struct(
                    type_struct("0x1", "simple_map", "Element"),
                    vec![
                        (
                            identifier("key"),
                            AnnotatedMoveValue::Address(address("0x123")),
                        ),
                        (
                            identifier("value"),
                            AnnotatedMoveValue::Struct(annotated_move_struct(
                                type_struct("0x789", "tontine", "MemberData"),
                                vec![
                                    (
                                        identifier("contributed_octa"),
                                        AnnotatedMoveValue::U64(50000),
                                    ),
                                    (
                                        identifier("reconfirmation_required"),
                                        AnnotatedMoveValue::Bool(false),
                                    ),
                                ],
                            )),
                        ),
                    ],
                ))],
            ),
        )],
    ));

    annotated_move_struct(type_struct("0x789", "tontine", "Tontine"), vec![
        (identifier("config"), tontine_config),
        (
            identifier("creation_time_secs"),
            AnnotatedMoveValue::U64(1686829095),
        ),
        (identifier("member_data"), member_data),
        (
            identifier("fallback_executed"),
            AnnotatedMoveValue::Bool(false),
        ),
        (identifier("funds_claimed_secs"), AnnotatedMoveValue::U64(0)),
        (
            identifier("funds_claimed_by"),
            AnnotatedMoveValue::Struct(annotated_move_struct(
                type_struct("0x1", "option", "Option"),
                vec![(
                    identifier("vec"),
                    // Empty vec because no one has claimed the funds yet.
                    AnnotatedMoveValue::Vector(TypeTag::Address, vec![]),
                )],
            )),
        ),
    ])
}

fn type_struct(addr: &str, module: &str, name: &str) -> StructTag {
    type_struct_with_type_params(addr, module, name, vec![])
}

fn type_struct_with_type_params(
    addr: &str,
    module: &str,
    name: &str,
    type_params: Vec<TypeTag>,
) -> StructTag {
    StructTag {
        address: address(addr),
        module: identifier(module),
        name: identifier(name),
        type_params,
    }
}

fn address(hex: &str) -> AccountAddress {
    AccountAddress::from_hex_literal(hex).unwrap()
}

fn annotated_move_struct(
    typ: StructTag,
    values: Vec<(Identifier, AnnotatedMoveValue)>,
) -> AnnotatedMoveStruct {
    AnnotatedMoveStruct {
        abilities: AbilitySet::EMPTY,
        type_: typ,
        value: values,
    }
}

fn identifier(id: &str) -> Identifier {
    Identifier::new(id).unwrap()
}

// Given the name of a Move package directory, compile it and return the compiled modules.
//
// TODO: Consider making something that memoizes this so we don't have to recompile the
// same stuff for each test.
pub fn compile_package(path: PathBuf) -> Result<Vec<MoveModule>> {
    let mut named_addresses = BTreeMap::new();
    named_addresses.insert("token_objects".to_string(), AccountAddress::TWO);
    named_addresses.insert("hero".to_string(), AccountAddress::TWO);
    let build_options = BuildOptions {
        with_abis: true,
        named_addresses,
        ..Default::default()
    };
    let pack = BuiltPackage::build(path.clone(), build_options)
        .with_context(|| format!("Failed to build package at {}", path.to_string_lossy()))?;
    pack.extract_metadata_and_save()
        .context("Failed to extract metadata and save")?;
    let modules: Vec<MoveModule> = pack.modules().cloned().map(|m| m.into()).collect();
    Ok(modules)
}
