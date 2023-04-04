// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use super::{
    types::{Incarnation, MVDataError, MVDataOutput, TxnIndex},
    *,
};
use aptos_aggregator::{
    delta_change_set::{delta_add, delta_sub, DeltaOp, DeltaUpdate},
    transaction::AggregatorValue,
};
use aptos_types::{
    access_path::AccessPath,
    executable::{ExecutableTestType, ModulePath},
    state_store::state_value::StateValue,
};
use std::sync::Arc;

mod proptest_types;

#[derive(Debug, PartialEq, Eq)]
struct Value(Vec<u32>);

impl TransactionWrite for Value {
    fn extract_raw_bytes(&self) -> Option<Vec<u8>> {
        let mut v: Vec<u8> = self
            .0
            .clone()
            .into_iter()
            .flat_map(|element| element.to_be_bytes())
            .collect();
        v.resize(16, 0);
        Some(v)
    }

    fn as_state_value(&self) -> Option<StateValue> {
        unimplemented!()
    }
}

// Generate a Vec deterministically based on txn_idx and incarnation.
fn value_for(txn_idx: TxnIndex, incarnation: Incarnation) -> Value {
    Value(vec![txn_idx * 5, txn_idx + incarnation, incarnation * 5])
}

// Generate the value_for txn_idx and incarnation in arc.
fn arc_value_for(txn_idx: TxnIndex, incarnation: Incarnation) -> Arc<Value> {
    // Generate a Vec deterministically based on txn_idx and incarnation.
    Arc::new(value_for(txn_idx, incarnation))
}

// Convert value for txn_idx and incarnation into u128.
fn u128_for(txn_idx: TxnIndex, incarnation: Incarnation) -> u128 {
    AggregatorValue::from_write(&value_for(txn_idx, incarnation))
        .unwrap()
        .into()
}

// Generate determinitc additions.
fn add_for(txn_idx: TxnIndex, limit: u128) -> DeltaOp {
    delta_add(txn_idx as u128, limit)
}

// Generate determinitc subtractions.
fn sub_for(txn_idx: TxnIndex, base: u128) -> DeltaOp {
    delta_sub(base + (txn_idx as u128), u128::MAX)
}

#[derive(Clone, Eq, Hash, PartialEq)]
pub(crate) struct KeyType<K: Hash + Clone + Eq>(
    /// Wrapping the types used for testing to add ModulePath trait implementation.
    pub K,
);

impl<K: Hash + Clone + Eq> ModulePath for KeyType<K> {
    fn module_path(&self) -> Option<AccessPath> {
        None
    }
}

