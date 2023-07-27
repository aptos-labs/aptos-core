// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::tests::utils::{build_vm_output, create, key, modify};
use aptos_aggregator::delta_change_set::{delta_add, serialize};
use aptos_language_e2e_tests::data_store::FakeDataStore;
use aptos_types::write_set::WriteSetMut;
use claims::{assert_err, assert_matches, assert_ok};
use move_core_types::vm_status::{AbortLocation, VMStatus};
use std::collections::BTreeMap;

#[test]
fn test_ok_output_equality_no_deltas() {
    let state_view = FakeDataStore::default();

    // Suppose transaction has the following write set:
    //   create 0
    //   modify 1
    // and has no deltas.
    // Then construct the VMOutput.
    let write_set = vec![(key(0), create(0)), (key(1), modify(1))];
    let output = build_vm_output(write_set, vec![]);

    // Different ways to materialize deltas:
    //   1. `try_materialize` preserves the type and returns a result.
    //   2. `into_transaction_output` changes the type and returns a result.
    //   3. `output_with_delta_writes` changes the type and simply merges delta sets.
    // Because there are no deltas, we should not see any difference in write sets and
    // also all calls must succeed.
    let vm_output = assert_ok!(output.clone().try_materialize(&state_view));
    let txn_output_1 = assert_ok!(output.clone().into_transaction_output(&state_view));
    let txn_output_2 = output.clone().output_with_delta_writes(vec![]);

    // Check the output of `try_materialize`.
    assert!(vm_output.change_set().aggregator_delta_set().is_empty());
    assert_eq!(
        vm_output.change_set().aggregator_write_set(),
        output.change_set().aggregator_write_set()
    );
    assert_eq!(vm_output.gas_used(), output.gas_used());
    assert_eq!(vm_output.status(), output.status());

    // Check the output of `into_transaction_output`.
    assert_eq!(
        txn_output_1.write_set().into_iter().count(),
        output.change_set().write_set_iter().count()
    );
    assert!(txn_output_1
        .write_set()
        .iter()
        .zip(output.change_set().write_set_iter())
        .all(|(a, b)| a.eq(&b)));
    assert_eq!(txn_output_1.gas_used(), output.gas_used());
    assert_eq!(txn_output_1.status(), output.status());

    // Check the output of `output_with_delta_writes`.
    assert_eq!(
        txn_output_2.write_set().into_iter().count(),
        output.change_set().write_set_iter().count()
    );
    assert!(txn_output_2
        .write_set()
        .iter()
        .zip(output.change_set().write_set_iter())
        .all(|(a, b)| a.eq(&b)));
    assert_eq!(txn_output_2.gas_used(), output.gas_used());
    assert_eq!(txn_output_2.status(), output.status());
}

#[test]
fn test_ok_output_equality_with_deltas() {
    // Ensure that we have something (30 to be precise) stored at key 1.
    let mut state_view = FakeDataStore::default();
    state_view.set_legacy(key(1), serialize(&30));

    // This transaction has the following write set:
    //   create 0
    // and the following delta set:
    //   add 20
    let write_set = vec![(key(0), create(0))];
    let delta_set = vec![(key(1), delta_add(20, 100))];

    // Construct the VMOutput.
    let output = build_vm_output(write_set, delta_set);

    // Again, we test three different ways to materialize deltas. Here, we
    // has a single delta which when materialized turns into 30 + 20 = 50.
    let vm_output = assert_ok!(output.clone().try_materialize(&state_view));
    let txn_output_1 = assert_ok!(output.clone().into_transaction_output(&state_view));
    let txn_output_2 = output
        .clone()
        .output_with_delta_writes(vec![(key(1), modify(50))]);

    // Due to materialization, the write set should become:
    // This transaction has the following write set:
    //   create 0
    //   modify 50
    let expected_changes = vec![(key(0), create(0)), (key(1), modify(50))];
    let expected_write_set = WriteSetMut::new(expected_changes.clone()).freeze().unwrap();

    // Check the output of `try_materialize`. Note that all deltas have to
    // be removed.
    assert!(vm_output.change_set().aggregator_delta_set().is_empty());
    assert_eq!(
        vm_output.change_set().aggregator_write_set(),
        &BTreeMap::from_iter(expected_changes)
    );
    assert_eq!(vm_output.gas_used(), output.gas_used());
    assert_eq!(vm_output.status(), output.status());

    // Check the output of `into_transaction_output`.
    assert_eq!(txn_output_1.write_set(), &expected_write_set);
    assert_eq!(txn_output_1.gas_used(), output.gas_used());
    assert_eq!(txn_output_1.status(), output.status());

    // Check the output of `output_with_delta_writes`.
    assert_eq!(txn_output_2.write_set(), &expected_write_set);
    assert_eq!(txn_output_2.gas_used(), output.gas_used());
    assert_eq!(txn_output_2.status(), output.status());
}

#[test]
fn test_err_output_equality_with_deltas() {
    // Make sure that state view has a large enough value which overflows
    // on delta materialization. Here we use the value of 90.
    let mut state_view = FakeDataStore::default();
    state_view.set_legacy(key(1), serialize(&90));

    // This transaction has the following write set:
    //   create 0
    // and the following delta set:
    //   add 20
    // Note that the last delta overflows when added to 90.
    let write_set = vec![(key(0), create(0))];
    let delta_set = vec![(key(1), delta_add(20, 100))];

    // Construct the VMOutput.
    let output = build_vm_output(write_set, delta_set);

    // Testing `output_with_delta_writes` doesn't make sense here because
    // when delta writes are constructed the error is caught.
    let vm_status_1 = assert_err!(output.clone().try_materialize(&state_view));
    let vm_status_2 = assert_err!(output.into_transaction_output(&state_view));

    // Error should be consistent.
    assert_eq!(vm_status_1, vm_status_2);

    // Aggregator errors lead to aborts. Because an overflow happens,
    // the code must be 131073.
    assert_matches!(
        vm_status_1,
        VMStatus::MoveAbort(AbortLocation::Module(_), 131073)
    );
}
