// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use aptos_schemadb::{
    DB, define_schema,
    iterator::SchemaIterator,
    schema::{KeyCodec, Schema, SeekKeyCodec, ValueCodec},
};
use aptos_storage_interface::AptosDbError;
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use rocksdb::{ColumnFamilyDescriptor, DEFAULT_COLUMN_FAMILY_NAME, SliceTransform};

define_schema!(TestSchema, TestKey, TestValue, "TestCF");

#[derive(Debug, Eq, PartialEq)]
struct TestKey(u32, u32, u32);

#[derive(Debug, Eq, PartialEq)]
struct TestValue(u32);

impl KeyCodec<TestSchema> for TestKey {
    fn encode_key(&self) -> Result<Vec<u8>> {
        let mut bytes = vec![];
        bytes.write_u32::<BigEndian>(self.0)?;
        bytes.write_u32::<BigEndian>(self.1)?;
        bytes.write_u32::<BigEndian>(self.2)?;
        Ok(bytes)
    }

    fn decode_key(data: &[u8]) -> Result<Self> {
        let mut reader = std::io::Cursor::new(data);
        Ok(TestKey(
            reader.read_u32::<BigEndian>()?,
            reader.read_u32::<BigEndian>()?,
            reader.read_u32::<BigEndian>()?,
        ))
    }
}

impl ValueCodec<TestSchema> for TestValue {
    fn encode_value(&self) -> Result<Vec<u8>> {
        Ok(self.0.to_be_bytes().to_vec())
    }

    fn decode_value(data: &[u8]) -> Result<Self> {
        let mut reader = std::io::Cursor::new(data);
        Ok(TestValue(reader.read_u32::<BigEndian>()?))
    }
}

pub struct KeyPrefix1(u32);

impl SeekKeyCodec<TestSchema> for KeyPrefix1 {
    fn encode_seek_key(&self) -> Result<Vec<u8>> {
        Ok(self.0.to_be_bytes().to_vec())
    }
}

pub struct KeyPrefix2(u32, u32);

impl SeekKeyCodec<TestSchema> for KeyPrefix2 {
    fn encode_seek_key(&self) -> Result<Vec<u8>> {
        let mut bytes = vec![];
        bytes.write_u32::<BigEndian>(self.0)?;
        bytes.write_u32::<BigEndian>(self.1)?;
        Ok(bytes)
    }
}

fn collect_values(mut iter: SchemaIterator<TestSchema>) -> Vec<u32> {
    collect_values_mut(&mut iter)
}

fn collect_values_mut(iter: &mut SchemaIterator<TestSchema>) -> Vec<u32> {
    iter.map(|row| (row.unwrap().1).0).collect()
}

fn collect_incomplete(iter: &mut SchemaIterator<TestSchema>) -> Vec<u32> {
    let mut res_vec = vec![];
    for res in iter {
        match res {
            Ok((_key, value)) => {
                res_vec.push(value.0);
            },
            Err(AptosDbError::RocksDbIncompleteResult(..)) => {
                return res_vec;
            },
            Err(e) => {
                panic!("expecting incomplete error, got {:?}", e);
            },
        }
    }

    panic!("expecting incomplete error, while iterator terminated.")
}

const EMPTY: [u32; 0] = [];

struct TestDB {
    _tmpdir: aptos_temppath::TempPath,
    db: DB,
}

