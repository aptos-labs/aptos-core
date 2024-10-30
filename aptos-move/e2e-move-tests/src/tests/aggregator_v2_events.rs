// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Note[Orderless]: Done
use crate::{
    aggregator_v2::AggV2TestHarness,
    tests::aggregator_v2::{AggregatorMode, EAGGREGATOR_OVERFLOW},
    BlockSplit, SUCCESS,
};
use aptos_language_e2e_tests::{account::Account, executor::ExecutorMode};
use aptos_types::transaction::SignedTransaction;
use claims::{assert_none, assert_ok, assert_some};
use move_core_types::{language_storage::TypeTag, parser::parse_struct_tag};
use rstest::rstest;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Deserialize, Serialize)]
struct Aggregator {
    value: u64,
    max_value: u64,
}

#[derive(Deserialize, Serialize)]
struct Counter {
    value: Aggregator,
}

macro_rules! assert_counter_value_eq {
    ($h:ident, $v:expr) => {
        let c = assert_some!($h.harness.read_resource::<Counter>(
            $h.account.address(),
            parse_struct_tag(&format!(
                "{}::events_with_aggregators::Counter",
                $h.account.address()
            ))
            .unwrap(),
        ));
        assert_eq!(c.value.value, $v);
    };
}

#[derive(Deserialize, Serialize)]
struct AggregatorSnapshot {
    value: u64,
}

#[derive(Deserialize, Serialize)]
struct Event {
    value: AggregatorSnapshot,
}

macro_rules! assert_event_value_eq {
    ($ce:expr, $v:expr) => {
        let event = assert_ok!(bcs::from_bytes::<Event>($ce.event_data()));
        assert_eq!(event.value.value, $v);
    };
}

fn create_test_txn(h: &mut AggV2TestHarness, account: &Account, name: &str) -> SignedTransaction {
    h.harness
        .create_entry_function(account, str::parse(name).unwrap(), vec![], vec![
            bcs::to_bytes(account.address()).unwrap(),
        ])
}

fn run(
    data: Vec<(u64, String, Option<u64>)>,
    stateless_account: bool,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) -> AggV2TestHarness {
    let mut h = crate::tests::aggregator_v2::setup(
        ExecutorMode::BothComparison,
        AggregatorMode::BothComparison,
        data.len(),
        stateless_account,
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );

    let account = h.account.clone();
    let (txns, event_values): (_, Vec<_>) = data
        .into_iter()
        .map(|(status_code, name, event_value)| {
            (
                (
                    status_code,
                    create_test_txn(
                        &mut h,
                        &account,
                        &format!("{}::{}", account.address(), name),
                    ),
                ),
                event_value,
            )
        })
        .unzip();

    let outputs = h.run_block_in_parts_and_check(BlockSplit::Whole, txns);

    let event_v1_tag = assert_ok!(TypeTag::from_str(&format!(
        "{}::events_with_aggregators::EventV1",
        account.address()
    )));
    let event_v2_tag = assert_ok!(TypeTag::from_str(&format!(
        "{}::events_with_aggregators::EventV2",
        account.address()
    )));

    outputs
        .into_iter()
        .zip(event_values)
        .for_each(|(output, event_value)| {
            let (_write_set, events) = output.into();
            let mut events: Vec<_> = events
                .into_iter()
                .filter(|e| e.type_tag() == &event_v1_tag || e.type_tag() == &event_v2_tag)
                .collect();
            if events.is_empty() {
                assert_none!(event_value);
            } else {
                assert_eq!(events.len(), 1);
                let value = assert_some!(event_value);
                assert_event_value_eq!(events.pop().unwrap(), value);
            }
        });

    h
}

macro_rules! increment_counter {
    () => {
        (
            SUCCESS,
            "events_with_aggregators::increment_counter".to_string(),
            None,
        )
    };
}

macro_rules! emit_event {
    ($v:expr, $w:expr) => {
        (
            SUCCESS,
            format!("events_with_aggregators::test_emit_event_v{}", $v),
            $w,
        )
    };
}

macro_rules! increment_counter_emit_event {
    ($s:expr, $v:expr, $w:expr) => {
        (
            $s,
            format!(
                "events_with_aggregators::test_increment_counter_and_emit_event_v{}",
                $v
            ),
            $w,
        )
    };
}

#[rstest(
    event_version,
    stateless_account,
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(1, true, false, false),
    case(1, true, true, false),
    case(1, true, true, true),
    case(1, false, false, false),
    case(1, false, true, false),
    case(1, false, true, true),
    case(2, true, false, false),
    case(2, true, true, false),
    case(2, true, true, true),
    case(2, false, false, false),
    case(2, false, true, false),
    case(2, false, true, true)
)]
fn test_events_with_snapshots(
    event_version: u64,
    stateless_account: bool,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let data = vec![
        increment_counter!(),
        increment_counter!(),
        emit_event!(event_version, Some(2)),
        increment_counter!(),
        increment_counter!(),
        increment_counter_emit_event!(SUCCESS, event_version, Some(5)),
        increment_counter!(),
        emit_event!(event_version, Some(6)),
        increment_counter!(),
        increment_counter!(),
    ];
    let h = run(
        data,
        stateless_account,
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    assert_counter_value_eq!(h, 8);
}

#[rstest(
    event_version,
    stateless_account,
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(1, true, false, false),
    case(1, true, true, false),
    case(1, true, true, true),
    case(1, false, false, false),
    case(1, false, true, false),
    case(1, false, true, true),
    case(2, true, false, false),
    case(2, true, true, false),
    case(2, true, true, true),
    case(2, false, false, false),
    case(2, false, true, false),
    case(2, false, true, true)
)]
fn test_events_with_snapshots_not_emitted_on_abort(
    event_version: u64,
    stateless_account: bool,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let data = vec![
        increment_counter!(),
        increment_counter!(),
        increment_counter!(),
        increment_counter!(),
        increment_counter!(),
        increment_counter!(),
        increment_counter!(),
        increment_counter!(),
        increment_counter!(),
        increment_counter!(),
        increment_counter_emit_event!(EAGGREGATOR_OVERFLOW, event_version, None),
    ];

    let h = run(
        data,
        stateless_account,
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    assert_counter_value_eq!(h, 10);
}
