// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::compiler::{as_module, compile_units};
use move_binary_format::errors::PartialVMResult;
use move_bytecode_verifier::VerifierConfig;
use move_core_types::{
    account_address::AccountAddress, gas_algebra::InternalGas, identifier::Identifier,
    language_storage::ModuleId,
};
use move_vm_runtime::{
    config::VMConfig, module_traversal::*, move_vm::MoveVM, native_functions::NativeFunction,
    session::Session, AsUnsyncCodeStorage, ModuleStorage, RuntimeEnvironment, StagingModuleStorage,
};
use move_vm_test_utils::InMemoryStorage;
use move_vm_types::{gas::UnmeteredGasMeter, natives::function::NativeResult};
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
fn test_publish_module_with_nested_loops() {
    let code = r#"
        module {{ADDR}}::M {
            entry fun foo() {
                Self::bar();
            }

            entry fun foo2() {
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
    let traversal_storage = TraversalStorage::new();

    {
        let storage = InMemoryStorage::new();

        let natives = vec![(
            TEST_ADDR,
            Identifier::new("M").unwrap(),
            Identifier::new("bar").unwrap(),
            make_failed_native(),
        )];
        let vm_config = VMConfig {
            verifier_config: VerifierConfig {
                max_loop_depth: Some(2),
                ..Default::default()
            },
            ..Default::default()
        };
        let runtime_environment =
            RuntimeEnvironment::new_with_config(natives.clone(), vm_config.clone());
        let vm = MoveVM::new_with_config(natives, vm_config);

        let mut sess = vm.new_session(&storage);
        let module_storage = storage.as_unsync_code_storage(&runtime_environment);
        if vm.vm_config().use_loader_v2 {
            let new_module_storage =
                StagingModuleStorage::create(&TEST_ADDR, &module_storage, vec![m_blob
                    .clone()
                    .into()])
                .expect("Module should be publishable");
            load_and_run_functions(
                &mut sess,
                &new_module_storage,
                &traversal_storage,
                &m.self_id(),
            );
        } else {
            #[allow(deprecated)]
            sess.publish_module(m_blob.clone(), TEST_ADDR, &mut UnmeteredGasMeter)
                .unwrap();
            load_and_run_functions(&mut sess, &module_storage, &traversal_storage, &m.self_id());
        };
    }
}

fn load_and_run_functions(
    session: &mut Session,
    module_storage: &impl ModuleStorage,
    traversal_storage: &TraversalStorage,
    module_id: &ModuleId,
) {
    let func = session
        .load_function(
            module_storage,
            module_id,
            &Identifier::new("foo").unwrap(),
            &[],
        )
        .unwrap();
    let err1 = session
        .execute_entry_function(
            func,
            Vec::<Vec<u8>>::new(),
            &mut UnmeteredGasMeter,
            &mut TraversalContext::new(traversal_storage),
            module_storage,
        )
        .unwrap_err();

    assert!(err1.exec_state().unwrap().stack_trace().is_empty());

    let func = session
        .load_function(
            module_storage,
            module_id,
            &Identifier::new("foo2").unwrap(),
            &[],
        )
        .unwrap();
    let err2 = session
        .execute_entry_function(
            func,
            Vec::<Vec<u8>>::new(),
            &mut UnmeteredGasMeter,
            &mut TraversalContext::new(traversal_storage),
            module_storage,
        )
        .unwrap_err();

    assert_eq!(err2.exec_state().unwrap().stack_trace().len(), 1);
}
