// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use byteorder::{LittleEndian, ReadBytesExt};
use rocksdb::DEFAULT_COLUMN_FAMILY_NAME;
use schemadb::{
    define_schema,
    schema::{KeyCodec, Schema, ValueCodec},
    ColumnFamilyName, SchemaBatch, DB,
};

// Creating two schemas that share exactly the same structure but are stored in different column
// families. Also note that the key and value are of the same type `TestField`. By implementing
// both the `KeyCodec<>` and `ValueCodec<>` traits for both schemas, we are able to use it
// everywhere.
define_schema!(TestSchema1, TestField, TestField, "TestCF1");
define_schema!(TestSchema2, TestField, TestField, "TestCF2");

#[derive(Debug, Eq, PartialEq)]
struct TestField(u32);

impl TestField {
    fn to_bytes(&self) -> Vec<u8> {
        self.0.to_le_bytes().to_vec()
    }

    fn from_bytes(data: &[u8]) -> Result<Self> {
        let mut reader = std::io::Cursor::new(data);
        Ok(TestField(reader.read_u32::<LittleEndian>()?))
    }
}

impl KeyCodec<TestSchema1> for TestField {
    fn encode_key(&self) -> Result<Vec<u8>> {
        Ok(self.to_bytes())
    }

    fn decode_key(data: &[u8]) -> Result<Self> {
        Self::from_bytes(data)
    }
}

impl ValueCodec<TestSchema1> for TestField {
    fn encode_value(&self) -> Result<Vec<u8>> {
        Ok(self.to_bytes())
    }

    fn decode_value(data: &[u8]) -> Result<Self> {
        Self::from_bytes(data)
    }
}

impl KeyCodec<TestSchema2> for TestField {
    fn encode_key(&self) -> Result<Vec<u8>> {
        Ok(self.to_bytes())
    }

    fn decode_key(data: &[u8]) -> Result<Self> {
        Self::from_bytes(data)
    }
}

impl ValueCodec<TestSchema2> for TestField {
    fn encode_value(&self) -> Result<Vec<u8>> {
        Ok(self.to_bytes())
    }

    fn decode_value(data: &[u8]) -> Result<Self> {
        Self::from_bytes(data)
    }
}

fn get_column_families() -> Vec<ColumnFamilyName> {
    vec![
        DEFAULT_COLUMN_FAMILY_NAME,
        TestSchema1::COLUMN_FAMILY_NAME,
        TestSchema2::COLUMN_FAMILY_NAME,
    ]
}

fn open_db(dir: &aptos_temppath::TempPath) -> DB {
    let mut db_opts = rocksdb::Options::default();
    db_opts.create_if_missing(true);
    db_opts.create_missing_column_families(true);
    DB::open(&dir.path(), "test", get_column_families(), &db_opts).expect("Failed to open DB.")
}

fn open_db_read_only(dir: &aptos_temppath::TempPath) -> DB {
    DB::open_cf_readonly(
        &rocksdb::Options::default(),
        &dir.path(),
        "test",
        get_column_families(),
    )
    .expect("Failed to open DB.")
}

fn open_db_as_secondary(dir: &aptos_temppath::TempPath, dir_sec: &aptos_temppath::TempPath) -> DB {
    DB::open_cf_as_secondary(
        &rocksdb::Options::default(),
        &dir.path(),
        &dir_sec.path(),
        "test",
        get_column_families(),
    )
    .expect("Failed to open DB.")
}

struct TestDB {
    _tmpdir: aptos_temppath::TempPath,
    db: DB,
}

impl TestDB {
    fn new() -> Self {
        let tmpdir = aptos_temppath::TempPath::new();
        let db = open_db(&tmpdir);

        TestDB {
            _tmpdir: tmpdir,
            db,
        }
    }
}

impl std::ops::Deref for TestDB {
    type Target = DB;

    fn deref(&self) -> &Self::Target {
        &self.db
    }
}

