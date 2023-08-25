// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use super::{
    types::{Incarnation, MVDataError, MVDataOutput, TxnIndex},
    unsync_map::UnsyncMap,
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
use claims::{assert_err_eq, assert_none, assert_ok_eq, assert_some_eq};
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

fn match_unresolved(
    read_result: anyhow::Result<MVDataOutput<Value>, MVDataError>,
    update: DeltaUpdate,
) {
    match read_result {
        Err(MVDataError::Unresolved(delta)) => assert_eq!(delta.get_update(), update),
        _ => unreachable!(),
    };
}

#[derive(Clone, Eq, Hash, PartialEq, Debug)]
pub(crate) struct KeyType<K: Hash + Clone + Debug + Eq>(
    /// Wrapping the types used for testing to add ModulePath trait implementation.
    pub K,
);

impl<K: Hash + Clone + Eq + Debug> ModulePath for KeyType<K> {
    fn module_path(&self) -> Option<AccessPath> {
        None
    }
}

#[test]
fn unsync_map_data_basic() {
    let map: UnsyncMap<KeyType<Vec<u8>>, Value, ExecutableTestType> = UnsyncMap::new();

    let ap = KeyType(b"/foo/b".to_vec());

    // Reads that should go the DB return None
    assert_none!(map.fetch_data(&ap));
    // Ensure write registers the new value.
    map.write(ap.clone(), value_for(10, 1));
    assert_some_eq!(map.fetch_data(&ap), arc_value_for(10, 1));
    // Ensure the next write overwrites the value.
    map.write(ap.clone(), value_for(14, 1));
    assert_some_eq!(map.fetch_data(&ap), arc_value_for(14, 1));
}

#[test]
fn create_write_read_placeholder_struct() {
    use MVDataError::*;
    use MVDataOutput::*;

    let ap1 = KeyType(b"/foo/b".to_vec());
    let ap2 = KeyType(b"/foo/c".to_vec());
    let ap3 = KeyType(b"/foo/d".to_vec());

    let mvtbl: MVHashMap<KeyType<Vec<u8>>, Value, ExecutableTestType> = MVHashMap::new();

    // Reads that should go the DB return Err(NotFound)
    let r_db = mvtbl.fetch_data(&ap1, 5);
    assert_eq!(Err(NotFound), r_db);

    // Write by txn 10.
    mvtbl.data().write(ap1.clone(), (10, 1), value_for(10, 1));

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
    mvtbl.add_delta(ap1.clone(), 11, delta_add(11, u128::MAX));
    mvtbl.add_delta(ap1.clone(), 12, delta_add(12, u128::MAX));
    mvtbl.add_delta(ap1.clone(), 13, delta_sub(74, u128::MAX));

    // Reads have to go traverse deltas until a write is found.
    let r_sum = mvtbl.fetch_data(&ap1, 14);
    assert_eq!(Ok(Resolved(u128_for(10, 1) + 11 + 12 - (61 + 13))), r_sum);

    // More writes.
    mvtbl.data().write(ap1.clone(), (12, 0), value_for(12, 0));
    mvtbl.data().write(ap1.clone(), (8, 3), value_for(8, 3));

    // Verify reads.
    let r_12 = mvtbl.fetch_data(&ap1, 15);
    assert_eq!(Ok(Resolved(u128_for(12, 0) - (61 + 13))), r_12);
    let r_10 = mvtbl.fetch_data(&ap1, 11);
    assert_eq!(Ok(Versioned((10, 1), arc_value_for(10, 1))), r_10);
    let r_8 = mvtbl.fetch_data(&ap1, 10);
    assert_eq!(Ok(Versioned((8, 3), arc_value_for(8, 3))), r_8);

    // Mark the entry written by 10 as an estimate.
    mvtbl.data().mark_estimate(&ap1, 10);

    // Read for txn 11 must observe a dependency.
    let r_10 = mvtbl.fetch_data(&ap1, 11);
    assert_eq!(Err(Dependency(10)), r_10);

    // Read for txn 12 must observe a dependency when resolving deltas at txn 11.
    let r_11 = mvtbl.fetch_data(&ap1, 12);
    assert_eq!(Err(Dependency(10)), r_11);

    // Delete the entry written by 10, write to a different ap.
    mvtbl.data().delete(&ap1, 10);
    mvtbl.data().write(ap2.clone(), (10, 2), value_for(10, 2));

    // Read by txn 11 no longer observes entry from txn 10.
    let r_8 = mvtbl.fetch_data(&ap1, 11);
    assert_eq!(Ok(Versioned((8, 3), arc_value_for(8, 3))), r_8);

    // Reads, writes for ap2 and ap3.
    mvtbl.data().write(ap2.clone(), (5, 0), value_for(5, 0));
    mvtbl.data().write(ap3.clone(), (20, 4), value_for(20, 4));
    let r_5 = mvtbl.fetch_data(&ap2, 10);
    assert_eq!(Ok(Versioned((5, 0), arc_value_for(5, 0))), r_5);
    let r_20 = mvtbl.fetch_data(&ap3, 21);
    assert_eq!(Ok(Versioned((20, 4), arc_value_for(20, 4))), r_20);

    // Clear ap1 and ap3.
    mvtbl.data().delete(&ap1, 12);
    mvtbl.data().delete(&ap1, 8);
    mvtbl.data().delete(&ap3, 20);

    // Reads from ap1 and ap3 go to db.
    match_unresolved(
        mvtbl.fetch_data(&ap1, 30),
        DeltaUpdate::Minus((61 + 13) - 11),
    );
    let r_db = mvtbl.fetch_data(&ap3, 30);
    assert_eq!(Err(NotFound), r_db);

    // Read entry by txn 10 at ap2.
    let r_10 = mvtbl.fetch_data(&ap2, 15);
    assert_eq!(Ok(Versioned((10, 2), arc_value_for(10, 2))), r_10);

    // Both delta-write and delta-delta application failures are detected.
    mvtbl.add_delta(ap1.clone(), 30, delta_add(30, 32));
    mvtbl.add_delta(ap1.clone(), 31, delta_add(31, 32));
    let r_33 = mvtbl.fetch_data(&ap1, 33);
    assert_eq!(Err(DeltaApplicationFailure), r_33);

    let val = value_for(10, 3);
    // sub base sub_for for which should underflow.
    let sub_base = AggregatorValue::from_write(&val).unwrap().into();
    mvtbl.data().write(ap2.clone(), (10, 3), val);
    mvtbl.add_delta(ap2.clone(), 30, delta_sub(30 + sub_base, u128::MAX));
    let r_31 = mvtbl.fetch_data(&ap2, 31);
    assert_eq!(Err(DeltaApplicationFailure), r_31);
}

