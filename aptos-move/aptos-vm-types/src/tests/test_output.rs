// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    output::VMOutput,
    tests::utils::{
        as_state_key, build_vm_output, mock_add, mock_create_with_layout, mock_modify,
        mock_module_modify,
    },
};
use aptos_aggregator::delta_change_set::serialize;
use aptos_transaction_simulation::{InMemoryStateStore, SimulationStateStore};
use aptos_types::{
    state_store::{state_key::StateKey, state_value::StateValue},
    transaction::TransactionOutput,
    write_set::WriteOp,
};
use claims::{assert_err, assert_matches, assert_ok};
use move_core_types::vm_status::{AbortLocation, VMStatus};
use std::collections::BTreeMap;

fn assert_eq_outputs(vm_output: &VMOutput, txn_output: TransactionOutput) {
    let vm_output_writes = &vm_output
        .concrete_write_set_iter()
        .map(|(k, v)| {
            (
                k.clone(),
                v.expect("expect only concrete write ops").clone(),
            )
        })
        .collect::<BTreeMap<StateKey, WriteOp>>();

    // A way to obtain a reference to the map inside a WriteSet.
    let mut write_set_mut = txn_output.write_set().clone().into_mut();
    let txn_output_writes = write_set_mut.as_inner_mut();

    assert_eq!(vm_output_writes, txn_output_writes);
    assert_eq!(vm_output.gas_used(), txn_output.gas_used());
    assert_eq!(vm_output.status(), txn_output.status());
}

#[test]
fn test_ok_output_equality_no_deltas() {
    let state_view = InMemoryStateStore::new();
    let vm_output = build_vm_output(
        vec![mock_create_with_layout("0", 0, None)],
        vec![mock_module_modify("1", 1)],
        vec![],
        vec![mock_modify("2", 2)],
        vec![],
    );

    // Different ways to materialize deltas:
    //   1. `try_materialize` preserves the type and returns a result.
    //   2. `try_into_transaction_output` changes the type and returns a result.
    //   3. `into_transaction_output_with_materialized_write_set` changes the type and
    //       simply merges writes for materialized deltas & combined groups.
    let mut materialized_vm_output = vm_output.clone();
    assert_ok!(materialized_vm_output.try_materialize(&state_view));
    let txn_output_1 = assert_ok!(vm_output
        .clone()
        .try_materialize_into_transaction_output(&state_view));
    let txn_output_2 = assert_ok!(vm_output
        .clone()
        .into_transaction_output_with_materialized_write_set(vec![], vec![], vec![]));

    // Because there are no deltas, we should not see any difference in write sets and
    // also all calls must succeed.
    assert_eq!(&vm_output, &materialized_vm_output);
    assert_eq_outputs(&vm_output, txn_output_1);
    assert_eq_outputs(&vm_output, txn_output_2);
}

#[test]
fn test_ok_output_equality_with_deltas() {
    let delta_key = "3";
    let state_view = InMemoryStateStore::new();
    state_view
        .set_state_value(
            as_state_key!(delta_key),
            StateValue::new_legacy(serialize(&100).into()),
        )
        .unwrap();

    let vm_output = build_vm_output(
        vec![mock_create_with_layout("0", 0, None)],
        vec![mock_module_modify("1", 1)],
        vec![],
        vec![mock_modify("2", 2)],
        vec![mock_add(delta_key, 300)],
    );

    let mut materialized_vm_output = vm_output.clone();
    assert_ok!(materialized_vm_output.try_materialize(&state_view));
    let txn_output_1 = assert_ok!(vm_output
        .clone()
        .try_materialize_into_transaction_output(&state_view));
    let txn_output_2 = vm_output
        .clone()
        .into_transaction_output_with_materialized_write_set(
            vec![mock_modify("3", 400)],
            vec![],
            vec![],
        );

    let expected_aggregator_write_set =
        BTreeMap::from([mock_modify("2", 2), mock_modify("3", 400)]);
    assert_eq!(
        materialized_vm_output.resource_write_set(),
        vm_output.resource_write_set()
    );
    assert_eq!(
        materialized_vm_output.module_write_set(),
        vm_output.module_write_set()
    );
    assert_eq!(
        materialized_vm_output.aggregator_v1_write_set(),
        &expected_aggregator_write_set
    );
    assert!(materialized_vm_output.aggregator_v1_delta_set().is_empty());
    assert_eq!(
        vm_output.fee_statement(),
        materialized_vm_output.fee_statement()
    );
    assert_eq!(vm_output.status(), materialized_vm_output.status());
    assert_eq_outputs(&materialized_vm_output, txn_output_1);
    assert_eq_outputs(&materialized_vm_output, txn_output_2.unwrap());
}

#[test]
fn test_err_output_equality_with_deltas() {
    let delta_key = "3";
    let state_view = InMemoryStateStore::new();
    state_view
        .set_state_value(
            as_state_key!(delta_key),
            StateValue::new_legacy(serialize(&900).into()),
        )
        .unwrap();

    let vm_output = build_vm_output(vec![], vec![], vec![], vec![], vec![mock_add(
        delta_key, 300,
    )]);

    let vm_status_1 = assert_err!(vm_output.clone().try_materialize(&state_view));
    let vm_status_2 = assert_err!(vm_output.try_materialize_into_transaction_output(&state_view));

    // Error should be consistent.
    assert_eq!(vm_status_1, vm_status_2);

    // Aggregator errors lead to aborts. Because an overflow happens,
    // the code must be 131073.
    assert_matches!(
        vm_status_1,
        VMStatus::MoveAbort(AbortLocation::Module(_), 131073)
    );
}
