// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{assert_abort, assert_success, tests::common, MoveHarness};
use velor_types::{
    account_address::AccountAddress,
    transaction::{ExecutionStatus, TransactionStatus},
};
use move_core_types::vm_status::StatusCode;
use test_case::test_case;

#[test_case(true)]
#[test_case(false)]
fn type_too_large(enable_lazy_loading: bool) {
    let mut h = MoveHarness::new_with_lazy_loading(enable_lazy_loading);

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

    // With lazy loading, layout construction errors with too many type nodes and the error is
    // propagated. Without lazy loading, the error happens inside the serializer and is remapped
    // to serialization failure error code (legacy behaviour).
    if enable_lazy_loading {
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
