// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    assert_success, assert_vm_status, build_package, build_package_with_compiler_version,
    MoveHarness,
};
use aptos_cached_packages::aptos_stdlib;
use aptos_framework::{
    BuildOptions, RuntimeModuleMetadata, RuntimeModuleMetadataV1, APTOS_METADATA_KEY,
    APTOS_METADATA_KEY_V1,
};
use aptos_package_builder::PackageBuilder;
use aptos_types::{
    chain_id::ChainId,
    on_chain_config::{FeatureFlag, OnChainConfig},
    transaction::{Script, TransactionPayload, TransactionStatus},
};
use move_binary_format::CompiledModule;
use move_core_types::{
    account_address::AccountAddress,
    language_storage::CORE_CODE_ADDRESS,
    metadata::Metadata,
    vm_status::{StatusCode, StatusCode::CONSTRAINT_NOT_SATISFIED},
};
use move_model::metadata::{CompilationMetadata, CompilerVersion, COMPILATION_METADATA_KEY};
use std::collections::BTreeMap;

#[test]
fn test_unknown_metadata_key() {
    let unknown_key = || {
        let metadata = Metadata {
            key: vec![1, 2, 3, 4, 5],
            value: vec![],
        };
        vec![metadata]
    };
    let result = test_metadata_with_changes(unknown_key);
    assert_vm_status!(result, StatusCode::CONSTRAINT_NOT_SATISFIED);
}

#[test]
fn test_duplicate_entries() {
    let duplicate_same_version = || {
        let metadata = Metadata {
            key: APTOS_METADATA_KEY_V1.to_vec(),
            value: bcs::to_bytes(&RuntimeModuleMetadataV1::default()).unwrap(),
        };
        vec![metadata.clone(), metadata]
    };
    let result = test_metadata_with_changes(duplicate_same_version);
    assert_vm_status!(result, StatusCode::CONSTRAINT_NOT_SATISFIED);

    let duplicate_different_version = || {
        let metadata_v1 = Metadata {
            key: APTOS_METADATA_KEY_V1.to_vec(),
            value: bcs::to_bytes(&RuntimeModuleMetadataV1::default()).unwrap(),
        };
        let metadata_v0 = Metadata {
            key: APTOS_METADATA_KEY.to_vec(),
            value: bcs::to_bytes(&RuntimeModuleMetadata {
                error_map: BTreeMap::new(),
            })
            .unwrap(),
        };
        vec![metadata_v1, metadata_v0]
    };

    let result = test_metadata_with_changes(duplicate_different_version);
    assert_vm_status!(result, StatusCode::CONSTRAINT_NOT_SATISFIED);
}

#[test]
fn test_malformed_metadata_value() {
    let invalid_value = || {
        let metadata = Metadata {
            key: APTOS_METADATA_KEY.to_vec(),
            value: vec![1, 2, 3],
        };
        vec![metadata]
    };
    let result = test_metadata_with_changes(invalid_value);
    assert_vm_status!(result, StatusCode::CONSTRAINT_NOT_SATISFIED);

    let v0_to_v1 = || {
        let metadata = Metadata {
            key: APTOS_METADATA_KEY.to_vec(),
            value: bcs::to_bytes(&RuntimeModuleMetadataV1::default()).unwrap(),
        };
        vec![metadata]
    };
    let result = test_metadata_with_changes(v0_to_v1);
    assert_vm_status!(result, StatusCode::CONSTRAINT_NOT_SATISFIED);

    let v1_to_v0 = || {
        let metadata = Metadata {
            key: APTOS_METADATA_KEY_V1.to_vec(),
            value: bcs::to_bytes(&RuntimeModuleMetadata {
                error_map: BTreeMap::new(),
            })
            .unwrap(),
        };
        vec![metadata]
    };
    let result = test_metadata_with_changes(v1_to_v0);
    assert_vm_status!(result, StatusCode::CONSTRAINT_NOT_SATISFIED);
}

