// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::error::DbError;
use crate::quorum_store::types::PersistedValue;
use anyhow::Result;
use aptos_crypto::HashValue;
use aptos_logger::info;
use schemadb::{
    schema::{KeyCodec, Schema, ValueCodec},
    ColumnFamilyName, Options, ReadOptions, SchemaBatch, DB,
};
use std::{collections::HashMap, path::Path, time::Instant};

const BATCH_CF_NAME: ColumnFamilyName = "batch";

pub(crate) struct BatchSchema;

impl Schema for BatchSchema {
    const COLUMN_FAMILY_NAME: schemadb::ColumnFamilyName = BATCH_CF_NAME;
    type Key = HashValue;
    type Value = PersistedValue;
}

impl KeyCodec<BatchSchema> for HashValue {
    fn encode_key(&self) -> Result<Vec<u8>> {
        Ok(self.to_vec())
    }

    fn decode_key(data: &[u8]) -> Result<Self> {
        Ok(HashValue::from_slice(data)?)
    }
}

impl ValueCodec<BatchSchema> for PersistedValue {
    fn encode_value(&self) -> Result<Vec<u8>> {
        Ok(bcs::to_bytes(&self)?)
    }

    fn decode_value(data: &[u8]) -> Result<Self> {
        Ok(bcs::from_bytes(data)?)
    }
}

pub struct QuorumStoreDB {
    db: DB,
}

#[allow(dead_code)]
impl QuorumStoreDB {
    pub(crate) fn new<P: AsRef<Path> + Clone>(db_root_path: P) -> Self {
        let column_families = vec![BATCH_CF_NAME];

        let path = db_root_path.as_ref().join("quorumstoreDB");
        let instant = Instant::now();
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);
        let db = DB::open(path.clone(), "quorumstoreDB", column_families, &opts)
            .expect("QuorumstoreDB open failed; unable to continue");

        info!(
            "Opened QuorumstoreDB at {:?} in {} ms",
            path,
            instant.elapsed().as_millis()
        );

        Self { db }
    }

    pub(crate) fn delete(&self, digests: Vec<HashValue>) -> Result<(), DbError> {
        let mut batch = SchemaBatch::new();
        for digest in digests.iter() {
            batch.delete::<BatchSchema>(digest)?;
        }
        self.db.write_schemas(batch)?;
        Ok(())
    }

    pub(crate) fn get_data(&self) -> Result<HashMap<HashValue, PersistedValue>> {
        let mut iter = self.db.iter::<BatchSchema>(ReadOptions::default())?;
        iter.seek_to_first();
        Ok(iter.collect::<Result<HashMap<HashValue, PersistedValue>>>()?)
    }

    pub(crate) fn save_batch(
        &self,
        digest: HashValue,
        batch: PersistedValue,
    ) -> Result<(), DbError> {
        Ok(self.db.put::<BatchSchema>(&digest, &batch)?)
    }

    pub(crate) fn get_batch(&self, digest: HashValue) -> Result<Option<PersistedValue>, DbError> {
        Ok(self.db.get::<BatchSchema>(&digest)?)
    }
}
