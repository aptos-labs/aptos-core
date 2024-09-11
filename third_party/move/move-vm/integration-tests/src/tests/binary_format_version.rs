// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use move_binary_format::{
    deserializer::DeserializerConfig,
    file_format::{basic_test_module, basic_test_script},
    file_format_common::{IDENTIFIER_SIZE_MAX, VERSION_MAX},
};
use move_core_types::{account_address::AccountAddress, vm_status::StatusCode};
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

        let natives = move_stdlib::natives::all_natives(
            AccountAddress::from_hex_literal("0x1").unwrap(),
            move_stdlib::natives::GasParameters::zeros(),
        );
        let runtime_environment = RuntimeEnvironment::new(natives.clone());
        let vm = MoveVM::new(natives);
        let mut sess = vm.new_session(&storage);

        if vm.vm_config().use_loader_v2 {
            let module_storage = storage.as_unsync_module_storage(&runtime_environment);
            let new_module_storage =
                StagingModuleStorage::create(m.self_addr(), &module_storage, vec![b_new
                    .clone()
                    .into()])
                .expect("New module should be publishable");
            StagingModuleStorage::create(m.self_addr(), &new_module_storage, vec![b_old
                .clone()
                .into()])
            .expect("Old module should be publishable");
        } else {
            #[allow(deprecated)]
            sess.publish_module_bundle(
                vec![b_new.clone()],
                *m.self_id().address(),
                &mut UnmeteredGasMeter,
            )
            .unwrap();

            #[allow(deprecated)]
            sess.publish_module_bundle(
                vec![b_old.clone()],
                *m.self_id().address(),
                &mut UnmeteredGasMeter,
            )
            .unwrap();
        }
    }

    // Should reject the module with newer version with max binary format version being set to VERSION_MAX - 1
    {
        let storage = InMemoryStorage::new();
        let natives = move_stdlib::natives::all_natives(
            AccountAddress::from_hex_literal("0x1").unwrap(),
            move_stdlib::natives::GasParameters::zeros(),
        );
        let vm_config = VMConfig {
            deserializer_config: DeserializerConfig::new(
                VERSION_MAX.checked_sub(1).unwrap(),
                IDENTIFIER_SIZE_MAX,
            ),
            ..Default::default()
        };
        let runtime_environment =
            RuntimeEnvironment::new_with_config(natives.clone(), vm_config.clone());
        let vm = MoveVM::new_with_config(natives, vm_config);
        let mut sess = vm.new_session(&storage);

        if vm.vm_config().use_loader_v2 {
            let module_storage = storage.as_unsync_module_storage(&runtime_environment);
            let result = StagingModuleStorage::create(m.self_addr(), &module_storage, vec![b_new
                .clone()
                .into()]);
            if let Err(err) = result {
                assert_eq!(err.major_status(), StatusCode::UNKNOWN_VERSION);
            } else {
                panic!("Module publishing should fail")
            }
            StagingModuleStorage::create(m.self_addr(), &module_storage, vec![b_old
                .clone()
                .into()])
            .unwrap();
        } else {
            #[allow(deprecated)]
            let s = sess
                .publish_module_bundle(
                    vec![b_new.clone()],
                    *m.self_id().address(),
                    &mut UnmeteredGasMeter,
                )
                .unwrap_err()
                .major_status();
            assert_eq!(s, StatusCode::UNKNOWN_VERSION);

            #[allow(deprecated)]
            sess.publish_module_bundle(
                vec![b_old.clone()],
                *m.self_id().address(),
                &mut UnmeteredGasMeter,
            )
            .unwrap();
        }
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
        let natives = move_stdlib::natives::all_natives(
            AccountAddress::from_hex_literal("0x1").unwrap(),
            move_stdlib::natives::GasParameters::zeros(),
        );
        let runtime_environment = RuntimeEnvironment::new(natives.clone());
        let vm = MoveVM::new(natives);
        let mut sess = vm.new_session(&storage);
        let code_storage = storage.as_unsync_code_storage(&runtime_environment);

        let args: Vec<Vec<u8>> = vec![];
        sess.execute_script(
            b_new.clone(),
            vec![],
            args.clone(),
            &mut UnmeteredGasMeter,
            &mut TraversalContext::new(&traversal_storage),
            &code_storage,
        )
        .unwrap();

        sess.execute_script(
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
        let storage = InMemoryStorage::new();
        let natives = move_stdlib::natives::all_natives(
            AccountAddress::from_hex_literal("0x1").unwrap(),
            move_stdlib::natives::GasParameters::zeros(),
        );
        let vm_config = VMConfig {
            deserializer_config: DeserializerConfig::new(
                VERSION_MAX.checked_sub(1).unwrap(),
                IDENTIFIER_SIZE_MAX,
            ),
            ..Default::default()
        };
        let runtime_environment =
            RuntimeEnvironment::new_with_config(natives.clone(), vm_config.clone());
        let vm = MoveVM::new_with_config(natives, vm_config);
        let mut sess = vm.new_session(&storage);
        let code_storage = storage.as_unsync_code_storage(&runtime_environment);

        let args: Vec<Vec<u8>> = vec![];
        assert_eq!(
            sess.execute_script(
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

        sess.execute_script(
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