fn test_metadata_with_changes(f: impl Fn() -> Vec<Metadata>) -> TransactionStatus {
    let mut h = MoveHarness::new();
    let account = h.new_account_at(AccountAddress::from_hex_literal("0xf00d").unwrap());

    let mut builder = PackageBuilder::new("Package");
    builder.add_source(
        "m.move",
        r#"
        module 0xf00d::M {
            #[view]
            fun foo(value: u64): u64 { value }
        }
        "#,
    );
    let path = builder.write_to_temp().unwrap();

    let package = build_package(path.path().to_path_buf(), BuildOptions::default())
        .expect("building package must succeed");
    let origin_code = package.extract_code();
    let mut compiled_module = CompiledModule::deserialize(&origin_code[0]).unwrap();
    let metadata = f();
    let mut invalid_code = vec![];
    compiled_module.metadata = metadata;
    compiled_module.serialize(&mut invalid_code).unwrap();

    let package_metadata = package
        .extract_metadata()
        .expect("extracting package metadata must succeed");
    h.run_transaction_payload(
        &account,
        aptos_stdlib::code_publish_package_txn(
            bcs::to_bytes(&package_metadata).expect("PackageMetadata has BCS"),
            vec![invalid_code],
        ),
    )
}

#[test]
fn test_duplicate_compilation_metadata_entries() {
    let duplicate_compilation_metatdata = || Metadata {
        key: COMPILATION_METADATA_KEY.to_vec(),
        value: bcs::to_bytes(&CompilationMetadata::default()).unwrap(),
    };
    let result = test_compilation_metadata_with_changes(
        duplicate_compilation_metatdata,
        CompilerVersion::V2_1,
    );
    assert_vm_status!(result, StatusCode::CONSTRAINT_NOT_SATISFIED);
    let result = test_compilation_metadata_with_changes(
        duplicate_compilation_metatdata,
        CompilerVersion::V1,
    );
    assert_success!(result);
}

fn test_compilation_metadata_with_changes(
    f: impl Fn() -> Metadata,
    compiler_version: CompilerVersion,
) -> TransactionStatus {
    let mut h = MoveHarness::new();
    let account = h.new_account_at(AccountAddress::from_hex_literal("0xf00d").unwrap());

    let mut builder = PackageBuilder::new("Package");
    builder.add_source(
        "m.move",
        r#"
        module 0xf00d::M {
            #[view]
            fun foo(value: u64): u64 { value }
        }
        "#,
    );
    let path = builder.write_to_temp().unwrap();

    let package = build_package_with_compiler_version(
        path.path().to_path_buf(),
        BuildOptions::default(),
        compiler_version,
    )
    .expect("building package must succeed");
    let origin_code = package.extract_code();
    let mut compiled_module = CompiledModule::deserialize(&origin_code[0]).unwrap();
    let metadata = f();
    let mut invalid_code = vec![];
    compiled_module.metadata.push(metadata);
    compiled_module.serialize(&mut invalid_code).unwrap();

    let package_metadata = package
        .extract_metadata()
        .expect("extracting package metadata must succeed");
    h.run_transaction_payload(
        &account,
        aptos_stdlib::code_publish_package_txn(
            bcs::to_bytes(&package_metadata).expect("PackageMetadata has BCS"),
            vec![invalid_code],
        ),
    )
}

fn test_compilation_metadata_internal(
    mainnet_flag: bool,
    v2_flag: bool,
    feature_enabled: bool,
) -> TransactionStatus {
    let mut h = MoveHarness::new();
    if feature_enabled {
        h.enable_features(vec![FeatureFlag::REJECT_UNSTABLE_BYTECODE], vec![]);
    } else {
        h.enable_features(vec![], vec![FeatureFlag::REJECT_UNSTABLE_BYTECODE]);
    }
    let account = h.new_account_at(AccountAddress::from_hex_literal("0xf00d").unwrap());
    let mut builder = PackageBuilder::new("Package");
    builder.add_source(
        "m.move",
        r#"
        module 0xf00d::M {
            #[view]
            fun foo(value: u64): u64 { value }
        }
        "#,
    );
    let path = builder.write_to_temp().unwrap();

    let compiler_version = if v2_flag {
        CompilerVersion::latest()
    } else {
        CompilerVersion::V1
    };
    let package = build_package_with_compiler_version(
        path.path().to_path_buf(),
        BuildOptions::default(),
        compiler_version,
    )
    .expect("building package must succeed");

    let package_metadata = package
        .extract_metadata()
        .expect("extracting package metadata must succeed");

    if mainnet_flag {
        h.set_resource(
            CORE_CODE_ADDRESS,
            ChainId::struct_tag(),
            &ChainId::mainnet().id(),
        );
        h.run_transaction_payload_mainnet(
            &account,
            aptos_stdlib::code_publish_package_txn(
                bcs::to_bytes(&package_metadata).expect("PackageMetadata has BCS"),
                package.extract_code(),
            ),
        )
    } else {
        h.run_transaction_payload(
            &account,
            aptos_stdlib::code_publish_package_txn(
                bcs::to_bytes(&package_metadata).expect("PackageMetadata has BCS"),
                package.extract_code(),
            ),
        )
    }
}

