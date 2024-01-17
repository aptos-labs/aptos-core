// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    error::DbError,
    rand::rand_gen::{
        storage::{
            interface::AugDataStorage,
            schema::{
                AugDataSchema, CertifiedAugDataSchema, AUG_DATA_CF_NAME, CERTIFIED_AUG_DATA_CF_NAME,
            },
        },
        types::{AugData, AugDataId, AugmentedData, CertifiedAugData},
    },
};
use anyhow::Result;
use aptos_logger::info;
use aptos_schemadb::{schema::Schema, Options, ReadOptions, SchemaBatch, DB};
use std::{path::Path, sync::Arc, time::Instant};

pub struct RandDb {
    db: Arc<DB>,
}

pub const RAND_DB_NAME: &str = "rand_db";

impl RandDb {
    pub(crate) fn new<P: AsRef<Path> + Clone>(db_root_path: P) -> Self {
        let column_families = vec![AUG_DATA_CF_NAME, CERTIFIED_AUG_DATA_CF_NAME];

        let path = db_root_path.as_ref().join(RAND_DB_NAME);
        let instant = Instant::now();
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);
        let db = Arc::new(
            DB::open(path.clone(), RAND_DB_NAME, column_families, &opts)
                .expect("RandDB open failed; unable to continue"),
        );

        info!(
            "Opened RandDB at {:?} in {} ms",
            path,
            instant.elapsed().as_millis()
        );

        Self { db }
    }

    fn commit(&self, batch: SchemaBatch) -> Result<(), DbError> {
        self.db.write_schemas(batch)?;
        Ok(())
    }

    fn put<S: Schema>(&self, key: &S::Key, value: &S::Value) -> Result<(), DbError> {
        let batch = SchemaBatch::new();
        batch.put::<S>(key, value)?;
        self.commit(batch)?;
        Ok(())
    }

    fn delete<S: Schema>(&self, mut keys: impl Iterator<Item = S::Key>) -> Result<(), DbError> {
        let batch = SchemaBatch::new();
        keys.try_for_each(|key| batch.delete::<S>(&key))?;
        self.commit(batch)
    }

    fn get_all<S: Schema>(&self) -> Result<Vec<(S::Key, S::Value)>, DbError> {
        let mut iter = self.db.iter::<S>(ReadOptions::default())?;
        iter.seek_to_first();
        Ok(iter
            .map(|e| match e {
                Ok((k, v)) => Ok((k, v)),
                Err(e) => Err(e.into()),
            })
            .collect::<Result<Vec<(S::Key, S::Value)>>>()?)
    }
}

impl<D: AugmentedData> AugDataStorage<D> for RandDb {
    fn save_aug_data(&self, aug_data: &AugData<D>) -> anyhow::Result<()> {
        Ok(self.put::<AugDataSchema<D>>(&aug_data.id(), aug_data)?)
    }

    fn save_certified_aug_data(
        &self,
        certified_aug_data: &CertifiedAugData<D>,
    ) -> anyhow::Result<()> {
        Ok(self.put::<CertifiedAugDataSchema<D>>(&certified_aug_data.id(), certified_aug_data)?)
    }

    fn get_all_aug_data(&self) -> anyhow::Result<Vec<(AugDataId, AugData<D>)>> {
        Ok(self.get_all::<AugDataSchema<D>>()?)
    }

    fn get_all_certified_aug_data(&self) -> anyhow::Result<Vec<(AugDataId, CertifiedAugData<D>)>> {
        Ok(self.get_all::<CertifiedAugDataSchema<D>>()?)
    }

    fn remove_aug_data(&self, aug_data: impl Iterator<Item = AugData<D>>) -> anyhow::Result<()> {
        Ok(self.delete::<AugDataSchema<D>>(aug_data.map(|d| d.id()))?)
    }

    fn remove_certified_aug_data(
        &self,
        certified_aug_data: impl Iterator<Item = CertifiedAugData<D>>,
    ) -> anyhow::Result<()> {
        Ok(self.delete::<CertifiedAugDataSchema<D>>(certified_aug_data.map(|d| d.id()))?)
    }
}
