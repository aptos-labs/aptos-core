// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{assert_abort, assert_success, tests::common, MoveHarness};
use aptos_types::{
    account_address::AccountAddress,
    transaction::{ExecutionStatus, TransactionStatus},
};
use move_core_types::vm_status::StatusCode;
use test_case::test_case;
use aptos_gas_schedule::gas_feature_versions::RELEASE_V1_40;

#[test_case(true, false)]
#[test_case(true, true)]
#[test_case(false, true)]
#[test_case(false, false)]
fn type_too_large(enable_lazy_loading: bool, enable_layout_sharing: bool) {
    let mut h = MoveHarness::new_with_lazy_loading(enable_lazy_loading);
    if !enable_layout_sharing {
        // reset to before 1.41 when we introduced layout sharing
        h.modify_gas_schedule_raw(|s| s.feature_version = RELEASE_V1_40)
    }

    // Load the code
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xbeef").unwrap());
    assert_success!(h.publish_package(
        &acc,
        &common::test_dir_path("type_too_large.data/type_too_large"),
    ));

    let result = h.run_entry_function(
        &acc,
        str::parse("0xbeef::test::run").unwrap(),
        vec![],
        vec![],
    );

    // With layout_sharing enabled, we should succeed. Otherwise,
    // with lazy loading, layout construction errors with too many type nodes and the error is
    // propagated. Without lazy loading, the error happens inside the serializer and is remapped
    // to serialization failure error code (legacy behaviour).
    if enable_layout_sharing {
        assert_success!(result)
    } else if enable_lazy_loading {
        assert!(matches!(
            result,
            TransactionStatus::Keep(ExecutionStatus::MiscellaneousError(Some(
                StatusCode::TOO_MANY_TYPE_NODES
            )))
        ));
    } else {
        assert_abort!(result, 0x1C5);
    }
}
