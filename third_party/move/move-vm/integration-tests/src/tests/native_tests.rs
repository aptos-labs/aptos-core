// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::tests::{compile_and_publish, execute_function_for_test};
use claims::assert_ok;
use move_binary_format::{
    errors::PartialVMResult, file_format::empty_module_with_dependencies_and_friends_at_addr,
    CompiledModule,
};
use move_core_types::{
    account_address::AccountAddress, gas_algebra::InternalGas, ident_str, identifier::Identifier,
    language_storage::ModuleId,
};
use move_vm_runtime::{
    config::VMConfig, native_functions::NativeFunction, AsUnsyncCodeStorage, RuntimeEnvironment,
};
use move_vm_test_utils::InMemoryStorage;
use move_vm_types::natives::function::NativeResult;
use std::sync::Arc;
use test_case::test_case;

const TEST_ADDR: AccountAddress = AccountAddress::new([42; AccountAddress::LENGTH]);

fn make_failed_native() -> NativeFunction {
    Arc::new(move |_, _, _| -> PartialVMResult<NativeResult> {
        Ok(NativeResult::Abort {
            cost: InternalGas::new(0),
            abort_code: 12,
        })
    })
}

#[test]
fn test_failed_native() {
    let module_id = ModuleId::new(TEST_ADDR, Identifier::new("M").unwrap());
    let natives = vec![(
        TEST_ADDR,
        Identifier::new("M").unwrap(),
        Identifier::new("bar").unwrap(),
        make_failed_native(),
    )];

    let runtime_environment = RuntimeEnvironment::new(natives);
    let mut storage = InMemoryStorage::new_with_runtime_environment(runtime_environment);

    let code = format!(
        r#"
        module 0x{0}::M {{
            fun foo() {{ Self::bar(); }}
            fun foo1() {{ Self::bar(); }}
            fun foo2() {{ Self::foo1(); }}

            native fun bar();
        }}
        "#,
        TEST_ADDR.to_hex(),
    );
    compile_and_publish(&mut storage, code);
    let module_storage = storage.as_unsync_code_storage();

    let err = execute_function_for_test(
        &storage,
        &module_storage,
        &module_id,
        ident_str!("foo"),
        &[],
        vec![],
    )
    .unwrap_err();
    assert!(err.exec_state().unwrap().stack_trace().is_empty());

    let err = execute_function_for_test(
        &storage,
        &module_storage,
        &module_id,
        ident_str!("foo2"),
        &[],
        vec![],
    )
    .unwrap_err();
    assert_eq!(err.exec_state().unwrap().stack_trace().len(), 1);
}

fn make_load_module_b() -> NativeFunction {
    Arc::new(move |_, _, _| -> PartialVMResult<NativeResult> {
        Ok(NativeResult::LoadModule {
            module_name: ModuleId::new(TEST_ADDR, ident_str!("b").to_owned()),
        })
    })
}

#[test_case(true)]
#[test_case(false)]
fn test_load_module_native_result(enable_lazy_loading: bool) {
    let a_id = ModuleId::new(TEST_ADDR, ident_str!("a").to_owned());
    let natives = vec![(
        TEST_ADDR,
        ident_str!("a").to_owned(),
        ident_str!("load_module_b").to_owned(),
        make_load_module_b(),
    )];
    let runtime_environment = RuntimeEnvironment::new_with_config(natives, VMConfig {
        enable_lazy_loading,
        ..VMConfig::default_for_test()
    });
    let mut storage = InMemoryStorage::new_with_runtime_environment(runtime_environment);

    let code_a = format!(
        r#"
        module 0x{0}::a {{
            fun foo() {{ load_module_b(); }}
            native fun load_module_b();
        }}
        "#,
        TEST_ADDR.to_hex(),
    );
    compile_and_publish(&mut storage, code_a);

    let mut add_module = |m: CompiledModule| {
        let mut blob = vec![];
        m.serialize(&mut blob).unwrap();
        storage.add_module_bytes(m.self_addr(), m.self_name(), blob.into());
        m.self_id()
    };

    let b = empty_module_with_dependencies_and_friends_at_addr(TEST_ADDR, "b", vec!["c"], vec![]);
    let c = empty_module_with_dependencies_and_friends_at_addr(TEST_ADDR, "c", vec!["d"], vec![]);
    let d = empty_module_with_dependencies_and_friends_at_addr(TEST_ADDR, "d", vec![], vec![]);

    let b_id = add_module(b);
    let c_id = add_module(c);
    let d_id = add_module(d);

    let code_storage = storage.as_unsync_code_storage();
    assert_ok!(execute_function_for_test(
        &storage,
        &code_storage,
        &a_id,
        ident_str!("foo"),
        &[],
        vec![],
    ));

    // Here we assert that the state of the cache contains specified modules. For lazy loading,
    // only loaded modules is deserialized for charging and cached. For eager loading, the
    // transitive closure is deserialized and cached for charging. The modules are not verified
    // because verification happens when other natives is called to load a function from the module
    // (here, "b").
    if enable_lazy_loading {
        code_storage
            .module_storage()
            .assert_cached_state(vec![&b_id], vec![&a_id]);
    } else {
        code_storage
            .module_storage()
            .assert_cached_state(vec![&b_id, &c_id, &d_id], vec![&a_id]);
    }
}
