// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_types::transaction::{
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
    let option_tag: Lazy<StructTag> =
        Lazy::new(|| StructTag::from_str("0x1::option::Option<u8>").unwrap());
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
        Struct(tag) => {
            if &**tag == Lazy::force(&str_tag) {
                Format::Seq(Box::new(Format::U8))
            } else if tag.address == Lazy::force(&option_tag).address
                && tag.module == Lazy::force(&option_tag).module
                && tag.name == Lazy::force(&option_tag).name
                && tag.type_args.len() == 1
            {
                Format::Option(Box::new(quote_type_as_format(&tag.type_args[0])))
            } else {
                type_not_allowed(type_tag)
            }
        },
        // TODO(#17645): signed integers
        Signer | Function(..) | I8 | I16 | I32 | I64 | I128 | I256 => type_not_allowed(type_tag),
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
    let option_tag: Lazy<StructTag> =
        Lazy::new(|| StructTag::from_str("0x1::option::Option<u8>").unwrap());

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
        Struct(tag) => {
            if &**tag == Lazy::force(&str_tag) {
                "string".into()
            } else if tag.address == Lazy::force(&option_tag).address
                && tag.module == Lazy::force(&option_tag).module
                && tag.name == Lazy::force(&option_tag).name
                && tag.type_args.len() == 1
            {
                format!("option{}", mangle_type(&tag.type_args[0]))
            } else {
                type_not_allowed(type_tag)
            }
        },
        // TODO(#17645): signed integers
        Signer | Function(..) | I8 | I16 | I32 | I64 | I128 | I256 => type_not_allowed(type_tag),
    }
}

pub(crate) fn get_external_definitions(aptos_types: &str) -> serde_generate::ExternalDefinitions {
    let definitions = vec![(aptos_types, vec![
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

#[cfg(test)]
mod tests {
    use super::*;
    use move_core_types::{
        account_address::AccountAddress,
        identifier::Identifier,
        language_storage::{ModuleId, StructTag, TypeTag},
        parser::parse_type_tag,
    };
    use serde_reflection::Format;

    /// Helper: build a 0x1::option::Option<inner> TypeTag.
    fn option_type(inner: TypeTag) -> TypeTag {
        TypeTag::Struct(Box::new(StructTag {
            address: AccountAddress::ONE,
            module: Identifier::new("option").unwrap(),
            name: Identifier::new("Option").unwrap(),
            type_args: vec![inner],
        }))
    }

    // ── mangle_type tests ──────────────────────────────────────

    #[test]
    fn test_mangle_type_primitives() {
        assert_eq!(mangle_type(&TypeTag::Bool), "bool");
        assert_eq!(mangle_type(&TypeTag::U8), "u8");
        assert_eq!(mangle_type(&TypeTag::U16), "u16");
        assert_eq!(mangle_type(&TypeTag::U32), "u32");
        assert_eq!(mangle_type(&TypeTag::U64), "u64");
        assert_eq!(mangle_type(&TypeTag::U128), "u128");
        assert_eq!(mangle_type(&TypeTag::U256), "u256");
        assert_eq!(mangle_type(&TypeTag::Address), "address");
    }

    #[test]
    fn test_mangle_type_option() {
        let type_tag1 = parse_type_tag("0x1::option::Option<u64>").unwrap();
        assert_eq!(mangle_type(&type_tag1), "optionu64");
        let type_tag2 = parse_type_tag("0x1::option::Option<bool>").unwrap();
        assert_eq!(mangle_type(&type_tag2), "optionbool");
        let type_tag3 = parse_type_tag("0x1::option::Option<address>").unwrap();
        assert_eq!(mangle_type(&type_tag3), "optionaddress");
    }
    #[test]
    fn test_mangle_type_option_nested() {
        // Nested Option<Option<u64>> is supported via recursion.
        let type_tag = parse_type_tag("0x1::option::Option<0x1::option::Option<u64>>").unwrap();
        assert_eq!(mangle_type(&type_tag), "optionoptionu64");
    }

    #[test]
    #[should_panic(expected = "Transaction scripts cannot take arguments of type")]
    fn test_mangle_type_option_unsupported_inner_type() {
        // An inner struct that is not String or Option should panic.
        let type_tag = parse_type_tag("0x1::option::Option<0x1::coin::Coin<u8>>").unwrap();
        mangle_type(&type_tag);
    }

    // ── quote_type_as_format tests ─────────────────────────────

    #[test]
    fn test_quote_type_as_format_option_u64() {
        let tag = option_type(TypeTag::U64);
        assert_eq!(
            quote_type_as_format(&tag),
            Format::Option(Box::new(Format::U64))
        );
    }

    #[test]
    fn test_quote_type_as_format_option_bool() {
        let tag = option_type(TypeTag::Bool);
        assert_eq!(
            quote_type_as_format(&tag),
            Format::Option(Box::new(Format::Bool))
        );
    }

    #[test]
    fn test_quote_type_as_format_option_u128() {
        let tag = option_type(TypeTag::U128);
        assert_eq!(
            quote_type_as_format(&tag),
            Format::Option(Box::new(Format::U128))
        );
    }

    #[test]
    fn test_quote_type_as_format_option_vector_u8() {
        let tag = option_type(TypeTag::Vector(Box::new(TypeTag::U8)));
        assert_eq!(
            quote_type_as_format(&tag),
            Format::Option(Box::new(Format::Seq(Box::new(Format::U8))))
        );
    }

    // ── make_abi_enum_container with Option arg ────────────────

    #[test]
    fn test_make_abi_enum_container_with_option_arg() {
        let abi = EntryABI::EntryFunction(EntryFunctionABI::new(
            "test_func".to_string(),
            ModuleId::new(AccountAddress::ONE, Identifier::new("test_module").unwrap()),
            String::new(),
            vec![],
            vec![ArgumentABI::new(
                "opt_val".to_string(),
                option_type(TypeTag::U64),
            )],
        ));
        // Should not panic — this was the original crash site.
        let container = make_abi_enum_container(&[abi]);
        match container {
            ContainerFormat::Enum(variants) => {
                assert_eq!(variants.len(), 1);
                let variant = variants.get(&0).unwrap();
                assert_eq!(variant.name, "TestModuleTestFunc");
            },
            _ => panic!("Expected Enum container"),
        }
    }
}
