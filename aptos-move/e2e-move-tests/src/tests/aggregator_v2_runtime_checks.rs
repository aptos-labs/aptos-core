// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

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

fn publish_test_package(h: &mut MoveHarness, aptos_framework_account: &Account) {
    let path_buf = common::test_dir_path("aggregator_v2.data/pack");
    assert_success!(h.publish_package_cache_building(aptos_framework_account, path_buf.as_path()));
}

fn create_test_txn(
    h: &mut MoveHarness,
    aptos_framework_account: &Account,
    name: &str,
) -> SignedTransaction {
    h.create_entry_function(
        aptos_framework_account,
        str::parse(name).unwrap(),
        vec![],
        vec![],
    )
}

// TODO[agg_v2](cleanup): deduplicate tests!

#[test]
fn test_equality() {
    let mut h = MoveHarness::new();
    let aptos_framework_account = h.aptos_framework_account();
    publish_test_package(&mut h, &aptos_framework_account);

    // Make sure aggregators are enabled, so that we can test
    h.enable_features(
        vec![
            FeatureFlag::AGGREGATOR_V2_API,
            FeatureFlag::AGGREGATOR_V2_DELAYED_FIELDS,
            FeatureFlag::RESOURCE_GROUPS_CHARGE_AS_SIZE_SUM,
        ],
        vec![],
    );

    let txns = vec![
        create_test_txn(
            &mut h,
            &aptos_framework_account,
            "0x1::runtime_checks::test_equality_with_aggregators_I",
        ),
        create_test_txn(
            &mut h,
            &aptos_framework_account,
            "0x1::runtime_checks::test_equality_with_aggregators_II",
        ),
        create_test_txn(
            &mut h,
            &aptos_framework_account,
            "0x1::runtime_checks::test_equality_with_aggregators_III",
        ),
        create_test_txn(
            &mut h,
            &aptos_framework_account,
            "0x1::runtime_checks::test_equality_with_snapshots_I",
        ),
        create_test_txn(
            &mut h,
            &aptos_framework_account,
            "0x1::runtime_checks::test_equality_with_snapshots_II",
        ),
        create_test_txn(
            &mut h,
            &aptos_framework_account,
            "0x1::runtime_checks::test_equality_with_snapshots_III",
        ),
        create_test_txn(
            &mut h,
            &aptos_framework_account,
            "0x1::runtime_checks::test_equality_with_derived_string_snapshots_I",
        ),
        create_test_txn(
            &mut h,
            &aptos_framework_account,
            "0x1::runtime_checks::test_equality_with_derived_string_snapshots_II",
        ),
        create_test_txn(
            &mut h,
            &aptos_framework_account,
            "0x1::runtime_checks::test_equality_with_derived_string_snapshots_III",
        ),
    ];

    let statuses = h.run_block(txns);
    for status in statuses.iter() {
        let status = assert_ok!(status.as_kept_status());
        assert_matches!(status, ExecutionStatus::ExecutionFailure { .. });
    }
}

#[test]
fn test_serialization() {
    let mut h = MoveHarness::new();
    let aptos_framework_account = h.aptos_framework_account();
    publish_test_package(&mut h, &aptos_framework_account);

    // Make sure aggregators are enabled, so that we can test
    h.enable_features(
        vec![
            FeatureFlag::AGGREGATOR_V2_API,
            FeatureFlag::AGGREGATOR_V2_DELAYED_FIELDS,
            FeatureFlag::RESOURCE_GROUPS_CHARGE_AS_SIZE_SUM,
        ],
        vec![],
    );

    let txns = vec![
        create_test_txn(
            &mut h,
            &aptos_framework_account,
            "0x1::runtime_checks::test_serialization_with_aggregators",
        ),
        create_test_txn(
            &mut h,
            &aptos_framework_account,
            "0x1::runtime_checks::test_serialization_with_snapshots",
        ),
        create_test_txn(
            &mut h,
            &aptos_framework_account,
            "0x1::runtime_checks::test_serialization_with_derived_string_snapshots",
        ),
    ];

    let statuses = h.run_block(txns);
    for status in statuses.iter() {
        let status = assert_ok!(status.as_kept_status());
        let bcs_location = AbortLocation::Module(ModuleId::new(
            AccountAddress::ONE,
            ident_str!("bcs").to_owned(),
        ));
        assert_eq!(status, ExecutionStatus::MoveAbort {
            location: bcs_location,
            code: NFE_BCS_SERIALIZATION_FAILURE,
            info: None,
        });
    }
}

#[test]
fn test_string_utils() {
    let mut h = MoveHarness::new();
    let aptos_framework_account = h.aptos_framework_account();
    publish_test_package(&mut h, &aptos_framework_account);

    // Make sure aggregators are enabled, so that we can test
    h.enable_features(
        vec![
            FeatureFlag::AGGREGATOR_V2_API,
            FeatureFlag::AGGREGATOR_V2_DELAYED_FIELDS,
            FeatureFlag::RESOURCE_GROUPS_CHARGE_AS_SIZE_SUM,
        ],
        vec![],
    );

    let txns = vec![
        create_test_txn(&mut h, &aptos_framework_account, "0x1::runtime_checks::test_to_string_with_aggregators"),
        create_test_txn(&mut h, &aptos_framework_account, "0x1::runtime_checks::test_to_string_with_canonical_addresses_with_aggregators"),
        create_test_txn(&mut h, &aptos_framework_account, "0x1::runtime_checks::test_to_string_with_integer_types_with_aggregators"),
        create_test_txn(&mut h, &aptos_framework_account, "0x1::runtime_checks::test_debug_string_with_aggregators"),

        create_test_txn(&mut h, &aptos_framework_account, "0x1::runtime_checks::test_to_string_with_snapshots"),
        create_test_txn(&mut h, &aptos_framework_account, "0x1::runtime_checks::test_to_string_with_canonical_addresses_with_snapshots"),
        create_test_txn(&mut h, &aptos_framework_account, "0x1::runtime_checks::test_to_string_with_integer_types_with_snapshots"),
        create_test_txn(&mut h, &aptos_framework_account, "0x1::runtime_checks::test_debug_string_with_snapshots"),

        create_test_txn(&mut h, &aptos_framework_account, "0x1::runtime_checks::test_to_string_with_derived_string_snapshots"),
        create_test_txn(&mut h, &aptos_framework_account, "0x1::runtime_checks::test_to_string_with_canonical_addresses_with_derived_string_snapshots"),
        create_test_txn(&mut h, &aptos_framework_account, "0x1::runtime_checks::test_to_string_with_integer_types_with_derived_string_snapshots"),
        create_test_txn(&mut h, &aptos_framework_account, "0x1::runtime_checks::test_debug_string_with_derived_string_snapshots"),
    ];

    let statuses = h.run_block(txns);
    for status in statuses.iter() {
        let status = assert_ok!(status.as_kept_status());
        let string_utils_id =
            ModuleId::new(AccountAddress::ONE, ident_str!("string_utils").to_owned());
        if let ExecutionStatus::MoveAbort {
            location: AbortLocation::Module(id),
            code: 3, // EUNABLE_TO_FORMAT
            info: Some(_),
        } = status
        {
            assert_eq!(id, string_utils_id.clone())
        } else {
            panic!("Should be move abort!")
        }
    }
}
