// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Tests for rejecting trailing bytes in modules being published.

use move_binary_format::file_format::basic_test_module;
use move_core_types::vm_status::StatusCode;
use move_vm_runtime::{
    config::VMConfig, AsUnsyncModuleStorage, RuntimeEnvironment, StagingModuleStorage,
};
use move_vm_test_utils::InMemoryStorage;

fn initialize_storage(reject_trailing_bytes_in_incoming_code: bool) -> InMemoryStorage {
    let vm_config = VMConfig {
        reject_trailing_bytes_in_incoming_code,
        ..VMConfig::default_for_test()
    };
    let runtime_environment = RuntimeEnvironment::new_with_config(vec![], vm_config);
    InMemoryStorage::new_with_runtime_environment(runtime_environment)
}

#[test]
fn test_publish_module_with_trailing_bytes() {
    let module = basic_test_module();
    let mut binary = vec![];
    module
        .serialize_for_version(Some(module.version), &mut binary)
        .unwrap();
    let binary_with_trailing_bytes = [binary.as_slice(), b"payload"].concat();

    let storage = initialize_storage(false);
    StagingModuleStorage::create(
        module.self_addr(),
        &storage.as_unsync_module_storage(),
        vec![binary_with_trailing_bytes.clone().into()],
    )
    .expect("Module with trailing bytes should be publishable if they are not rejected");

    let storage = initialize_storage(true);
    let module_storage = storage.as_unsync_module_storage();
    let result = StagingModuleStorage::create(module.self_addr(), &module_storage, vec![
        binary_with_trailing_bytes.into(),
    ]);
    match result {
        Ok(_) => panic!("Module with trailing bytes should not be publishable"),
        Err(err) => assert_eq!(err.major_status(), StatusCode::MALFORMED),
    }
    StagingModuleStorage::create(module.self_addr(), &module_storage, vec![binary.into()])
        .expect("Module without trailing bytes should be publishable");
}
