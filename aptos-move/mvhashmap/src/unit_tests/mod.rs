// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use super::*;

mod proptest_types;

// Generate a Vec deterministically based on txn_idx and incarnation.
fn value_for(txn_idx: usize, incarnation: usize) -> Vec<usize> {
    vec![txn_idx * 5, txn_idx + incarnation, incarnation * 5]
}

// Generate the value_for txn_idx and incarnation in arc.
fn arc_value_for(txn_idx: usize, incarnation: usize) -> Arc<Vec<usize>> {
    // Generate a Vec deterministically based on txn_idx and incarnation.
    Arc::new(value_for(txn_idx, incarnation))
}

#[test]
fn create_write_read_placeholder_struct() {
    let ap1 = b"/foo/b".to_vec();
    let ap2 = b"/foo/c".to_vec();
    let ap3 = b"/foo/d".to_vec();

    let mvtbl = MVHashMap::new();

    // Reads that should go the the DB return Err(None)
    let r_db = mvtbl.read(&ap1, 5);
    assert_eq!(Err(None), r_db);

    // Write by txn 10.
    mvtbl.write(&ap1, (10, 1), value_for(10, 1));

    // Reads that should go the the DB return Err(None)
    let r_db = mvtbl.read(&ap1, 9);
    assert_eq!(Err(None), r_db);
    // Reads return entries from smaller txns, not txn 10.
    let r_db = mvtbl.read(&ap1, 10);
    assert_eq!(Err(None), r_db);

    // Reads for a higher txn return the entry written by txn 10.
    let r_10 = mvtbl.read(&ap1, 15);
    assert_eq!(Ok(((10, 1), arc_value_for(10, 1))), r_10);

    // More writes.
    mvtbl.write(&ap1, (12, 0), value_for(12, 0));
    mvtbl.write(&ap1, (8, 3), value_for(8, 3));

    // Verify reads.
    let r_12 = mvtbl.read(&ap1, 15);
    assert_eq!(Ok(((12, 0), arc_value_for(12, 0))), r_12);
    let r_10 = mvtbl.read(&ap1, 11);
    assert_eq!(Ok(((10, 1), arc_value_for(10, 1))), r_10);
    let r_8 = mvtbl.read(&ap1, 10);
    assert_eq!(Ok(((8, 3), arc_value_for(8, 3))), r_8);

    // Mark the entry written by 10 as an estimate.
    mvtbl.mark_estimate(&ap1, 10);

    // Read for txn 11 must observe a dependency.
    let r_10 = mvtbl.read(&ap1, 11);
    assert_eq!(Err(Some(10)), r_10);

    // Delete the entry written by 10, write to a different ap.
    mvtbl.delete(&ap1, 10);
    mvtbl.write(&ap2, (10, 2), value_for(10, 2));

    // Read by txn 11 no longer observes entry from txn 10.
    let r_8 = mvtbl.read(&ap1, 11);
    assert_eq!(Ok(((8, 3), arc_value_for(8, 3))), r_8);

    // Reads, writes for ap2 and ap3.
    mvtbl.write(&ap2, (5, 0), value_for(5, 0));
    mvtbl.write(&ap3, (20, 4), value_for(20, 4));
    let r_5 = mvtbl.read(&ap2, 10);
    assert_eq!(Ok(((5, 0), arc_value_for(5, 0))), r_5);
    let r_20 = mvtbl.read(&ap3, 21);
    assert_eq!(Ok(((20, 4), arc_value_for(20, 4))), r_20);

    // Clear ap1 and ap3.
    mvtbl.delete(&ap1, 12);
    mvtbl.delete(&ap1, 8);
    mvtbl.delete(&ap3, 20);

    // Reads from ap1 and ap3 go to db.
    let r_db = mvtbl.read(&ap1, 30);
    assert_eq!(Err(None), r_db);
    let r_db = mvtbl.read(&ap3, 30);
    assert_eq!(Err(None), r_db);

    // No-op delete at ap2.
    mvtbl.delete(&ap2, 11);

    // Read entry by txn 10 at ap2.
    let r_10 = mvtbl.read(&ap2, 15);
    assert_eq!(Ok(((10, 2), arc_value_for(10, 2))), r_10);
}