impl TestDB {
    fn new() -> Self {
        let tmpdir = aptos_temppath::TempPath::new();
        let column_families = vec![DEFAULT_COLUMN_FAMILY_NAME, TestSchema::COLUMN_FAMILY_NAME];
        let mut db_opts = rocksdb::Options::default();
        db_opts.create_if_missing(true);
        db_opts.create_missing_column_families(true);
        let db = DB::open(tmpdir.path(), "test", column_families, &db_opts).unwrap();

        db.put::<TestSchema>(&TestKey(1, 0, 0), &TestValue(100))
            .unwrap();
        db.put::<TestSchema>(&TestKey(1, 0, 2), &TestValue(102))
            .unwrap();
        db.put::<TestSchema>(&TestKey(1, 0, 4), &TestValue(104))
            .unwrap();
        db.put::<TestSchema>(&TestKey(1, 1, 0), &TestValue(110))
            .unwrap();
        db.put::<TestSchema>(&TestKey(1, 1, 2), &TestValue(112))
            .unwrap();
        db.put::<TestSchema>(&TestKey(1, 1, 4), &TestValue(114))
            .unwrap();
        db.put::<TestSchema>(&TestKey(2, 0, 0), &TestValue(200))
            .unwrap();
        db.put::<TestSchema>(&TestKey(2, 0, 2), &TestValue(202))
            .unwrap();

        TestDB {
            _tmpdir: tmpdir,
            db,
        }
    }
}

impl TestDB {
    fn iter(&self) -> SchemaIterator<TestSchema> {
        self.db.iter().expect("Failed to create iterator.")
    }

    fn rev_iter(&self) -> SchemaIterator<TestSchema> {
        self.db.rev_iter().expect("Failed to create iterator.")
    }
}

impl std::ops::Deref for TestDB {
    type Target = DB;

    fn deref(&self) -> &Self::Target {
        &self.db
    }
}

#[test]
fn test_seek_to_first() {
    let db = TestDB::new();

    let mut iter = db.iter();
    iter.seek_to_first();
    assert_eq!(collect_values(iter), [
        100, 102, 104, 110, 112, 114, 200, 202
    ]);

    let mut iter = db.rev_iter();
    iter.seek_to_first();
    assert_eq!(collect_values(iter), [100]);
}

#[test]
fn test_seek_to_last() {
    let db = TestDB::new();

    let mut iter = db.iter();
    iter.seek_to_last();
    assert_eq!(collect_values(iter), [202]);

    let mut iter = db.rev_iter();
    iter.seek_to_last();
    assert_eq!(collect_values(iter), [
        202, 200, 114, 112, 110, 104, 102, 100
    ]);
}

#[test]
fn test_seek_by_existing_key() {
    let db = TestDB::new();

    let mut iter = db.iter();
    iter.seek(&TestKey(1, 1, 0)).unwrap();
    assert_eq!(collect_values(iter), [110, 112, 114, 200, 202]);

    let mut iter = db.rev_iter();
    iter.seek(&TestKey(1, 1, 0)).unwrap();
    assert_eq!(collect_values(iter), [110, 104, 102, 100]);
}

#[test]
fn test_seek_by_nonexistent_key() {
    let db = TestDB::new();

    let mut iter = db.iter();
    iter.seek(&TestKey(1, 1, 1)).unwrap();
    assert_eq!(collect_values(iter), [112, 114, 200, 202]);

    let mut iter = db.rev_iter();
    iter.seek(&TestKey(1, 1, 1)).unwrap();
    assert_eq!(collect_values(iter), [112, 110, 104, 102, 100]);
}

#[test]
fn test_seek_for_prev_by_existing_key() {
    let db = TestDB::new();

    let mut iter = db.iter();
    iter.seek_for_prev(&TestKey(1, 1, 0)).unwrap();
    assert_eq!(collect_values(iter), [110, 112, 114, 200, 202]);

    let mut iter = db.rev_iter();
    iter.seek_for_prev(&TestKey(1, 1, 0)).unwrap();
    assert_eq!(collect_values(iter), [110, 104, 102, 100]);
}

#[test]
fn test_seek_for_prev_by_nonexistent_key() {
    let db = TestDB::new();

    let mut iter = db.iter();
    iter.seek_for_prev(&TestKey(1, 1, 1)).unwrap();
    assert_eq!(collect_values(iter), [110, 112, 114, 200, 202]);

    let mut iter = db.rev_iter();
    iter.seek_for_prev(&TestKey(1, 1, 1)).unwrap();
    assert_eq!(collect_values(iter), [110, 104, 102, 100]);
}

