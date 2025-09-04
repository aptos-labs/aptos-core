// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{SpeculativeEvent, SpeculativeEvents};
use claims::{assert_err, assert_ok};

mod logging;
mod proptests;

impl SpeculativeEvent for () {
    fn dispatch(self) {}
}

fn check_event_lens(logs: &SpeculativeEvents<()>, len1: usize, len2: usize) {
    let events_internal_ref = logs
        .events_with_checked_length(0)
        .expect("must return event storage of length 2");
    assert_eq!(events_internal_ref.len(), 2);
    assert_eq!(events_internal_ref[0].lock().len(), len1);
    assert_eq!(events_internal_ref[1].lock().len(), len2);
}

fn init_speculative_events() -> SpeculativeEvents<()> {
    let speculative_events = SpeculativeEvents::<()>::new(2);

    // out of bounds
    assert_err!(speculative_events.record(2, ()));

    // level trace isn't enabled
    assert_ok!(speculative_events.record(0, (),));

    assert_ok!(speculative_events.record(0, (),));
    assert_ok!(speculative_events.record(0, (),));
    assert_ok!(speculative_events.record(0, (),));
    assert_ok!(speculative_events.record(0, (),));
    assert_ok!(speculative_events.record(1, (),));
    assert_ok!(speculative_events.record(0, (),));

    check_event_lens(&speculative_events, 6, 1);
    speculative_events
}

#[test]
fn test_all_clear() {
    let speculative_events = init_speculative_events();

    // Clear everything and check lengths.
    speculative_events.clear_all_events();
    check_event_lens(&speculative_events, 0, 0);
}

#[test]
fn test_txn_clear() {
    let speculative_events = init_speculative_events();

    // Out of bounds.
    assert_err!(speculative_events.clear_txn_events(3));

    // Clear only logs from transaction idx = 1.
    assert_ok!(speculative_events.clear_txn_events(1));
    check_event_lens(&speculative_events, 6, 0);

    assert_ok!(speculative_events.record(0, (),));
    assert_ok!(speculative_events.record(1, (),)); // Expected
    assert_ok!(speculative_events.record(0, (),));
    assert_ok!(speculative_events.record(0, (),));
    assert_ok!(speculative_events.record(0, (),));
    // Clear only logs from transaction idx = 0.
    assert_ok!(speculative_events.clear_txn_events(0));
    check_event_lens(&speculative_events, 0, 1);

    // Out of bounds.
    assert_err!(speculative_events.clear_txn_events(2));
}
