// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{assert_success, tests::common, MoveHarness};
use aptos_framework::BuildOptions;
use aptos_language_e2e_tests::executor::FakeExecutor;
use aptos_types::{move_utils::MemberId, transaction::ExecutionStatus};
use claims::assert_ok;
use move_core_types::{
    account_address::AccountAddress,
    ident_str,
    language_storage::ModuleId,
    vm_status::{sub_status::NFE_BCS_SERIALIZATION_FAILURE, AbortLocation},
};
use std::str::FromStr;

fn initialize(h: &mut MoveHarness) {
    let build_options = BuildOptions::move_2().set_latest_language();
    let path = common::test_dir_path("bcs.data/function-values");

    let framework_account = h.aptos_framework_account();
    let status = h.publish_package_with_options(&framework_account, path.as_path(), build_options);
    assert_success!(status);
}

#[test]
fn test_function_value_serialization() {
    let mut h = MoveHarness::new_with_executor(FakeExecutor::from_head_genesis());
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0x123").unwrap());
    initialize(&mut h);

    let status = h.run_entry_function(
        &acc,
        MemberId::from_str("0x1::bcs_function_values_test::successful_bcs_tests").unwrap(),
        vec![],
        vec![],
    );
    assert_success!(status);

    let expected_failures = [
        "failure_bcs_test_friend_function",
        "failure_bcs_test_friend_function_with_capturing",
        "failure_bcs_test_private_function",
        "failure_bcs_test_private_function_with_capturing",
        "failure_bcs_test_anonymous",
        "failure_bcs_test_anonymous_with_capturing",
    ];

    let bcs_location = AbortLocation::Module(ModuleId::new(
        AccountAddress::ONE,
        ident_str!("bcs").to_owned(),
    ));
    let expected_status = ExecutionStatus::MoveAbort {
        location: bcs_location.clone(),
        code: NFE_BCS_SERIALIZATION_FAILURE,
        info: None,
    };

    for name in expected_failures {
        let status = assert_ok!(h
            .run_entry_function(
                &acc,
                MemberId::from_str(&format!("0x1::bcs_function_values_test::{name}")).unwrap(),
                vec![],
                vec![],
            )
            .as_kept_status());
        assert_eq!(&status, &expected_status);
    }
}
