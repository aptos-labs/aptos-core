// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    materializer::Materializer,
    output::VMOutput,
    tests::utils::{
        build_vm_output, mock_create_with_layout, mock_modify_with_layout, mock_module_modify,
    },
};
use aptos_types::{
    error::PanicError, state_store::state_key::StateKey, transaction::TransactionOutput,
    write_set::WriteOp,
};
use bytes::Bytes;
use claims::assert_ok;
use move_core_types::value::MoveTypeLayout;
use std::collections::BTreeMap;

/// A materializer for outputs that contain nothing to materialize.
struct NeverMaterializer;

impl Materializer for NeverMaterializer {
    type Key = StateKey;

    fn replace_identifiers_with_values(
        &self,
        _bytes: &[u8],
        _layout: &MoveTypeLayout,
    ) -> Result<Bytes, PanicError> {
        unreachable!("No delayed fields to materialize")
    }

    fn serialized_group_bytes(&self, _key: &StateKey) -> Result<Bytes, PanicError> {
        unreachable!("No groups to materialize")
    }

    fn exchanged_read_bytes(&self, _key: &StateKey) -> Result<Bytes, PanicError> {
        unreachable!("No exchanged reads to materialize")
    }
}

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
    let mut write_set_mut = txn_output.write_set().clone().into_value_writes();
    let txn_output_writes = write_set_mut.as_inner_mut();

    assert_eq!(vm_output_writes, txn_output_writes);
    assert_eq!(vm_output.gas_used(), txn_output.gas_used());
    assert_eq!(vm_output.status(), txn_output.status());
}

#[test]
fn test_ok_output_equality() {
    let vm_output = build_vm_output(
        vec![
            mock_create_with_layout("0", 0, None),
            mock_modify_with_layout("2", 2, None),
        ],
        vec![mock_module_modify("1", 1)],
        vec![],
    );

    // Two ways to construct the transaction output, which must agree when there are no
    // delayed fields to materialize:
    //   1. `into_transaction_output` just converts the output.
    //   2. `into_transaction_output_materialized` patches delayed-field writes
    //      and groups in place (none here).
    let txn_output_1 = assert_ok!(vm_output.clone().into_transaction_output());
    let txn_output_2 = assert_ok!(vm_output
        .clone()
        .into_transaction_output_materialized(&NeverMaterializer));

    assert_eq_outputs(&vm_output, txn_output_1);
    assert_eq_outputs(&vm_output, txn_output_2);
}