#[test]
fn test_seek_by_1prefix() {
    let db = TestDB::new();

    let mut iter = db.iter();
    iter.seek(&KeyPrefix1(2)).unwrap();
    assert_eq!(collect_values(iter), [200, 202]);

    let mut iter = db.rev_iter();
    iter.seek(&KeyPrefix1(2)).unwrap();
    assert_eq!(collect_values(iter), [200, 114, 112, 110, 104, 102, 100]);
}

#[test]
fn test_seek_for_prev_by_1prefix() {
    let db = TestDB::new();

    let mut iter = db.iter();
    iter.seek_for_prev(&KeyPrefix1(2)).unwrap();
    assert_eq!(collect_values(iter), [114, 200, 202]);

    let mut iter = db.rev_iter();
    iter.seek_for_prev(&KeyPrefix1(2)).unwrap();
    assert_eq!(collect_values(iter), [114, 112, 110, 104, 102, 100]);
}

#[test]
fn test_seek_by_2prefix() {
    let db = TestDB::new();

    let mut iter = db.iter();
    iter.seek(&KeyPrefix2(2, 0)).unwrap();
    assert_eq!(collect_values(iter), [200, 202]);

    let mut iter = db.rev_iter();
    iter.seek(&KeyPrefix2(2, 0)).unwrap();
    assert_eq!(collect_values(iter), [200, 114, 112, 110, 104, 102, 100]);
}

#[test]
fn test_seek_for_prev_by_2prefix() {
    let db = TestDB::new();

    let mut iter = db.iter();
    iter.seek_for_prev(&KeyPrefix2(2, 0)).unwrap();
    assert_eq!(collect_values(iter), [114, 200, 202]);

    let mut iter = db.rev_iter();
    iter.seek_for_prev(&KeyPrefix2(2, 0)).unwrap();
    assert_eq!(collect_values(iter), [114, 112, 110, 104, 102, 100]);
}

struct TestDBWithPrefixExtractor {
    _tmpdir: aptos_temppath::TempPath,
    db: DB,
}

impl TestDBWithPrefixExtractor {
    fn new() -> Self {
        let tmpdir = aptos_temppath::TempPath::new();
        let mut db_opts = rocksdb::Options::default();
        db_opts.create_if_missing(true);
        db_opts.create_missing_column_families(true);
        let db = DB::open_cf(&db_opts, tmpdir.path(), "test_with_prefix", vec![
            ColumnFamilyDescriptor::new(DEFAULT_COLUMN_FAMILY_NAME, rocksdb::Options::default()),
            ColumnFamilyDescriptor::new(TestSchema::COLUMN_FAMILY_NAME, {
                let mut opts = rocksdb::Options::default();
                opts.set_prefix_extractor(SliceTransform::create(
                    "2_prefix_extractor",
                    |key| &key[0..std::cmp::min(8, key.len())],
                    None,
                ));
                opts
            }),
        ])
        .unwrap();

        // delete later
        db.put::<TestSchema>(&TestKey(1, 1, 1), &TestValue(111))
            .unwrap();
        db.put::<TestSchema>(&TestKey(1, 2, 2), &TestValue(122))
            .unwrap();
        db.put::<TestSchema>(&TestKey(1, 2, 3), &TestValue(123))
            .unwrap();

        //  delete without update TestKey(1,2,4)

        db.put::<TestSchema>(&TestKey(1, 2, 5), &TestValue(125))
            .unwrap();
        db.put::<TestSchema>(&TestKey(1, 5, 5), &TestValue(155))
            .unwrap();
        // delete later
        db.put::<TestSchema>(&TestKey(1, 6, 6), &TestValue(166))
            .unwrap();
        // to be overwritten
        db.put::<TestSchema>(&TestKey(1, 7, 7), &TestValue(0))
            .unwrap();
        // overwrite
        db.put::<TestSchema>(&TestKey(1, 7, 7), &TestValue(177))
            .unwrap();
        db.put::<TestSchema>(&TestKey(2, 2, 2), &TestValue(222))
            .unwrap();
        // delete later
        db.put::<TestSchema>(&TestKey(2, 3, 3), &TestValue(233))
            .unwrap();

        // delete without update TestKey(2,4,4)

        db.put::<TestSchema>(&TestKey(2, 7, 7), &TestValue(277))
            .unwrap();
        db.put::<TestSchema>(&TestKey(2, 8, 8), &TestValue(288))
            .unwrap();
        db.put::<TestSchema>(&TestKey(2, 9, 9), &TestValue(299))
            .unwrap();
        // delete later
        db.put::<TestSchema>(&TestKey(3, 7, 7), &TestValue(377))
            .unwrap();
        // delete later
        db.put::<TestSchema>(&TestKey(3, 8, 8), &TestValue(388))
            .unwrap();
        db.put::<TestSchema>(&TestKey(3, 9, 9), &TestValue(399))
            .unwrap();

        db.delete::<TestSchema>(&TestKey(1, 1, 1)).unwrap();
        db.delete::<TestSchema>(&TestKey(1, 2, 4)).unwrap();
        db.delete::<TestSchema>(&TestKey(1, 6, 6)).unwrap();
        db.delete::<TestSchema>(&TestKey(2, 3, 3)).unwrap();
        db.delete::<TestSchema>(&TestKey(2, 4, 4)).unwrap();
        db.delete::<TestSchema>(&TestKey(3, 7, 7)).unwrap();
        db.delete::<TestSchema>(&TestKey(3, 8, 8)).unwrap();

        TestDBWithPrefixExtractor {
            _tmpdir: tmpdir,
            db,
        }
    }

