// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    compiler::{as_module, compile_units},
    tests::execute_function_for_test,
};
use move_binary_format::errors::PartialVMResult;
use move_core_types::{
    account_address::AccountAddress, gas_algebra::InternalGas, ident_str, identifier::Identifier,
};
use move_vm_runtime::{
<<<<<<< HEAD
<<<<<<< HEAD
    config::VMConfig, module_traversal::*, move_vm::MoveVM, native_functions::NativeFunction,
    session::Session, AsUnsyncCodeStorage, ModuleStorage, RuntimeEnvironment, StagingModuleStorage,
=======
    config::VMConfig, module_traversal::*, move_vm::MoveVm, native_functions::NativeFunction,
    AsUnsyncCodeStorage, ModuleStorage, RuntimeEnvironment, StagingModuleStorage,
>>>>>>> 7bae6066b8 ([refactoring] Remove resolver from session, use impl in sesson_ext and respawned)
=======
    native_functions::NativeFunction, AsUnsyncCodeStorage, RuntimeEnvironment, StagingModuleStorage,
>>>>>>> 35ea878580 (remove move vm session)
};
use move_vm_test_utils::InMemoryStorage;
use move_vm_types::natives::function::NativeResult;
use std::sync::Arc;

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
    let code = r#"
        module {{ADDR}}::M {
            fun foo() {
                Self::bar();
            }

            fun foo2() {
                Self::foo1();
            }

            fun foo1() {
                Self::bar();
            }

            native fun bar();
        }
    "#;
    let code = code.replace("{{ADDR}}", &format!("0x{}", TEST_ADDR.to_hex()));
    let mut units = compile_units(&code).unwrap();

    let m = as_module(units.pop().unwrap());
    let mut m_blob = vec![];
    m.serialize(&mut m_blob).unwrap();

    {
        let natives = vec![(
            TEST_ADDR,
            Identifier::new("M").unwrap(),
            Identifier::new("bar").unwrap(),
            make_failed_native(),
        )];
        let runtime_environment = RuntimeEnvironment::new(natives);
        let storage = InMemoryStorage::new_with_runtime_environment(runtime_environment);

<<<<<<< HEAD
<<<<<<< HEAD
        let mut sess = MoveVM::new_session(&storage);
=======
        let mut session = MoveVm::new_session();
>>>>>>> 7bae6066b8 ([refactoring] Remove resolver from session, use impl in sesson_ext and respawned)
=======
>>>>>>> 35ea878580 (remove move vm session)
        let module_storage = storage.as_unsync_code_storage();
        let new_module_storage =
            StagingModuleStorage::create(&TEST_ADDR, &module_storage, vec![m_blob.clone().into()])
                .expect("Module should be publishable");

        let err = execute_function_for_test(
            &storage,
            &new_module_storage,
            &m.self_id(),
            ident_str!("foo"),
            &[],
            vec![],
        )
        .unwrap_err();
        assert!(err.exec_state().unwrap().stack_trace().is_empty());

        let err = execute_function_for_test(
            &storage,
            &new_module_storage,
            &m.self_id(),
            ident_str!("foo2"),
            &[],
            vec![],
        )
        .unwrap_err();
        assert_eq!(err.exec_state().unwrap().stack_trace().len(), 1);
    }
}
