// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    compiler::{as_module, as_script, compile_units},
    tests::execute_script_for_test,
};
use claims::{assert_err, assert_ok};
use move_bytecode_verifier::VerifierConfig;
use move_core_types::account_address::AccountAddress;
use move_vm_runtime::{
    config::VMConfig, AsUnsyncModuleStorage, RuntimeEnvironment, StagingModuleStorage,
};
use move_vm_test_utils::InMemoryStorage;

const TEST_ADDR: AccountAddress = AccountAddress::new([42; AccountAddress::LENGTH]);

fn initialize_storage(max_loop_depth: usize) -> InMemoryStorage {
    let vm_config = VMConfig {
        verifier_config: VerifierConfig {
            max_loop_depth: Some(max_loop_depth),
            ..Default::default()
        },
        ..VMConfig::default_for_test()
    };
    let runtime_environment = RuntimeEnvironment::new_with_config(vec![], vm_config);
    InMemoryStorage::new_with_runtime_environment(runtime_environment)
}

#[test]
fn test_publish_module_with_nested_loops() {
    let code = r#"
        module {{ADDR}}::M {
            fun foo() {
                let i = 0;
                while (i < 10) {
                    let j = 0;
                    while (j < 10) {
                        j = j + 1;
                    };
                    i = i + 1;
                };
            }
        }
    "#;
    let code = code.replace("{{ADDR}}", &format!("0x{}", TEST_ADDR.to_hex()));
    let mut units = compile_units(&code).unwrap();

    let m = as_module(units.pop().unwrap());
    let mut m_blob = vec![];
    m.serialize(&mut m_blob).unwrap();

    // Should succeed with max_loop_depth = 2
    {
        let storage = initialize_storage(2);

        let module_storage = storage.as_unsync_module_storage();
        let result =
            StagingModuleStorage::create(&TEST_ADDR, &module_storage, vec![m_blob.clone().into()]);
        assert_ok!(result);
    }

    // Should fail with max_loop_depth = 1
    {
        let storage = initialize_storage(1);

        let module_storage = storage.as_unsync_module_storage();
        let result =
            StagingModuleStorage::create(&TEST_ADDR, &module_storage, vec![m_blob.clone().into()]);
        assert!(result.is_err());
    }
}

#[test]
fn test_run_script_with_nested_loops() {
    let code = r#"
        script {
            fun main() {
                let i = 0;
                while (i < 10) {
                    let j = 0;
                    while (j < 10) {
                        j = j + 1;
                    };
                    i = i + 1;
                };
            }
        }
    "#;
    let code = code.replace("{{ADDR}}", &format!("0x{}", TEST_ADDR.to_hex()));
    let mut units = compile_units(&code).unwrap();

    let s = as_script(units.pop().unwrap());
    let mut s_blob: Vec<u8> = vec![];
    s.serialize(&mut s_blob).unwrap();

    // Should succeed with max_loop_depth = 2
    {
        let storage = initialize_storage(2);
        let result = execute_script_for_test(&storage, &s_blob, &[], vec![]);
        assert_ok!(result);
    }

    // Should fail with max_loop_depth = 1
    {
        let storage = initialize_storage(1);
        let result = execute_script_for_test(&storage, &s_blob, &[], vec![]);
        assert_err!(result);
    }
}