    fn iter(&self) -> SchemaIterator<TestSchema> {
        self.db.iter().expect("Failed to create iterator.")
    }

    fn iter_with_same_prefix(&self) -> SchemaIterator<TestSchema> {
        let mut opts = rocksdb::ReadOptions::default();
        opts.set_prefix_same_as_start(true);
        self.db
            .iter_with_opts(opts)
            .expect("Failed to create iterator.")
    }

    fn iter_with_max_skipped_deletions(&self, num_skips: u64) -> SchemaIterator<TestSchema> {
        let mut opts = rocksdb::ReadOptions::default();
        opts.set_max_skippable_internal_keys(num_skips);
        self.db
            .iter_with_opts(opts)
            .expect("Failed to create iterator.")
    }

    fn iter_with_upper_bound(&self, upper_bound: Vec<u8>) -> SchemaIterator<TestSchema> {
        let mut opts = rocksdb::ReadOptions::default();
        opts.set_iterate_upper_bound(upper_bound);
        self.db
            .iter_with_opts(opts)
            .expect("Failed to create iterator.")
    }
}

#[test]
fn test_iter_with_prefix_extractor() {
    let db = TestDBWithPrefixExtractor::new();

    let all_values = [122, 123, 125, 155, 177, 222, 277, 288, 299, 399];

    let mut iter = db.iter();
    iter.seek(&TestKey(0, 0, 0)).unwrap();
    assert_eq!(collect_values_mut(&mut iter), all_values);

    iter.seek(&KeyPrefix2(0, 0)).unwrap();
    assert_eq!(collect_values_mut(&mut iter), all_values);

    iter.seek(&KeyPrefix1(0)).unwrap();
    assert_eq!(collect_values_mut(&mut iter), all_values);
}

#[test]
fn test_iter_with_same_prefix() {
    let db = TestDBWithPrefixExtractor::new();

    let mut iter = db.iter_with_same_prefix();
    iter.seek(&TestKey(1, 2, 0)).unwrap();
    assert_eq!(collect_values_mut(&mut iter), [122, 123, 125]);

    iter.seek(&TestKey(1, 2, 3)).unwrap();
    assert_eq!(collect_values_mut(&mut iter), [123, 125]);

    iter.seek(&TestKey(1, 2, 4)).unwrap();
    assert_eq!(collect_values_mut(&mut iter), [125]);

    iter.seek(&KeyPrefix2(1, 2)).unwrap();
    assert_eq!(collect_values_mut(&mut iter), [122, 123, 125]);

    iter.seek(&KeyPrefix2(1, 0)).unwrap();
    assert_eq!(collect_values_mut(&mut iter), EMPTY);

    iter.seek(&KeyPrefix2(1, 1)).unwrap();
    assert_eq!(collect_values_mut(&mut iter), EMPTY);

    iter.seek(&KeyPrefix1(1)).unwrap();
    assert_eq!(collect_values_mut(&mut iter), EMPTY);
}

