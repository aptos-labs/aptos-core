// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{assert_success, assert_vm_status, build_package, MoveHarness};
use aptos_cached_packages::aptos_stdlib;
use aptos_framework::BuildOptions;
use aptos_package_builder::PackageBuilder;
use aptos_types::{
    chain_id::ChainId,
    on_chain_config::OnChainConfig,
    transaction::{Script, TransactionPayload, TransactionStatus},
    vm::module_metadata::{
        RuntimeModuleMetadata, RuntimeModuleMetadataV1, APTOS_METADATA_KEY, APTOS_METADATA_KEY_V1,
    },
};
use move_binary_format::CompiledModule;
use move_core_types::{
    language_storage::CORE_CODE_ADDRESS, metadata::Metadata, vm_status::StatusCode,
};
use move_model::metadata::{CompilationMetadata, CompilerVersion, COMPILATION_METADATA_KEY};
use rstest::rstest;
use std::collections::BTreeMap;

#[rstest(
    stateless_account,
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(true, false, false),
    case(true, true, false),
    case(true, true, true),
    case(false, false, false),
    case(false, true, false),
    case(false, true, true)
)]
fn test_unknown_metadata_key(
    stateless_account: bool,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let unknown_key = || {
        let metadata = Metadata {
            key: vec![1, 2, 3, 4, 5],
            value: vec![],
        };
        vec![metadata]
    };
    let result = test_metadata_with_changes(
        unknown_key,
        stateless_account,
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    assert_vm_status!(result, StatusCode::CONSTRAINT_NOT_SATISFIED);
}

#[rstest(
    stateless_account,
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(true, false, false),
    case(true, true, false),
    case(true, true, true),
    case(false, false, false),
    case(false, true, false),
    case(false, true, true)
)]
fn test_duplicate_entries(
    stateless_account: bool,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let duplicate_same_version = || {
        let metadata = Metadata {
            key: APTOS_METADATA_KEY_V1.to_vec(),
            value: bcs::to_bytes(&RuntimeModuleMetadataV1::default()).unwrap(),
        };
        vec![metadata.clone(), metadata]
    };
    let result = test_metadata_with_changes(
        duplicate_same_version,
        stateless_account,
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
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

    let result = test_metadata_with_changes(
        duplicate_different_version,
        stateless_account,
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    assert_vm_status!(result, StatusCode::CONSTRAINT_NOT_SATISFIED);
}

#[rstest(
    stateless_account,
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(true, false, false),
    case(true, true, false),
    case(true, true, true),
    case(false, false, false),
    case(false, true, false),
    case(false, true, true)
)]
fn test_malformed_metadata_value(
    stateless_account: bool,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let invalid_value = || {
        let metadata = Metadata {
            key: APTOS_METADATA_KEY.to_vec(),
            value: vec![1, 2, 3],
        };
        vec![metadata]
    };
    let result = test_metadata_with_changes(
        invalid_value,
        stateless_account,
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    assert_vm_status!(result, StatusCode::CONSTRAINT_NOT_SATISFIED);

    let v0_to_v1 = || {
        let metadata = Metadata {
            key: APTOS_METADATA_KEY.to_vec(),
            value: bcs::to_bytes(&RuntimeModuleMetadataV1::default()).unwrap(),
        };
        vec![metadata]
    };
    let result = test_metadata_with_changes(
        v0_to_v1,
        stateless_account,
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
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
    let result = test_metadata_with_changes(
        v1_to_v0,
        stateless_account,
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    assert_vm_status!(result, StatusCode::CONSTRAINT_NOT_SATISFIED);
}

fn test_metadata_with_changes(
    f: impl Fn() -> Vec<Metadata>,
    stateless_account: bool,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) -> TransactionStatus {
    let mut h = MoveHarness::new_with_flags(use_txn_payload_v2_format, use_orderless_transactions);
    let account = h.new_account_with_key_pair(if stateless_account { None } else { Some(0) });
    let mut builder = PackageBuilder::new("Package");
    builder.add_source(
        "m.move",
        format!(
            r#"
        module {}::M {{
            #[view]
            fun foo(value: u64): u64 {{ value }}
        }}
        "#,
            account.address()
        )
        .as_str(),
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

#[rstest(
    stateless_account,
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(true, false, false),
    case(true, true, false),
    case(true, true, true),
    case(false, false, false),
    case(false, true, false),
    case(false, true, true)
)]
fn test_duplicate_compilation_metadata_entries(
    stateless_account: bool,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let duplicate_compilation_metatdata = || Metadata {
        key: COMPILATION_METADATA_KEY.to_vec(),
        value: bcs::to_bytes(&CompilationMetadata::default()).unwrap(),
    };
    let result = test_compilation_metadata_with_changes(
        duplicate_compilation_metatdata,
        CompilerVersion::latest(),
        stateless_account,
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    assert_vm_status!(result, StatusCode::CONSTRAINT_NOT_SATISFIED);
    let result = test_compilation_metadata_with_changes(
        duplicate_compilation_metatdata,
        CompilerVersion::latest_stable(),
        stateless_account,
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    assert_vm_status!(result, StatusCode::CONSTRAINT_NOT_SATISFIED);
}

fn test_compilation_metadata_with_changes(
    f: impl Fn() -> Metadata,
    compiler_version: CompilerVersion,
    stateless_account: bool,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) -> TransactionStatus {
    let mut h = MoveHarness::new_with_flags(use_txn_payload_v2_format, use_orderless_transactions);
    let account = h.new_account_with_key_pair(if stateless_account { None } else { Some(0) });
    let mut builder = PackageBuilder::new("Package");
    builder.add_source(
        "m.move",
        format!(
            r#"
        module {}::M {{
            #[view]
            fun foo(value: u64): u64 {{ value }}
        }}
        "#,
            account.address()
        )
        .as_str(),
    );
    let path = builder.write_to_temp().unwrap();

    let package = build_package(path.path().to_path_buf(), BuildOptions {
        compiler_version: Some(compiler_version),
        ..BuildOptions::default()
    })
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
    unstable_flag: bool,
    stateless_account: bool,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) -> TransactionStatus {
    let mut h = MoveHarness::new_with_flags(use_txn_payload_v2_format, use_orderless_transactions);
    let account = h.new_account_with_key_pair(if stateless_account { None } else { Some(0) });
    let mut builder = PackageBuilder::new("Package");
    builder.add_source(
        "m.move",
        format!(
            r#"
        module {}::M {{
            #[view]
            fun foo(value: u64): u64 {{ value }}
        }}
        "#,
            account.address()
        )
        .as_str(),
    );
    let path = builder.write_to_temp().unwrap();

    let compiler_version = if unstable_flag {
        CompilerVersion::latest()
    } else {
        CompilerVersion::latest_stable()
    };
    let package = build_package(path.path().to_path_buf(), BuildOptions {
        compiler_version: Some(compiler_version),
        ..BuildOptions::default()
    })
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
    unstable_flag: bool,
    stateless_account: bool,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) -> TransactionStatus {
    let mut h = MoveHarness::new_with_flags(use_txn_payload_v2_format, use_orderless_transactions);
    let account = h.new_account_with_key_pair(if stateless_account { None } else { Some(0) });
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

    let compiler_version = if unstable_flag {
        CompilerVersion::latest()
    } else {
        CompilerVersion::latest_stable()
    };
    let package = build_package(path.path().to_path_buf(), BuildOptions {
        compiler_version: Some(compiler_version),
        ..BuildOptions::default()
    })
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

#[rstest(
    stateless_account,
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(true, false, false),
    case(true, true, false),
    case(true, true, true),
    case(false, false, false),
    case(false, true, false),
    case(false, true, true)
)]
fn test_compilation_metadata_for_script(
    stateless_account: bool,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    // run unstable compiler code to mainnet
    assert_vm_status!(
        test_compilation_metadata_script_internal(
            true,
            true,
            stateless_account,
            use_txn_payload_v2_format,
            use_orderless_transactions
        ),
        StatusCode::UNSTABLE_BYTECODE_REJECTED
    );
    // run stable compiler code to mainnet
    assert_success!(test_compilation_metadata_script_internal(
        true,
        false,
        stateless_account,
        use_txn_payload_v2_format,
        use_orderless_transactions,
    ));
    // run unstable compiler code to test
    assert_success!(test_compilation_metadata_script_internal(
        false,
        true,
        stateless_account,
        use_txn_payload_v2_format,
        use_orderless_transactions,
    ));
    // run stable compiler code to test
    assert_success!(test_compilation_metadata_script_internal(
        false,
        false,
        stateless_account,
        use_txn_payload_v2_format,
        use_orderless_transactions,
    ));
}

#[rstest(
    stateless_account,
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(true, false, false),
    case(true, true, false),
    case(true, true, true),
    case(false, false, false),
    case(false, true, false),
    case(false, true, true)
)]
fn test_compilation_metadata(
    stateless_account: bool,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    // publish unstable compiler code to mainnet
    assert_vm_status!(
        test_compilation_metadata_internal(
            true,
            true,
            stateless_account,
            use_txn_payload_v2_format,
            use_orderless_transactions
        ),
        StatusCode::UNSTABLE_BYTECODE_REJECTED
    );
    // publish stable compiler code to mainnet
    assert_success!(test_compilation_metadata_internal(
        true,
        false,
        stateless_account,
        use_txn_payload_v2_format,
        use_orderless_transactions,
    ));
    // publish unstable compiler code to test
    assert_success!(test_compilation_metadata_internal(
        false,
        true,
        stateless_account,
        use_txn_payload_v2_format,
        use_orderless_transactions,
    ));
    // publish stable compiler code to test
    assert_success!(test_compilation_metadata_internal(
        false,
        false,
        stateless_account,
        use_txn_payload_v2_format,
        use_orderless_transactions,
    ));
}
