// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use super::{
    types::{
        test::{KeyType, TestValue},
        MVDataError, MVDataOutput,
    },
    unsync_map::UnsyncMap,
    *,
};
use crate::types::test::value_for;
use aptos_types::{
    on_chain_config::CurrentTimeMicroseconds, state_store::state_value::StateValueMetadata,
    write_set::WriteOpKind,
};
use claims::{assert_none, assert_some_eq};

mod dependencies;
mod proptest_types;

#[test]
fn unsync_map_data_basic() {
    let map: UnsyncMap<KeyType<Vec<u8>>, usize, TestValue, ()> = UnsyncMap::new();

    let ap = KeyType(b"/foo/b".to_vec());

    // Reads that should go the DB return None
    assert_none!(map.fetch_data(&ap));
    // Ensure write registers the new value.
    map.write(ap.clone(), value_for(10, 1));
    assert_some_eq!(map.fetch_data(&ap), value_for(10, 1));
    // Ensure the next write overwrites the value.
    map.write(ap.clone(), value_for(14, 1));
    assert_some_eq!(map.fetch_data(&ap), value_for(14, 1));
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TestMetadataValue {
    metadata: u64,
}

impl TestMetadataValue {
    fn as_state_value_metadata(&self) -> Option<StateValueMetadata> {
        Some(StateValueMetadata::legacy(
            self.metadata,
            &CurrentTimeMicroseconds {
                microseconds: self.metadata,
            },
        ))
    }
}

impl SpeculativeValue for TestMetadataValue {
    fn eq_value(&self, other: &Self) -> bool {
        self == other
    }

    fn eq_metadata(&self, other: &Self) -> bool {
        self.as_state_value_metadata() == other.as_state_value_metadata()
    }

    fn bytes_len(&self) -> Option<usize> {
        None
    }

    fn write_op_kind(&self) -> WriteOpKind {
        unimplemented!("Irrelevant for the tests")
    }
}

#[test]
fn write_metadata() {
    let ap = KeyType(b"/foo/b".to_vec());

    let mvtbl: MVHashMap<KeyType<Vec<u8>>, usize, TestMetadataValue, ()> = MVHashMap::new();

    let metadata_5 = TestMetadataValue { metadata: 5 };
    let metadata_6 = TestMetadataValue { metadata: 6 };

    assert!(mvtbl
        .data()
        .write_metadata(ap.clone(), 10, 1, metadata_5.clone()));
    assert!(mvtbl.data().write_metadata(ap.clone(), 10, 1, metadata_6));
    assert!(mvtbl
        .data()
        .write_metadata(ap.clone(), 10, 1, metadata_5.clone()));
    // Should be equal to recorded metadata and return false (no change).
    assert!(!mvtbl
        .data()
        .write_metadata(ap.clone(), 10, 1, metadata_5.clone()));

    assert!(mvtbl.data().write_metadata(ap.clone(), 11, 1, metadata_5));
}

#[test]
fn create_write_read_placeholder_struct() {
    use MVDataError::*;
    use MVDataOutput::*;

    let ap1 = KeyType(b"/foo/b".to_vec());
    let ap2 = KeyType(b"/foo/c".to_vec());
    let ap3 = KeyType(b"/foo/d".to_vec());

    let mvtbl: MVHashMap<KeyType<Vec<u8>>, usize, TestValue, ()> = MVHashMap::new();

    // Reads that should go the DB return Err(Uninitialized)
    let r_db = mvtbl.data().fetch_data_no_record(&ap1, 5);
    assert_eq!(Err(Uninitialized), r_db);

    // Write by txn 10.
    mvtbl
        .data()
        .write(ap1.clone(), 10, 1, value_for(10, 1))
        .unwrap();

    // Reads that should go the DB return Err(Uninitialized)
    let r_db = mvtbl.data().fetch_data_no_record(&ap1, 9);
    assert_eq!(Err(Uninitialized), r_db);
    // Reads return entries from smaller txns, not txn 10.
    let r_db = mvtbl.data().fetch_data_no_record(&ap1, 10);
    assert_eq!(Err(Uninitialized), r_db);

    // Reads for a higher txn return the entry written by txn 10.
    let r_10 = mvtbl.data().fetch_data_no_record(&ap1, 15);
    assert_eq!(Ok(Versioned(Ok((10, 1)), value_for(10, 1))), r_10);

    // More writes.
    mvtbl
        .data()
        .write(ap1.clone(), 12, 0, value_for(12, 0))
        .unwrap();
    mvtbl
        .data()
        .write(ap1.clone(), 8, 3, value_for(8, 3))
        .unwrap();

    // Verify reads return the latest write below the reader index.
    let r_12 = mvtbl.data().fetch_data_no_record(&ap1, 15);
    assert_eq!(Ok(Versioned(Ok((12, 0)), value_for(12, 0))), r_12);
    let r_10 = mvtbl.data().fetch_data_no_record(&ap1, 11);
    assert_eq!(Ok(Versioned(Ok((10, 1)), value_for(10, 1))), r_10);
    let r_8 = mvtbl.data().fetch_data_no_record(&ap1, 10);
    assert_eq!(Ok(Versioned(Ok((8, 3)), value_for(8, 3))), r_8);

    // Mark the entry written by 10 as an estimate.
    mvtbl.data().mark_estimate(&ap1, 10);

    // Read for txn 11 must observe a dependency.
    let r_10 = mvtbl.data().fetch_data_no_record(&ap1, 11);
    assert_eq!(Err(Dependency(10)), r_10);

    // Delete the entry written by 10, write to a different ap.
    mvtbl.data().remove(&ap1, 10);
    mvtbl
        .data()
        .write(ap2.clone(), 10, 2, value_for(10, 2))
        .unwrap();

    // Read by txn 11 no longer observes entry from txn 10.
    let r_8 = mvtbl.data().fetch_data_no_record(&ap1, 11);
    assert_eq!(Ok(Versioned(Ok((8, 3)), value_for(8, 3))), r_8);

    // Reads, writes for ap2 and ap3.
    mvtbl
        .data()
        .write(ap2.clone(), 5, 0, value_for(5, 0))
        .unwrap();
    mvtbl
        .data()
        .write(ap3.clone(), 20, 4, value_for(20, 4))
        .unwrap();
    let r_5 = mvtbl.data().fetch_data_no_record(&ap2, 10);
    assert_eq!(Ok(Versioned(Ok((5, 0)), value_for(5, 0))), r_5);
    let r_20 = mvtbl.data().fetch_data_no_record(&ap3, 21);
    assert_eq!(Ok(Versioned(Ok((20, 4)), value_for(20, 4))), r_20);

    // Clear ap1 and ap3.
    mvtbl.data().remove(&ap1, 12);
    mvtbl.data().remove(&ap1, 8);
    mvtbl.data().remove(&ap3, 20);

    // Reads from emptied ap1 and ap3 go to db.
    let r_db = mvtbl.data().fetch_data_no_record(&ap1, 30);
    assert_eq!(Err(Uninitialized), r_db);
    let r_db = mvtbl.data().fetch_data_no_record(&ap3, 30);
    assert_eq!(Err(Uninitialized), r_db);

    // Read entry by txn 10 at ap2.
    let r_10 = mvtbl.data().fetch_data_no_record(&ap2, 15);
    assert_eq!(Ok(Versioned(Ok((10, 2)), value_for(10, 2))), r_10);
}