#[test]
fn test_iter_with_max_skipped_deletions() {
    let db = TestDBWithPrefixExtractor::new();

    // -----------------
    // max skip 1

    let mut iter = db.iter_with_max_skipped_deletions(1);

    iter.seek_to_first();
    assert_eq!(collect_incomplete(&mut iter), EMPTY);

    iter.seek(&TestKey(1, 1, 1)).unwrap();
    assert_eq!(collect_incomplete(&mut iter), EMPTY);

    iter.seek(&KeyPrefix1(0)).unwrap();
    assert_eq!(collect_incomplete(&mut iter), EMPTY);

    iter.seek(&KeyPrefix2(1, 1)).unwrap();
    assert_eq!(collect_incomplete(&mut iter), EMPTY);

    iter.seek(&TestKey(1, 2, 3)).unwrap();
    assert_eq!(collect_incomplete(&mut iter), [123, 125, 155]);

    iter.seek(&TestKey(1, 5, 5)).unwrap();
    assert_eq!(collect_incomplete(&mut iter), [155]);

    iter.seek(&TestKey(1, 6, 7)).unwrap();
    assert_eq!(collect_incomplete(&mut iter), [177, 222]);

    // -----------------
    // max skip 2

    let mut iter = db.iter_with_max_skipped_deletions(2);

    iter.seek(&TestKey(1, 2, 3)).unwrap();
    assert_eq!(collect_incomplete(&mut iter), [123, 125, 155, 177, 222]);

    // -----------------
    // max skip 3

    let mut iter = db.iter_with_max_skipped_deletions(3);
    iter.seek(&TestKey(1, 5, 5)).unwrap();
    assert_eq!(collect_incomplete(&mut iter), [
        155, 177, 222, 277, 288, 299
    ]);

    // -----------------
    // max skip 4

    let mut iter = db.iter_with_max_skipped_deletions(4);
    iter.seek(&TestKey(0, 0, 0)).unwrap();
    assert_eq!(collect_values_mut(&mut iter), [
        122, 123, 125, 155, 177, 222, 277, 288, 299, 399
    ]);
}

#[test]
fn test_iter_with_upper_bound() {
    let db = TestDBWithPrefixExtractor::new();

    let mut iter =
        db.iter_with_upper_bound(KeyCodec::<TestSchema>::encode_key(&TestKey(1, 5, 5)).unwrap());
    iter.seek_to_first();
    assert_eq!(collect_values_mut(&mut iter), [122, 123, 125]);

    let mut iter =
        db.iter_with_upper_bound(KeyCodec::<TestSchema>::encode_key(&TestKey(1, 2, 4)).unwrap());
    iter.seek_to_first();
    assert_eq!(collect_values_mut(&mut iter), [122, 123]);

    let mut iter = db.iter_with_upper_bound(
        SeekKeyCodec::<TestSchema>::encode_seek_key(&KeyPrefix1(2)).unwrap(),
    );
    iter.seek(&KeyPrefix1(1)).unwrap();
    assert_eq!(collect_values_mut(&mut iter), [122, 123, 125, 155, 177]);

    let mut iter = db.iter_with_upper_bound(
        SeekKeyCodec::<TestSchema>::encode_seek_key(&KeyPrefix2(1, 5)).unwrap(),
    );
    iter.seek(&KeyPrefix1(1)).unwrap();
    assert_eq!(collect_values_mut(&mut iter), [122, 123, 125]);
}