#[test]
fn test_schema_put_get() {
    let db = TestDB::new();

    db.put::<TestSchema1>(&TestField(0), &TestField(0)).unwrap();
    db.put::<TestSchema1>(&TestField(1), &TestField(1)).unwrap();
    db.put::<TestSchema1>(&TestField(2), &TestField(2)).unwrap();
    db.put::<TestSchema2>(&TestField(2), &TestField(3)).unwrap();
    db.put::<TestSchema2>(&TestField(3), &TestField(4)).unwrap();
    db.put::<TestSchema2>(&TestField(4), &TestField(5)).unwrap();

    assert_eq!(
        db.get::<TestSchema1>(&TestField(0)).unwrap(),
        Some(TestField(0)),
    );
    assert_eq!(
        db.get::<TestSchema1>(&TestField(1)).unwrap(),
        Some(TestField(1)),
    );
    assert_eq!(
        db.get::<TestSchema1>(&TestField(2)).unwrap(),
        Some(TestField(2)),
    );
    assert_eq!(db.get::<TestSchema1>(&TestField(3)).unwrap(), None);

    assert_eq!(db.get::<TestSchema2>(&TestField(1)).unwrap(), None);
    assert_eq!(
        db.get::<TestSchema2>(&TestField(2)).unwrap(),
        Some(TestField(3)),
    );
    assert_eq!(
        db.get::<TestSchema2>(&TestField(3)).unwrap(),
        Some(TestField(4)),
    );
    assert_eq!(
        db.get::<TestSchema2>(&TestField(4)).unwrap(),
        Some(TestField(5)),
    );
}

fn collect_values<S: Schema>(db: &TestDB) -> Vec<(S::Key, S::Value)> {
    let mut iter = db
        .iter::<S>(Default::default())
        .expect("Failed to create iterator.");
    iter.seek_to_first();
    iter.collect::<Result<Vec<_>>>().unwrap()
}

fn gen_expected_values(values: &[(u32, u32)]) -> Vec<(TestField, TestField)> {
    values
        .iter()
        .cloned()
        .map(|(x, y)| (TestField(x), TestField(y)))
        .collect()
}

#[test]
fn test_single_schema_batch() {
    let db = TestDB::new();

    let db_batch = SchemaBatch::new();
    db_batch
        .put::<TestSchema1>(&TestField(0), &TestField(0))
        .unwrap();
    db_batch
        .put::<TestSchema1>(&TestField(1), &TestField(1))
        .unwrap();
    db_batch
        .put::<TestSchema1>(&TestField(2), &TestField(2))
        .unwrap();
    db_batch
        .put::<TestSchema2>(&TestField(3), &TestField(3))
        .unwrap();
    db_batch.delete::<TestSchema2>(&TestField(4)).unwrap();
    db_batch.delete::<TestSchema2>(&TestField(3)).unwrap();
    db_batch
        .put::<TestSchema2>(&TestField(4), &TestField(4))
        .unwrap();
    db_batch
        .put::<TestSchema2>(&TestField(5), &TestField(5))
        .unwrap();

    db.write_schemas(db_batch).unwrap();

    assert_eq!(
        collect_values::<TestSchema1>(&db),
        gen_expected_values(&[(0, 0), (1, 1), (2, 2)]),
    );
    assert_eq!(
        collect_values::<TestSchema2>(&db),
        gen_expected_values(&[(4, 4), (5, 5)]),
    );
}

#[test]
fn test_two_schema_batches() {
    let db = TestDB::new();

    let db_batch1 = SchemaBatch::new();
    db_batch1
        .put::<TestSchema1>(&TestField(0), &TestField(0))
        .unwrap();
    db_batch1
        .put::<TestSchema1>(&TestField(1), &TestField(1))
        .unwrap();
    db_batch1
        .put::<TestSchema1>(&TestField(2), &TestField(2))
        .unwrap();
    db_batch1.delete::<TestSchema1>(&TestField(2)).unwrap();
    db.write_schemas(db_batch1).unwrap();

    assert_eq!(
        collect_values::<TestSchema1>(&db),
        gen_expected_values(&[(0, 0), (1, 1)]),
    );

    let db_batch2 = SchemaBatch::new();
    db_batch2.delete::<TestSchema2>(&TestField(3)).unwrap();
    db_batch2
        .put::<TestSchema2>(&TestField(3), &TestField(3))
        .unwrap();
    db_batch2
        .put::<TestSchema2>(&TestField(4), &TestField(4))
        .unwrap();
    db_batch2
        .put::<TestSchema2>(&TestField(5), &TestField(5))
        .unwrap();
    db.write_schemas(db_batch2).unwrap();

    assert_eq!(
        collect_values::<TestSchema1>(&db),
        gen_expected_values(&[(0, 0), (1, 1)]),
    );
    assert_eq!(
        collect_values::<TestSchema2>(&db),
        gen_expected_values(&[(3, 3), (4, 4), (5, 5)]),
    );
}

