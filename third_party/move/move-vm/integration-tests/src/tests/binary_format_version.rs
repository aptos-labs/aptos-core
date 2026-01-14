// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::tests::execute_script_for_test;
use claims::assert_ok;
use move_binary_format::{
    deserializer::DeserializerConfig,
    file_format::{basic_test_module, basic_test_script},
    file_format_common::{IDENTIFIER_SIZE_MAX, VERSION_MAX},
};
use move_core_types::vm_status::StatusCode;
use move_vm_runtime::{
    config::VMConfig, AsUnsyncModuleStorage, RuntimeEnvironment, StagingModuleStorage,
};
use move_vm_test_utils::InMemoryStorage;

fn initialize_storage_with_binary_format_version(binary_format_version: u32) -> InMemoryStorage {
    let vm_config = VMConfig {
        deserializer_config: DeserializerConfig::new(binary_format_version, IDENTIFIER_SIZE_MAX),
        ..VMConfig::default_for_test()
    };
    let runtime_environment = RuntimeEnvironment::new_with_config(vec![], vm_config);
    InMemoryStorage::new_with_runtime_environment(runtime_environment)
}

#[test]
fn test_publish_module_with_custom_max_binary_format_version() {
    let m = basic_test_module();

    let new_version = VERSION_MAX;
    let mut b_new = vec![];
    m.serialize_for_version(Some(new_version), &mut b_new)
        .unwrap();

    let old_version = new_version - 1;
    let mut b_old = vec![];
    m.serialize_for_version(Some(old_version), &mut b_old)
        .unwrap();

    // Should accept both modules with the default settings
    {
        let storage = initialize_storage_with_binary_format_version(new_version);
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
        let storage = initialize_storage_with_binary_format_version(old_version);
        let module_storage = storage.as_unsync_module_storage();

        let result_new = StagingModuleStorage::create(m.self_addr(), &module_storage, vec![b_new
            .clone()
            .into()]);
        if let Err(err) = result_new {
            assert_eq!(err.major_status(), StatusCode::UNKNOWN_VERSION);
        } else {
            panic!("New module should not be publishable")
        }
        StagingModuleStorage::create(m.self_addr(), &module_storage, vec![b_old.clone().into()])
            .expect("Old module should be publishable");
    }
}

#[test]
fn test_run_script_with_custom_max_binary_format_version() {
    let s = basic_test_script();

    let new_version = VERSION_MAX;
    let mut b_new = vec![];
    s.serialize_for_version(Some(new_version), &mut b_new)
        .unwrap();

    let old_version = new_version - 1;
    let mut b_old = vec![];
    s.serialize_for_version(Some(old_version), &mut b_old)
        .unwrap();

    // Should accept both modules with the default settings.
    {
        let storage = initialize_storage_with_binary_format_version(new_version);
        let result_new = execute_script_for_test(&storage, &b_new, &[], vec![]);
        let result_old = execute_script_for_test(&storage, &b_old, &[], vec![]);
        assert!(result_new.is_ok() && result_old.is_ok());
    }

    // Should reject the module with newer version with max binary format version being set to the
    // smaller one.
    {
        let storage = initialize_storage_with_binary_format_version(old_version);
        let status_new = execute_script_for_test(&storage, &b_new, &[], vec![])
            .unwrap_err()
            .major_status();
        assert_eq!(status_new, StatusCode::CODE_DESERIALIZATION_ERROR);

        let result_old = execute_script_for_test(&storage, &b_old, &[], vec![]);
        assert_ok!(result_old);
    }
}
