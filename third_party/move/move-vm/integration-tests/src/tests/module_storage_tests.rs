// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::compiler::{as_module, compile_units};
use bytes::Bytes;
use move_core_types::{
    ability::{Ability, AbilitySet},
    account_address::AccountAddress,
    ident_str,
    identifier::Identifier,
    language_storage::{ModuleId, TypeTag},
};
use move_vm_runtime::{
    AsFunctionValueExtension, AsUnsyncModuleStorage, RuntimeEnvironment, WithRuntimeEnvironment,
};
use move_vm_test_utils::InMemoryStorage;
use move_vm_types::{
    loaded_data::runtime_types::{AbilityInfo, StructIdentifier, StructNameIndex, TypeBuilder},
    value_serde::FunctionValueExtension,
};
use std::str::FromStr;

#[cfg(test)]
fn module_bytes(module_code: &str) -> Bytes {
    let compiled_module = as_module(compile_units(module_code).unwrap().pop().unwrap());
    let mut bytes = vec![];
    compiled_module.serialize(&mut bytes).unwrap();
    bytes.into()
}

#[test]
fn test_function_value_extension() {
    let mut module_bytes_storage = InMemoryStorage::new();

    let code = r#"
        module 0x1::test {
            struct Foo {}

            fun b() { }
            fun c(a: u64, _foo: &Foo): u64 { a }
            fun d<A: drop, C: drop>(_a: A, b: u64, _c: C): u64 { b }
        }
    "#;
    let bytes = module_bytes(code);
    let test_id = ModuleId::new(AccountAddress::ONE, Identifier::new("test").unwrap());
    module_bytes_storage.add_module_bytes(&AccountAddress::ONE, ident_str!("test"), bytes);

    let code = r#"
        module 0x1::other_test {
            struct Bar has drop {}
        }
    "#;
    let bytes = module_bytes(code);
    let other_test_id = ModuleId::new(AccountAddress::ONE, Identifier::new("other_test").unwrap());
    module_bytes_storage.add_module_bytes(&AccountAddress::ONE, ident_str!("other_test"), bytes);

    let runtime_environment = RuntimeEnvironment::new(vec![]);
    let module_storage = module_bytes_storage.into_unsync_module_storage(runtime_environment);
    let function_value_extension = module_storage.as_function_value_extension();

    let result = function_value_extension.get_function_arg_tys(
        &ModuleId::new(AccountAddress::ONE, Identifier::new("test").unwrap()),
        ident_str!("a"),
        vec![],
    );
    assert!(result.is_err());

    let mut types = function_value_extension
        .get_function_arg_tys(&test_id, ident_str!("c"), vec![])
        .unwrap();
    assert_eq!(types.len(), 2);

    let ty_builder = TypeBuilder::with_limits(100, 100);
    let foo_ty = types.pop().unwrap();
    let name = module_storage
        .runtime_environment()
        .idx_to_struct_name_for_test(StructNameIndex(0))
        .unwrap();
    assert_eq!(name, StructIdentifier {
        module: test_id.clone(),
        name: Identifier::new("Foo").unwrap(),
    });
    assert_eq!(
        foo_ty,
        ty_builder
            .create_ref_ty(
                &ty_builder
                    .create_struct_ty(StructNameIndex(0), AbilityInfo::struct_(AbilitySet::EMPTY)),
                false
            )
            .unwrap()
    );
    let u64_ty = types.pop().unwrap();
    assert_eq!(u64_ty, ty_builder.create_u64_ty());

    // Generic function without type parameters  should fail.
    let result = function_value_extension.get_function_arg_tys(
        &ModuleId::new(AccountAddress::ONE, Identifier::new("test").unwrap()),
        ident_str!("d"),
        vec![],
    );
    assert!(result.is_err());

    let mut types = function_value_extension
        .get_function_arg_tys(&test_id, ident_str!("d"), vec![
            TypeTag::from_str("0x1::other_test::Bar").unwrap(),
            TypeTag::Vector(Box::new(TypeTag::U8)),
        ])
        .unwrap();
    assert_eq!(types.len(), 3);

    let vec_ty = types.pop().unwrap();
    assert_eq!(
        vec_ty,
        ty_builder
            .create_vec_ty(&ty_builder.create_u8_ty())
            .unwrap()
    );
    let u64_ty = types.pop().unwrap();
    assert_eq!(u64_ty, ty_builder.create_u64_ty());
    let bar_ty = types.pop().unwrap();
    let name = module_storage
        .runtime_environment()
        .idx_to_struct_name_for_test(StructNameIndex(1))
        .unwrap();
    assert_eq!(name, StructIdentifier {
        module: other_test_id,
        name: Identifier::new("Bar").unwrap(),
    });
    assert_eq!(
        bar_ty,
        ty_builder.create_struct_ty(
            StructNameIndex(1),
            AbilityInfo::struct_(AbilitySet::from_u8(Ability::Drop as u8).unwrap())
        )
    );
}