#[test]
fn materialize_delta_shortcut() {
    use MVDataOutput::*;

    let vd: VersionedData<KeyType<Vec<u8>>, Value> = VersionedData::new();
    let ap = KeyType(b"/foo/b".to_vec());
    let limit = 10000;

    vd.add_delta(ap.clone(), 5, delta_add(10, limit));
    vd.add_delta(ap.clone(), 8, delta_add(20, limit));
    vd.add_delta(ap.clone(), 11, delta_add(30, limit));

    match_unresolved(vd.fetch_data(&ap, 10), DeltaUpdate::Plus(30));
    assert_err_eq!(
        vd.materialize_delta(&ap, 8),
        DeltaOp::new(DeltaUpdate::Plus(30), limit, 30, 0)
    );
    vd.set_aggregator_base_value(&ap, 5);
    // Multiple calls are idempotent.
    vd.set_aggregator_base_value(&ap, 5);

    // With base set, commit delta should now succeed.
    assert_ok_eq!(vd.materialize_delta(&ap, 8), 35);
    assert_eq!(vd.fetch_data(&ap, 10), Ok(Resolved(35)));

    // Make sure shortcut is committed by adding a delta at a lower txn idx
    // and ensuring tha fetch_data output no longer changes.
    vd.add_delta(ap.clone(), 6, delta_add(15, limit));
    assert_eq!(vd.fetch_data(&ap, 10), Ok(Resolved(35)));

    // However, if we add a delta at txn_idx = 9, it should have an effect.
    vd.add_delta(ap.clone(), 9, delta_add(15, limit));
    assert_eq!(vd.fetch_data(&ap, 10), Ok(Resolved(50)));
}

#[test]
#[should_panic]
fn aggregator_base_mismatch() {
    let vd: VersionedData<KeyType<Vec<u8>>, Value> = VersionedData::new();
    let ap = KeyType(b"/foo/b".to_vec());

    vd.set_aggregator_base_value(&ap, 10);
    // This call must panic, because it provides a mismatching base value.
    vd.set_aggregator_base_value(&ap, 11);
}

#[test]
#[should_panic]
fn aggregator_base_without_deltas() {
    let vd: VersionedData<KeyType<Vec<u8>>, Value> = VersionedData::new();
    let ap = KeyType(b"/foo/b".to_vec());

    // Must panic as there are no deltas at all.
    vd.set_aggregator_base_value(&ap, 10);
}

#[test]
#[should_panic]
fn commit_without_deltas() {
    let vd: VersionedData<KeyType<Vec<u8>>, Value> = VersionedData::new();
    let ap = KeyType(b"/foo/b".to_vec());

    // Must panic as there are no deltas at all.
    let _ = vd.materialize_delta(&ap, 10);
}

#[test]
#[should_panic]
fn commit_without_entry() {
    let vd: VersionedData<KeyType<Vec<u8>>, Value> = VersionedData::new();
    let ap = KeyType(b"/foo/b".to_vec());

    vd.add_delta(ap.clone(), 8, delta_add(20, 1000));
    vd.set_aggregator_base_value(&ap, 10);

    // Must panic as there is no delta at provided index.
    let _ = vd.materialize_delta(&ap, 9);
}
