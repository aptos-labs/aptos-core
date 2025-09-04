// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use velor_types::transaction::{
    ArgumentABI, EntryABI, EntryFunctionABI, TransactionScriptABI, TypeArgumentABI,
};
use heck::ToUpperCamelCase;
use move_core_types::language_storage::{StructTag, TypeTag};
use once_cell::sync::Lazy;
use serde_reflection::{ContainerFormat, Format, Named, VariantFormat};
use std::{
    collections::{BTreeMap, BTreeSet},
    str::FromStr,
};

/// Useful error message.
pub(crate) fn type_not_allowed(type_tag: &TypeTag) -> ! {
    panic!(
        "Transaction scripts cannot take arguments of type {}.",
        type_tag.to_canonical_string()
    );
}

/// Clean up doc comments extracted by the Move prover.
pub(crate) fn prepare_doc_string(doc: &str) -> String {
    doc.replace("\n ", "\n").trim().to_string()
}

fn quote_type_as_format(type_tag: &TypeTag) -> Format {
    use TypeTag::*;
    let str_tag: Lazy<StructTag> =
        Lazy::new(|| StructTag::from_str("0x1::string::String").unwrap());
    match type_tag {
        Bool => Format::Bool,
        U8 => Format::U8,
        U16 => Format::U16,
        U32 => Format::U32,
        U64 => Format::U64,
        U128 => Format::U128,
        U256 => Format::TypeName("U256".into()),
        Address => Format::TypeName("AccountAddress".into()),
        Vector(type_tag) => Format::Seq(Box::new(quote_type_as_format(type_tag))),
        Struct(tag) => match tag {
            tag if &**tag == Lazy::force(&str_tag) => Format::Seq(Box::new(Format::U8)),
            _ => type_not_allowed(type_tag),
        },
        Signer | Function(..) => type_not_allowed(type_tag),
    }
}

fn quote_type_parameter_as_field(ty_arg: &TypeArgumentABI) -> Named<Format> {
    Named {
        name: ty_arg.name().to_string(),
        value: Format::TypeName("TypeTag".into()),
    }
}

fn quote_parameter_as_field(arg: &ArgumentABI) -> Named<Format> {
    Named {
        name: arg.name().to_string(),
        value: quote_type_as_format(arg.type_tag()),
    }
}

pub(crate) fn make_abi_enum_container(abis: &[EntryABI]) -> ContainerFormat {
    let mut variants = BTreeMap::new();
    for (index, abi) in abis.iter().enumerate() {
        let mut fields = Vec::new();
        for ty_arg in abi.ty_args() {
            fields.push(quote_type_parameter_as_field(ty_arg));
        }
        for arg in abi.args() {
            fields.push(quote_parameter_as_field(arg));
        }

        let name = match abi {
            EntryABI::EntryFunction(sf) => {
                format!(
                    "{}{}",
                    sf.module_name().name().to_string().to_upper_camel_case(),
                    abi.name().to_upper_camel_case()
                )
            },
            _ => abi.name().to_upper_camel_case(),
        };

        variants.insert(index as u32, Named {
            name,
            value: VariantFormat::Struct(fields),
        });
    }
    ContainerFormat::Enum(variants)
}

pub(crate) fn mangle_type(type_tag: &TypeTag) -> String {
    use TypeTag::*;
    let str_tag: Lazy<StructTag> =
        Lazy::new(|| StructTag::from_str("0x1::string::String").unwrap());

    match type_tag {
        Bool => "bool".into(),
        U8 => "u8".into(),
        U16 => "u16".into(),
        U32 => "u32".into(),
        U64 => "u64".into(),
        U128 => "u128".into(),
        U256 => "u256".into(),
        Address => "address".into(),
        Vector(type_tag) => match type_tag.as_ref() {
            U8 => "u8vector".into(),
            Vector(type_tag) => {
                if type_tag.as_ref() == &U8 {
                    "vecbytes".into()
                } else {
                    type_not_allowed(type_tag)
                }
            },
            _ => format!("vec{}", mangle_type(type_tag)),
        },
        Struct(tag) => match tag {
            tag if &**tag == Lazy::force(&str_tag) => "string".into(),
            _ => type_not_allowed(type_tag),
        },
        Signer | Function(..) => type_not_allowed(type_tag),
    }
}

pub(crate) fn get_external_definitions(velor_types: &str) -> serde_generate::ExternalDefinitions {
    let definitions = vec![(velor_types, vec![
        "AccountAddress",
        "TypeTag",
        "Script",
        "TransactionArgument",
    ])];
    definitions
        .into_iter()
        .map(|(module, defs)| {
            (
                module.to_string(),
                defs.into_iter().map(String::from).collect(),
            )
        })
        .collect()
}

pub(crate) fn get_required_helper_types(abis: &[EntryABI]) -> BTreeSet<&TypeTag> {
    let mut required_types = BTreeSet::new();
    for abi in abis {
        for arg in abi.args() {
            let type_tag = arg.type_tag();
            required_types.insert(type_tag);
        }
    }
    required_types
}

pub(crate) fn filter_transaction_scripts(abis: &[EntryABI]) -> Vec<EntryABI> {
    abis.iter()
        .filter(|abi| abi.is_transaction_script_abi())
        .cloned()
        .collect()
}

pub(crate) fn transaction_script_abis(abis: &[EntryABI]) -> Vec<TransactionScriptABI> {
    abis.iter()
        .cloned()
        .filter_map(|abi| match abi {
            EntryABI::TransactionScript(abi) => Some(abi),
            EntryABI::EntryFunction(_) => None,
        })
        .collect::<Vec<_>>()
}

pub(crate) fn entry_function_abis(abis: &[EntryABI]) -> Vec<EntryFunctionABI> {
    abis.iter()
        .cloned()
        .filter_map(|abi| match abi {
            EntryABI::EntryFunction(abi) => Some(abi),
            EntryABI::TransactionScript(_) => None,
        })
        .collect::<Vec<_>>()
}