fn test_compilation_metadata_script_internal(
    mainnet_flag: bool,
    v2_flag: bool,
    feature_enabled: bool,
) -> TransactionStatus {
    let mut h = MoveHarness::new();
    if feature_enabled {
        h.enable_features(
            vec![FeatureFlag::REJECT_UNSTABLE_BYTECODE_FOR_SCRIPT],
            vec![],
        );
    } else {
        h.enable_features(vec![], vec![
            FeatureFlag::REJECT_UNSTABLE_BYTECODE_FOR_SCRIPT,
        ]);
    }
    let account = h.new_account_at(AccountAddress::from_hex_literal("0xf00d").unwrap());
    let mut builder = PackageBuilder::new("Package");
    builder.add_source(
        "m.move",
        r#"
        script {
            fun main() { }
        }
        "#,
    );
    let path = builder.write_to_temp().unwrap();

    let compiler_version = if v2_flag {
        CompilerVersion::latest()
    } else {
        CompilerVersion::V1
    };
    let package = build_package_with_compiler_version(
        path.path().to_path_buf(),
        BuildOptions::default(),
        compiler_version,
    )
    .expect("building package must succeed");

    let code = package.extract_script_code().into_iter().next().unwrap();

    let script = TransactionPayload::Script(Script::new(code, vec![], vec![]));

    if mainnet_flag {
        h.set_resource(
            CORE_CODE_ADDRESS,
            ChainId::struct_tag(),
            &ChainId::mainnet().id(),
        );
        h.run_transaction_payload_mainnet(&account, script)
    } else {
        h.run_transaction_payload(&account, script)
    }
}

#[test]
fn test_compilation_metadata_for_script() {
    let mut enable_check = true;
    // run compiler v2 code to mainnet
    assert_vm_status!(
        test_compilation_metadata_script_internal(true, true, enable_check),
        StatusCode::UNSTABLE_BYTECODE_REJECTED
    );
    // run compiler v1 code to mainnet
    assert_success!(test_compilation_metadata_script_internal(
        true,
        false,
        enable_check
    ));
    // run compiler v2 code to test
    assert_success!(test_compilation_metadata_script_internal(
        false,
        true,
        enable_check
    ));
    // run compiler v1 code to test
    assert_success!(test_compilation_metadata_script_internal(
        false,
        false,
        enable_check
    ));

    enable_check = false;
    // run compiler v2 code to mainnet
    // success because the feature flag is turned off
    assert_success!(test_compilation_metadata_script_internal(
        true,
        true,
        enable_check
    ),);
    // run compiler v1 code to mainnet
    assert_success!(test_compilation_metadata_script_internal(
        true,
        false,
        enable_check
    ));
    // run compiler v2 code to test
    // success because the feature flag is turned off
    assert_success!(test_compilation_metadata_script_internal(
        false,
        true,
        enable_check
    ),);
    // run compiler v1 code to test
    assert_success!(test_compilation_metadata_script_internal(
        false,
        false,
        enable_check
    ));
}

#[test]
fn test_compilation_metadata() {
    let mut enable_check = true;
    // publish compiler v2 code to mainnet
    assert_vm_status!(
        test_compilation_metadata_internal(true, true, enable_check),
        StatusCode::UNSTABLE_BYTECODE_REJECTED
    );
    // publish compiler v1 code to mainnet
    assert_success!(test_compilation_metadata_internal(
        true,
        false,
        enable_check
    ));
    // publish compiler v2 code to test
    assert_success!(test_compilation_metadata_internal(
        false,
        true,
        enable_check
    ));
    // publish compiler v1 code to test
    assert_success!(test_compilation_metadata_internal(
        false,
        false,
        enable_check
    ));

    enable_check = false;
    // publish compiler v2 code to mainnet
    // failed because the metadata cannot be recognized
    assert_vm_status!(
        test_compilation_metadata_internal(true, true, enable_check),
        CONSTRAINT_NOT_SATISFIED
    );
    // publish compiler v1 code to mainnet
    assert_success!(test_compilation_metadata_internal(
        true,
        false,
        enable_check
    ));
    // publish compiler v2 code to test
    // failed because the metadata cannot be recognized
    assert_vm_status!(
        test_compilation_metadata_internal(false, true, enable_check),
        CONSTRAINT_NOT_SATISFIED
    );
    // publish compiler v1 code to test
    assert_success!(test_compilation_metadata_internal(
        false,
        false,
        enable_check
    ));
}
