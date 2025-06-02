// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    compiler::{as_module, compile_units},
    tests::execute_function_with_single_storage_for_test,
};
use claims::assert_ok;
use move_binary_format::errors::PartialVMResult;
use move_core_types::{
    account_address::AccountAddress, gas_algebra::GasQuantity, ident_str, identifier::Identifier,
    language_storage::ModuleId, vm_status::StatusCode,
};
use move_vm_runtime::{native_functions::NativeFunction, RuntimeEnvironment};
use move_vm_test_utils::InMemoryStorage;
use move_vm_types::natives::function::NativeResult;
use smallvec::SmallVec;
use std::sync::Arc;

const TEST_ADDR: AccountAddress = AccountAddress::new([42; AccountAddress::LENGTH]);

fn make_load_c() -> NativeFunction {
    Arc::new(move |_, _, _| -> PartialVMResult<NativeResult> {
        Ok(NativeResult::LoadModule {
            module_name: ModuleId::new(TEST_ADDR, Identifier::new("C").unwrap()),
        })
    })
}

fn make_dispatch_native() -> NativeFunction {
    Arc::new(move |_, _, _| -> PartialVMResult<NativeResult> {
        Ok(NativeResult::CallFunction {
            cost: GasQuantity::zero(),
            module_name: ModuleId::new(TEST_ADDR, Identifier::new("A").unwrap()),
            func_name: Identifier::new("foo").unwrap(),
            ty_args: vec![],
            args: SmallVec::new(),
        })
    })
}

fn make_dispatch_c_native() -> NativeFunction {
    Arc::new(move |_, _, _| -> PartialVMResult<NativeResult> {
        Ok(NativeResult::CallFunction {
            cost: GasQuantity::zero(),
            module_name: ModuleId::new(TEST_ADDR, Identifier::new("C").unwrap()),
            func_name: Identifier::new("foo").unwrap(),
            ty_args: vec![],
            args: SmallVec::new(),
        })
    })
}

fn make_dispatch_d_native() -> NativeFunction {
    Arc::new(move |_, _, _| -> PartialVMResult<NativeResult> {
        Ok(NativeResult::CallFunction {
            cost: GasQuantity::zero(),
            module_name: ModuleId::new(TEST_ADDR, Identifier::new("D").unwrap()),
            func_name: Identifier::new("foo3").unwrap(),
            ty_args: vec![],
            args: SmallVec::new(),
        })
    })
}

fn compile_and_publish(storage: &mut InMemoryStorage, code: String) {
    let mut units = compile_units(&code).unwrap();
    let m = as_module(units.pop().unwrap());
    let mut blob = vec![];
    m.serialize(&mut blob).unwrap();
    storage.add_module_bytes(m.self_addr(), m.self_name(), blob.into());
}

#[test]
fn runtime_reentrancy_check() {
    let natives = vec![
        (
            TEST_ADDR,
            Identifier::new("B").unwrap(),
            Identifier::new("dispatch").unwrap(),
            make_dispatch_native(),
        ),
        (
            TEST_ADDR,
            Identifier::new("B").unwrap(),
            Identifier::new("dispatch_c").unwrap(),
            make_dispatch_c_native(),
        ),
        (
            TEST_ADDR,
            Identifier::new("B").unwrap(),
            Identifier::new("dispatch_d").unwrap(),
            make_dispatch_d_native(),
        ),
        (
            TEST_ADDR,
            Identifier::new("B").unwrap(),
            Identifier::new("load_c").unwrap(),
            make_load_c(),
        ),
    ];
    let runtime_environment = RuntimeEnvironment::new(natives);
    let mut storage = InMemoryStorage::new_with_runtime_environment(runtime_environment);

    let code_1 = format!(
        r#"
        module 0x{0}::B {{
            public fun foo1() {{ Self::dispatch(0); return }}
            public fun foo2() {{ Self::load_c(); Self::dispatch_c(0); return }}
            public fun foo3() {{ Self::dispatch_d(0); return }}

            native fun dispatch(_f: u64);
            native fun dispatch_c(_f: u64);
            native fun dispatch_d(_f: u64);
            native fun load_c();
        }}
        "#,
        TEST_ADDR.to_hex(),
    );

    compile_and_publish(&mut storage, code_1);

    let code_2 = format!(
        r#"
        module 0x{0}::A {{
            use 0x{0}::B;
            public fun foo1() {{ B::foo1(); return }}
            public fun foo2() {{ B::foo2(); return }}
            public fun foo3() {{ B::foo3(); return }}

            public fun foo() {{ return }}
        }}
        module 0x{0}::B {{
            public fun foo1() {{ Self::dispatch(0); return }}
            public fun foo2() {{ Self::load_c(); Self::dispatch_c(0); return }}
            public fun foo3() {{ Self::dispatch_d(0); return }}

            native fun dispatch(_f: u64);
            native fun dispatch_c(_f: u64);
            native fun dispatch_d(_f: u64);
            native fun load_c();
        }}
        "#,
        TEST_ADDR.to_hex(),
    );

    compile_and_publish(&mut storage, code_2);

    let code_3 = format!(
        r#"
        module 0x{0}::C {{
            public fun foo() {{ return }}
        }}
        "#,
        TEST_ADDR.to_hex(),
    );

    compile_and_publish(&mut storage, code_3);

    let module_id = ModuleId::new(TEST_ADDR, Identifier::new("A").unwrap());

    // Call stack look like following:
    // A::foo1 -> B::foo1 -> B::dispatch -> A::foo3, Re-entrancy happens at foo3.
    let status = execute_function_with_single_storage_for_test(
        &storage,
        &module_id,
        ident_str!("foo1"),
        &[],
        vec![],
    )
    .unwrap_err()
    .major_status();
    assert_eq!(status, StatusCode::RUNTIME_DISPATCH_ERROR);

    // Call stack look like following:
    // A::foo2 -> B::foo2 -> B::dispatch_c -> C::foo3, No reentrancy, executed successfully.
    //
    // Note that C needs to be loaded into module cache at runtime.
    let result = execute_function_with_single_storage_for_test(
        &storage,
        &module_id,
        ident_str!("foo2"),
        &[],
        vec![],
    );
    assert_ok!(result);

    // Call stack look like following:
    // A::foo3 -> B::foo3 -> B::dispatch_d -> D::foo3, D doesn't exist, thus an error.
    let status = execute_function_with_single_storage_for_test(
        &storage,
        &module_id,
        ident_str!("foo3"),
        &[],
        vec![],
    )
    .unwrap_err()
    .major_status();
    assert_eq!(status, StatusCode::FUNCTION_RESOLUTION_FAILURE);
}