#[test]
fn test_reopen() {
    let tmpdir = aptos_temppath::TempPath::new();
    {
        let db = open_db(&tmpdir);
        db.put::<TestSchema1>(&TestField(0), &TestField(0)).unwrap();
        assert_eq!(
            db.get::<TestSchema1>(&TestField(0)).unwrap(),
            Some(TestField(0)),
        );
    }
    {
        let db = open_db(&tmpdir);
        assert_eq!(
            db.get::<TestSchema1>(&TestField(0)).unwrap(),
            Some(TestField(0)),
        );
    }
}

#[test]
fn test_open_read_only() {
    let tmpdir = aptos_temppath::TempPath::new();
    {
        let db = open_db(&tmpdir);
        db.put::<TestSchema1>(&TestField(0), &TestField(0)).unwrap();
    }
    {
        let db = open_db_read_only(&tmpdir);
        assert_eq!(
            db.get::<TestSchema1>(&TestField(0)).unwrap(),
            Some(TestField(0)),
        );
        assert!(db.put::<TestSchema1>(&TestField(1), &TestField(1)).is_err());
    }
}

#[test]
fn test_open_as_secondary() {
    let tmpdir = aptos_temppath::TempPath::new();
    let tmpdir_sec = aptos_temppath::TempPath::new();

    let db = open_db(&tmpdir);
    db.put::<TestSchema1>(&TestField(0), &TestField(0)).unwrap();

    let db_sec = open_db_as_secondary(&tmpdir, &tmpdir_sec);
    assert_eq!(
        db_sec.get::<TestSchema1>(&TestField(0)).unwrap(),
        Some(TestField(0)),
    );
}

#[test]
fn test_report_size() {
    let db = TestDB::new();

    for i in 0..1000 {
        let db_batch = SchemaBatch::new();
        db_batch
            .put::<TestSchema1>(&TestField(i), &TestField(i))
            .unwrap();
        db_batch
            .put::<TestSchema2>(&TestField(i), &TestField(i))
            .unwrap();
        db.write_schemas(db_batch).unwrap();
    }

    db.flush_cf("TestCF1").unwrap();
    db.flush_cf("TestCF2").unwrap();

    assert!(
        db.get_property("TestCF1", "rocksdb.estimate-live-data-size")
            .unwrap()
            > 0
    );
    assert!(
        db.get_property("TestCF2", "rocksdb.estimate-live-data-size")
            .unwrap()
            > 0
    );
    assert_eq!(
        db.get_property("default", "rocksdb.estimate-live-data-size")
            .unwrap(),
        0
    );
}

#[test]
fn test_checkpoint() {
    let tmpdir = aptos_temppath::TempPath::new();
    let checkpoint = aptos_temppath::TempPath::new();
    {
        let db = open_db(&tmpdir);
        db.put::<TestSchema1>(&TestField(0), &TestField(0)).unwrap();
        db.create_checkpoint(&checkpoint).unwrap();
    }
    {
        let db = open_db(&tmpdir);
        assert_eq!(
            db.get::<TestSchema1>(&TestField(0)).unwrap(),
            Some(TestField(0)),
        );

        let cp = open_db(&checkpoint);
        assert_eq!(
            cp.get::<TestSchema1>(&TestField(0)).unwrap(),
            Some(TestField(0)),
        );
        cp.put::<TestSchema1>(&TestField(1), &TestField(1)).unwrap();
        assert_eq!(
            cp.get::<TestSchema1>(&TestField(1)).unwrap(),
            Some(TestField(1)),
        );
        assert_eq!(db.get::<TestSchema1>(&TestField(1)).unwrap(), None);
    }
}
