// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Note[Orderless]: Done
use crate::{assert_success, tests::common, MoveHarness};
use aptos_language_e2e_tests::account::Account;
use aptos_types::{
    on_chain_config::FeatureFlag,
    transaction::{ExecutionStatus, SignedTransaction},
};
use claims::{assert_matches, assert_ok};
use move_core_types::{
    account_address::AccountAddress,
    ident_str,
    language_storage::ModuleId,
    vm_status::{sub_status::NFE_BCS_SERIALIZATION_FAILURE, AbortLocation},
};
use rstest::rstest;

fn publish_test_package(h: &mut MoveHarness, aptos_framework_account: &Account) {
    let path_buf = common::test_dir_path("aggregator_v2.data/pack");
    assert_success!(h.publish_package_cache_building(aptos_framework_account, path_buf.as_path()));
}

fn create_test_txn(h: &mut MoveHarness, acc: &Account, name: &str) -> SignedTransaction {
    h.create_entry_function(acc, str::parse(name).unwrap(), vec![], vec![])
}

fn run_entry_functions<F: Fn(ExecutionStatus)>(
    func_names: Vec<&str>,
    check_status: F,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut h = MoveHarness::new_with_flags(use_txn_payload_v2_format, use_orderless_transactions);
    let aptos_framework_account =
        h.new_account_at(AccountAddress::from_hex_literal("0x1").unwrap(), Some(0));
    publish_test_package(&mut h, &aptos_framework_account);

    // Make sure aggregators are enabled, so that we can test
    h.enable_features(
        vec![
            FeatureFlag::AGGREGATOR_V2_API,
            FeatureFlag::AGGREGATOR_V2_DELAYED_FIELDS,
            FeatureFlag::RESOURCE_GROUPS_SPLIT_IN_VM_CHANGE_SET,
        ],
        vec![],
    );

    let txns = func_names
        .into_iter()
        .map(|name| create_test_txn(&mut h, &aptos_framework_account, name))
        .collect();

    let statuses = h.run_block(txns);
    for status in statuses.iter() {
        let execution_status = assert_ok!(status.as_kept_status());
        check_status(execution_status);
    }
}

#[rstest(
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(false, false),
    case(true, false),
    case(true, true)
)]
fn test_equality(use_txn_payload_v2_format: bool, use_orderless_transactions: bool) {
    let func_names = vec![
        // Aggregators.
        "0x1::runtime_checks::test_equality_with_aggregators_I",
        "0x1::runtime_checks::test_equality_with_aggregators_II",
        "0x1::runtime_checks::test_equality_with_aggregators_III",
        // Snapshots.
        "0x1::runtime_checks::test_equality_with_snapshots_I",
        "0x1::runtime_checks::test_equality_with_snapshots_II",
        "0x1::runtime_checks::test_equality_with_snapshots_III",
        // Derived string snapshots.
        "0x1::runtime_checks::test_equality_with_derived_string_snapshots_I",
        "0x1::runtime_checks::test_equality_with_derived_string_snapshots_II",
        "0x1::runtime_checks::test_equality_with_derived_string_snapshots_III",
    ];
    run_entry_functions(
        func_names,
        |status: ExecutionStatus| {
            assert_matches!(status, ExecutionStatus::ExecutionFailure { .. });
        },
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
}

#[rstest(
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(false, false),
    case(true, false),
    case(true, true)
)]
fn test_serialization(use_txn_payload_v2_format: bool, use_orderless_transactions: bool) {
    let func_names = vec![
        "0x1::runtime_checks::test_serialization_with_aggregators",
        "0x1::runtime_checks::test_serialization_with_snapshots",
        "0x1::runtime_checks::test_serialization_with_derived_string_snapshots",
    ];
    let bcs_location = AbortLocation::Module(ModuleId::new(
        AccountAddress::ONE,
        ident_str!("bcs").to_owned(),
    ));
    run_entry_functions(
        func_names,
        |status: ExecutionStatus| {
            assert_eq!(status, ExecutionStatus::MoveAbort {
                location: bcs_location.clone(),
                code: NFE_BCS_SERIALIZATION_FAILURE,
                info: None,
            });
        },
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
}

#[rstest(
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(false, false),
    case(true, false),
    case(true, true)
)]
fn test_serialized_size(use_txn_payload_v2_format: bool, use_orderless_transactions: bool) {
    let func_names = vec![
        "0x1::runtime_checks::test_serialized_size_with_aggregators",
        "0x1::runtime_checks::test_serialized_size_with_snapshots",
        "0x1::runtime_checks::test_serialized_size_with_derived_string_snapshots",
    ];

    // Serialized size of delayed values is deterministic and fixed, so running
    // these functions should succeed, unlike regular serialization.
    run_entry_functions(
        func_names,
        |status: ExecutionStatus| {
            assert_eq!(status, ExecutionStatus::Success);
        },
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
}

#[rstest(
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(false, false),
    case(true, false),
    case(true, true)
)]
fn test_string_utils(use_txn_payload_v2_format: bool, use_orderless_transactions: bool) {
    let func_names = vec![
        // Aggregators.
        "0x1::runtime_checks::test_to_string_with_aggregators",
        "0x1::runtime_checks::test_to_string_with_canonical_addresses_with_aggregators",
        "0x1::runtime_checks::test_to_string_with_integer_types_with_aggregators",
        "0x1::runtime_checks::test_debug_string_with_aggregators",
        // Snapshots.
        "0x1::runtime_checks::test_to_string_with_snapshots",
        "0x1::runtime_checks::test_to_string_with_canonical_addresses_with_snapshots",
        "0x1::runtime_checks::test_to_string_with_integer_types_with_snapshots",
        "0x1::runtime_checks::test_debug_string_with_snapshots",
        // Derived string snapshots.
        "0x1::runtime_checks::test_to_string_with_derived_string_snapshots",
        "0x1::runtime_checks::test_to_string_with_canonical_addresses_with_derived_string_snapshots",
        "0x1::runtime_checks::test_to_string_with_integer_types_with_derived_string_snapshots",
        "0x1::runtime_checks::test_debug_string_with_derived_string_snapshots",
    ];

    let string_utils_id = ModuleId::new(AccountAddress::ONE, ident_str!("string_utils").to_owned());
    run_entry_functions(
        func_names,
        |status: ExecutionStatus| {
            if let ExecutionStatus::MoveAbort {
                location: AbortLocation::Module(id),
                code: 3, // EUNABLE_TO_FORMAT_DELAYED_FIELD
                info: Some(_),
            } = status
            {
                assert_eq!(id, string_utils_id.clone())
            } else {
                unreachable!("Expected Move abort, got {:?}", status)
            }
        },
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
}
