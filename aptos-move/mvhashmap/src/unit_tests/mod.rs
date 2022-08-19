// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use super::*;

mod proptest_types;

#[derive(Debug, PartialEq)]
struct Value(Vec<usize>);

impl DeserializeU128 for Value {
    fn deserialize(&self) -> Option<u128> {
        let value = self.0.clone().into_iter().reduce(|a, b| a + b)?;
        Some(value as u128)
    }
}

// Generate a Vec deterministically based on txn_idx and incarnation.
fn value_for(txn_idx: usize, incarnation: usize) -> Value {
    Value(vec![txn_idx * 5, txn_idx + incarnation, incarnation * 5])
}

// Generate the value_for txn_idx and incarnation in arc.
fn arc_value_for(txn_idx: usize, incarnation: usize) -> Arc<Value> {
    // Generate a Vec deterministically based on txn_idx and incarnation.
    Arc::new(value_for(txn_idx, incarnation))
}

// Convert value for txn_idx and incarnation into u128.
fn u128_for(txn_idx: usize, incarnation: usize) -> u128 {
    value_for(txn_idx, incarnation).deserialize().unwrap()
}

// Generate determinitc additions.
fn add_for(txn_idx: usize, limit: u128) -> DeltaOp {
    DeltaOp::Addition {
        value: txn_idx as u128,
        limit,
    }
}

// Generate determinitc subtractions.
fn sub_for(txn_idx: usize) -> DeltaOp {
    DeltaOp::Subtraction {
        value: 5 * txn_idx as u128,
    }
}

#[test]
fn create_write_read_placeholder_struct() {
    use MVHashMapError::*;
    use MVHashMapOutput::*;

    let ap1 = b"/foo/b".to_vec();
    let ap2 = b"/foo/c".to_vec();
    let ap3 = b"/foo/d".to_vec();

    let mvtbl = MVHashMap::new();

    // Reads that should go the the DB return Err(NotFound)
    let r_db = mvtbl.read(&ap1, 5);
    assert_eq!(Err(NotFound), r_db);

    // Write by txn 10.
    mvtbl.add_write(&ap1, (10, 1), value_for(10, 1));

    // Reads that should go the the DB return Err(NotFound)
    let r_db = mvtbl.read(&ap1, 9);
    assert_eq!(Err(NotFound), r_db);
    // Reads return entries from smaller txns, not txn 10.
    let r_db = mvtbl.read(&ap1, 10);
    assert_eq!(Err(NotFound), r_db);

    // Reads for a higher txn return the entry written by txn 10.
    let r_10 = mvtbl.read(&ap1, 15);
    assert_eq!(Ok(Version((10, 1), arc_value_for(10, 1))), r_10);

    // More deltas.
    mvtbl.add_delta(&ap1, 11, add_for(11, 1000));
    mvtbl.add_delta(&ap1, 12, add_for(12, 1000));
    mvtbl.add_delta(&ap1, 13, sub_for(13));

    // Reads have to go traverse deltas until a write is found.
    let r_sum = mvtbl.read(&ap1, 14);
    assert_eq!(Ok(Resolved(u128_for(10, 1) + 11 + 12 - 5 * 13)), r_sum);

    // More writes.
    mvtbl.add_write(&ap1, (12, 0), value_for(12, 0));
    mvtbl.add_write(&ap1, (8, 3), value_for(8, 3));

    // Verify reads.
    let r_12 = mvtbl.read(&ap1, 15);
    assert_eq!(Ok(Resolved(u128_for(12, 0) - 5 * 13)), r_12);
    let r_10 = mvtbl.read(&ap1, 11);
    assert_eq!(Ok(Version((10, 1), arc_value_for(10, 1))), r_10);
    let r_8 = mvtbl.read(&ap1, 10);
    assert_eq!(Ok(Version((8, 3), arc_value_for(8, 3))), r_8);

    // Mark the entry written by 10 as an estimate.
    mvtbl.mark_estimate(&ap1, 10);

    // Read for txn 11 must observe a dependency.
    let r_10 = mvtbl.read(&ap1, 11);
    assert_eq!(Err(Dependency(10)), r_10);

    // Read for txn 12 must observe a dependency when resolving deltas at txn 11.
    let r_11 = mvtbl.read(&ap1, 12);
    assert_eq!(Err(Dependency(10)), r_11);

    // Delete the entry written by 10, write to a different ap.
    mvtbl.delete(&ap1, 10);
    mvtbl.add_write(&ap2, (10, 2), value_for(10, 2));

    // Read by txn 11 no longer observes entry from txn 10.
    let r_8 = mvtbl.read(&ap1, 11);
    assert_eq!(Ok(Version((8, 3), arc_value_for(8, 3))), r_8);

    // Reads, writes for ap2 and ap3.
    mvtbl.add_write(&ap2, (5, 0), value_for(5, 0));
    mvtbl.add_write(&ap3, (20, 4), value_for(20, 4));
    let r_5 = mvtbl.read(&ap2, 10);
    assert_eq!(Ok(Version((5, 0), arc_value_for(5, 0))), r_5);
    let r_20 = mvtbl.read(&ap3, 21);
    assert_eq!(Ok(Version((20, 4), arc_value_for(20, 4))), r_20);

    // Clear ap1 and ap3.
    mvtbl.delete(&ap1, 12);
    mvtbl.delete(&ap1, 8);
    mvtbl.delete(&ap3, 20);

    // Reads from ap1 and ap3 go to db.
    let r_db = mvtbl.read(&ap1, 30);
    assert_eq!(
        Err(Unresolved(DeltaOp::Subtraction { value: 5 * 13 - 11 })),
        r_db
    );
    let r_db = mvtbl.read(&ap3, 30);
    assert_eq!(Err(NotFound), r_db);

    // No-op delete at ap2.
    mvtbl.delete(&ap2, 11);

    // Read entry by txn 10 at ap2.
    let r_10 = mvtbl.read(&ap2, 15);
    assert_eq!(Ok(Version((10, 2), arc_value_for(10, 2))), r_10);

    // Both delta-write and delte-delta application failures are detected.
    mvtbl.add_delta(&ap1, 30, add_for(30, 32));
    mvtbl.add_delta(&ap1, 31, add_for(31, 32));
    let r_33 = mvtbl.read(&ap1, 33);
    assert_eq!(Err(DeltaApplicationFailure), r_33);

    mvtbl.add_write(&ap2, (10, 3), value_for(10, 3));
    mvtbl.add_delta(&ap2, 30, sub_for(30));
    let r_31 = mvtbl.read(&ap2, 31);
    assert_eq!(Err(DeltaApplicationFailure), r_31);
}
