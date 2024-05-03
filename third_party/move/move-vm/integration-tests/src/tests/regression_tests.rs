// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::compiler::{as_module, as_script, compile_units_with_stdlib};
use move_binary_format::file_format::{Bytecode, CompiledModule, CompiledScript, SignatureIndex};
use move_bytecode_verifier::VerifierConfig;
use move_core_types::{
    account_address::AccountAddress,
    identifier::Identifier,
    language_storage::{StructTag, TypeTag},
    vm_status::StatusCode,
};
use move_vm_runtime::{config::VMConfig, move_vm::MoveVM};
use move_vm_test_utils::InMemoryStorage;
use move_vm_types::gas::UnmeteredGasMeter;
use std::time::Instant;

fn get_nested_struct_type(
    depth: usize,
    num_type_args: usize,
    module_address: AccountAddress,
    module_identifier: Identifier,
    struct_identifier: Identifier,
) -> TypeTag {
    let mut ret = TypeTag::Bool;
    for _ in 0..depth {
        let type_params = std::iter::repeat(ret).take(num_type_args).collect();
        ret = TypeTag::Struct(Box::new(StructTag {
            address: module_address,
            module: module_identifier.clone(),
            name: struct_identifier.clone(),
            type_params,
        }))
    }
    ret
}
#[test]
fn script_large_ty() {
    let test_str = r#"
    script {
        use std::vector;
        use 0x42::pwn::Struct30TyArgs;

        use 0x42::pwn::Struct2TyArgs;
        fun f<A:drop> () {
            let v: vector<A> = vector::empty();
            let vv: vector<vector<Struct2TyArgs<Struct30TyArgs<A, A, A, A, A, A, A, A, A, A, A, A, A, A, A, A, A, A, A, A, A, A, A, A, A, A, A, A, A, A>, Struct30TyArgs<A, A, A, A, A, A, A, A, A, A, A, A, A, A, A, A, A, A, A, A, A, A, A, A, A, A, A, A, A, A>>>> = vector::empty();
        }
    }

    module 0x42::pwn {
        struct Struct2TyArgs<A1, A2> has drop {}

        struct Struct25TyArgs<A1, A2, A3, A4, A5, A6, A7, A8, A9, A10, A11, A12, A13, A14, A15, A16, A17, A18, A19, A20, A21, A22, A23, A24, A25> has drop {}

        struct Struct30TyArgs<A1, A2, A3, A4, A5, A6, A7, A8, A9, A10, A11, A12, A13, A14, A15, A16, A17, A18, A19, A20, A21, A22, A23, A24, A25, A26, A27, A28, A29, A30> has drop {}
    }
    "#;

    let mut units = compile_units_with_stdlib(test_str).unwrap();

    let mut decompiled_script = as_script(units.pop().unwrap());
    let decompiled_module = as_module(units.pop().unwrap());

    let verifier_config = VerifierConfig {
        max_loop_depth: Some(5),
        max_generic_instantiation_length: Some(32),
        max_function_parameters: Some(128),
        max_basic_blocks: Some(1024),
        max_value_stack_size: 1024,
        max_type_nodes: Some(256),
        max_push_size: Some(10000),
        max_dependency_depth: Some(100),
        max_struct_definitions: Some(200),
        max_fields_in_struct: Some(30),
        max_function_definitions: Some(1000),
        ..Default::default()
    };

    let script_code = &mut decompiled_script.code.code;
    script_code.clear();
    let num_vecpacks = 1000;
    for _ in 0..num_vecpacks {
        script_code.push(Bytecode::VecPack(SignatureIndex(2), 0));
    }
    for _ in 0..num_vecpacks {
        script_code.push(Bytecode::Pop);
    }
    script_code.push(Bytecode::Ret);

    move_bytecode_verifier::verify_module_with_config(&verifier_config, &decompiled_module)
        .unwrap();

    let start = Instant::now();
    move_bytecode_verifier::verify_script_with_config(&verifier_config, &decompiled_script)
        .unwrap();
    println!("script verification time: {:?}", start.elapsed());

    let mut script = vec![];
    decompiled_script.serialize(&mut script).unwrap();
    println!("Serialized len: {}", script.len());
    CompiledScript::deserialize(&script).unwrap();

    let mut module = vec![];
    decompiled_module.serialize(&mut module).unwrap();
    println!("Serialized len: {}", script.len());
    CompiledModule::deserialize(&module).unwrap();

    let mut storage = InMemoryStorage::new();
    let move_vm = MoveVM::new_with_config(vec![], VMConfig {
        verifier: verifier_config,
        paranoid_type_checks: true,
        type_size_limit: true,
        ..Default::default()
    })
    .unwrap();

    let module_address = AccountAddress::from_hex_literal("0x42").unwrap();
    let module_identifier = Identifier::new("pwn").unwrap();

    storage.publish_or_overwrite_module(decompiled_module.self_id(), module.to_vec());

    // constructs a type with about 25^3 nodes
    let num_type_args = 25;
    let struct_name = Identifier::new(format!("Struct{}TyArgs", num_type_args)).unwrap();
    let input_type = get_nested_struct_type(
        3,
        num_type_args,
        module_address,
        module_identifier,
        struct_name,
    );

    let mut session = move_vm.new_session(&storage);
    let res = session
        .execute_script(
            script.as_ref(),
            vec![input_type],
            Vec::<Vec<u8>>::new(),
            &mut UnmeteredGasMeter,
        )
        .unwrap_err();

    assert_eq!(res.major_status(), StatusCode::TOO_MANY_TYPE_NODES);
}
