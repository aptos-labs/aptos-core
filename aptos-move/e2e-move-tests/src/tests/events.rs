// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{assert_success, tests::common, MoveHarness};
use aptos_framework::natives::event::ECANNOT_CREATE_EVENT;
use aptos_types::{move_utils::MemberId, transaction::ExecutionStatus};
use claims::assert_ok;
use move_core_types::account_address::AccountAddress;
use std::str::FromStr;

#[test]
fn test_events_ty_tag_size_too_large() {
    let mut h = MoveHarness::new();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0x815").unwrap());

    assert_success!(
        h.publish_package_cache_building(&acc, &common::test_dir_path("events.data/pack"))
    );

    assert_success!(h.run_entry_function(
        &acc,
        MemberId::from_str("0x815::test_module::emit_event_v1_ok").unwrap(),
        vec![],
        vec![]
    ));
    let status = h.run_entry_function(
        &acc,
        MemberId::from_str("0x815::test_module::emit_event_v1_too_large").unwrap(),
        vec![],
        vec![],
    );
    let execution_status = assert_ok!(status.as_kept_status());
    assert!(matches!(execution_status, ExecutionStatus::MoveAbort {
        code: ECANNOT_CREATE_EVENT,
        ..
    }));

    assert_success!(h.run_entry_function(
        &acc,
        MemberId::from_str("0x815::test_module::emit_event_v2_ok").unwrap(),
        vec![],
        vec![]
    ));
    let status = h.run_entry_function(
        &acc,
        MemberId::from_str("0x815::test_module::emit_event_v2_too_large").unwrap(),
        vec![],
        vec![],
    );
    let execution_status = assert_ok!(status.as_kept_status());
    assert!(matches!(execution_status, ExecutionStatus::MoveAbort {
        code: ECANNOT_CREATE_EVENT,
        ..
    }));
}
