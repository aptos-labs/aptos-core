// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use move_binary_format::{
    deserializer::DeserializerConfig,
    file_format::{basic_test_module, basic_test_script},
    file_format_common::{IDENTIFIER_SIZE_MAX, VERSION_MAX},
};
use move_core_types::vm_status::StatusCode;
use move_vm_runtime::{
    config::VMConfig, module_traversal::*, move_vm::MoveVM, AsUnsyncCodeStorage,
    AsUnsyncModuleStorage, RuntimeEnvironment, StagingModuleStorage,
};
use move_vm_test_utils::InMemoryStorage;
use move_vm_types::gas::UnmeteredGasMeter;

#[test]
fn test_publish_module_with_custom_max_binary_format_version() {
    let m = basic_test_module();
    let mut b_new = vec![];
    let mut b_old = vec![];
    m.serialize_for_version(Some(VERSION_MAX), &mut b_new)
        .unwrap();
    m.serialize_for_version(Some(VERSION_MAX.checked_sub(1).unwrap()), &mut b_old)
        .unwrap();

    // Should accept both modules with the default settings
    {
        let storage = InMemoryStorage::new();

        let module_storage = storage.as_unsync_module_storage();
        let new_module_storage =
            StagingModuleStorage::create(m.self_addr(), &module_storage, vec![b_new
                .clone()
                .into()])
            .expect("New module should be publishable");
        StagingModuleStorage::create(m.self_addr(), &new_module_storage, vec![b_old
            .clone()
            .into()])
        .expect("Old module should be publishable");
    }

    // Should reject the module with newer version with max binary format version being set to VERSION_MAX - 1
    {
        let vm_config = VMConfig {
            deserializer_config: DeserializerConfig::new(
                VERSION_MAX.checked_sub(1).unwrap(),
                IDENTIFIER_SIZE_MAX,
            ),
            ..Default::default()
        };
        let runtime_environment = RuntimeEnvironment::new_with_config(vec![], vm_config);
        let storage = InMemoryStorage::new_with_runtime_environment(runtime_environment);

        let module_storage = storage.as_unsync_module_storage();
        let result = StagingModuleStorage::create(m.self_addr(), &module_storage, vec![b_new
            .clone()
            .into()]);
        if let Err(err) = result {
            assert_eq!(err.major_status(), StatusCode::UNKNOWN_VERSION);
        } else {
            panic!("Module publishing should fail")
        }
        StagingModuleStorage::create(m.self_addr(), &module_storage, vec![b_old.clone().into()])
            .unwrap();
    }
}

#[test]
fn test_run_script_with_custom_max_binary_format_version() {
    let s = basic_test_script();
    let mut b_new = vec![];
    let mut b_old = vec![];
    s.serialize_for_version(Some(VERSION_MAX), &mut b_new)
        .unwrap();
    s.serialize_for_version(Some(VERSION_MAX.checked_sub(1).unwrap()), &mut b_old)
        .unwrap();
    let traversal_storage = TraversalStorage::new();

    // Should accept both modules with the default settings
    {
        let storage = InMemoryStorage::new();
        let mut sess = MoveVM::new_session(&storage);
        let code_storage = storage.as_unsync_code_storage();

        let args: Vec<Vec<u8>> = vec![];
        sess.load_and_execute_script(
            b_new.clone(),
            vec![],
            args.clone(),
            &mut UnmeteredGasMeter,
            &mut TraversalContext::new(&traversal_storage),
            &code_storage,
        )
        .unwrap();

        sess.load_and_execute_script(
            b_old.clone(),
            vec![],
            args,
            &mut UnmeteredGasMeter,
            &mut TraversalContext::new(&traversal_storage),
            &code_storage,
        )
        .unwrap();
    }

    // Should reject the module with newer version with max binary format version being set to VERSION_MAX - 1
    {
        let vm_config = VMConfig {
            deserializer_config: DeserializerConfig::new(
                VERSION_MAX.checked_sub(1).unwrap(),
                IDENTIFIER_SIZE_MAX,
            ),
            ..Default::default()
        };
        let runtime_environment = RuntimeEnvironment::new_with_config(vec![], vm_config);
        let storage = InMemoryStorage::new_with_runtime_environment(runtime_environment);
        let mut sess = MoveVM::new_session(&storage);
        let code_storage = storage.as_unsync_code_storage();

        let args: Vec<Vec<u8>> = vec![];
        assert_eq!(
            sess.load_and_execute_script(
                b_new.clone(),
                vec![],
                args.clone(),
                &mut UnmeteredGasMeter,
                &mut TraversalContext::new(&traversal_storage),
                &code_storage,
            )
            .unwrap_err()
            .major_status(),
            StatusCode::CODE_DESERIALIZATION_ERROR
        );

        sess.load_and_execute_script(
            b_old.clone(),
            vec![],
            args,
            &mut UnmeteredGasMeter,
            &mut TraversalContext::new(&traversal_storage),
            &code_storage,
        )
        .unwrap();
    }
}