#[test]
fn create_write_read_placeholder_struct() {
    use MVDataError::*;
    use MVDataOutput::*;

    let ap1 = KeyType(b"/foo/b".to_vec());
    let ap2 = KeyType(b"/foo/c".to_vec());
    let ap3 = KeyType(b"/foo/d".to_vec());

    let mvtbl: MVHashMap<KeyType<Vec<u8>>, Value, ExecutableTestType> = MVHashMap::new(None);

    // Reads that should go the DB return Err(NotFound)
    let r_db = mvtbl.fetch_data(&ap1, 5);
    assert_eq!(Err(NotFound), r_db);

    // Write by txn 10.
    mvtbl.write(&ap1, (10, 1), value_for(10, 1));

    // Reads that should go the DB return Err(NotFound)
    let r_db = mvtbl.fetch_data(&ap1, 9);
    assert_eq!(Err(NotFound), r_db);
    // Reads return entries from smaller txns, not txn 10.
    let r_db = mvtbl.fetch_data(&ap1, 10);
    assert_eq!(Err(NotFound), r_db);

    // Reads for a higher txn return the entry written by txn 10.
    let r_10 = mvtbl.fetch_data(&ap1, 15);
    assert_eq!(Ok(Versioned((10, 1), arc_value_for(10, 1))), r_10);

    // More deltas.
    mvtbl.add_delta(&ap1, 11, add_for(11, 1000));
    mvtbl.add_delta(&ap1, 12, add_for(12, 1000));
    mvtbl.add_delta(&ap1, 13, sub_for(13, 61));

    // Reads have to go traverse deltas until a write is found.
    let r_sum = mvtbl.fetch_data(&ap1, 14);
    assert_eq!(Ok(Resolved(u128_for(10, 1) + 11 + 12 - (61 + 13))), r_sum);

    // More writes.
    mvtbl.write(&ap1, (12, 0), value_for(12, 0));
    mvtbl.write(&ap1, (8, 3), value_for(8, 3));

    // Verify reads.
    let r_12 = mvtbl.fetch_data(&ap1, 15);
    assert_eq!(Ok(Resolved(u128_for(12, 0) - (61 + 13))), r_12);
    let r_10 = mvtbl.fetch_data(&ap1, 11);
    assert_eq!(Ok(Versioned((10, 1), arc_value_for(10, 1))), r_10);
    let r_8 = mvtbl.fetch_data(&ap1, 10);
    assert_eq!(Ok(Versioned((8, 3), arc_value_for(8, 3))), r_8);

    // Mark the entry written by 10 as an estimate.
    mvtbl.mark_estimate(&ap1, 10);

    // Read for txn 11 must observe a dependency.
    let r_10 = mvtbl.fetch_data(&ap1, 11);
    assert_eq!(Err(Dependency(10)), r_10);

    // Read for txn 12 must observe a dependency when resolving deltas at txn 11.
    let r_11 = mvtbl.fetch_data(&ap1, 12);
    assert_eq!(Err(Dependency(10)), r_11);

    // Delete the entry written by 10, write to a different ap.
    mvtbl.delete(&ap1, 10);
    mvtbl.write(&ap2, (10, 2), value_for(10, 2));

    // Read by txn 11 no longer observes entry from txn 10.
    let r_8 = mvtbl.fetch_data(&ap1, 11);
    assert_eq!(Ok(Versioned((8, 3), arc_value_for(8, 3))), r_8);

    // Reads, writes for ap2 and ap3.
    mvtbl.write(&ap2, (5, 0), value_for(5, 0));
    mvtbl.write(&ap3, (20, 4), value_for(20, 4));
    let r_5 = mvtbl.fetch_data(&ap2, 10);
    assert_eq!(Ok(Versioned((5, 0), arc_value_for(5, 0))), r_5);
    let r_20 = mvtbl.fetch_data(&ap3, 21);
    assert_eq!(Ok(Versioned((20, 4), arc_value_for(20, 4))), r_20);

    // Clear ap1 and ap3.
    mvtbl.delete(&ap1, 12);
    mvtbl.delete(&ap1, 8);
    mvtbl.delete(&ap3, 20);

    // Reads from ap1 and ap3 go to db.
    let r_db = mvtbl.fetch_data(&ap1, 30);
    match r_db {
        Err(Unresolved(delta)) => delta.get_update() == DeltaUpdate::Minus((61 + 13) - 11),
        _ => unreachable!(),
    };
    let r_db = mvtbl.fetch_data(&ap3, 30);
    assert_eq!(Err(NotFound), r_db);

    // Read entry by txn 10 at ap2.
    let r_10 = mvtbl.fetch_data(&ap2, 15);
    assert_eq!(Ok(Versioned((10, 2), arc_value_for(10, 2))), r_10);

    // Both delta-write and delta-delta application failures are detected.
    mvtbl.add_delta(&ap1, 30, add_for(30, 32));
    mvtbl.add_delta(&ap1, 31, add_for(31, 32));
    let r_33 = mvtbl.fetch_data(&ap1, 33);
    assert_eq!(Err(DeltaApplicationFailure), r_33);

    let val = value_for(10, 3);
    // sub base sub_for for which should underflow (with txn index)
    let sub_base = AggregatorValue::from_write(&val).unwrap().into();
    mvtbl.write(&ap2, (10, 3), val);
    mvtbl.add_delta(&ap2, 30, sub_for(30, sub_base));
    let r_31 = mvtbl.fetch_data(&ap2, 31);
    assert_eq!(Err(DeltaApplicationFailure), r_31);
}
