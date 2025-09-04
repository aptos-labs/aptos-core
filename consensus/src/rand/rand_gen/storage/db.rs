// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    error::DbError,
    rand::rand_gen::{
        storage::{
            interface::RandStorage,
            schema::{
                AugDataSchema, CertifiedAugDataSchema, KeyPairSchema, AUG_DATA_CF_NAME,
                CERTIFIED_AUG_DATA_CF_NAME, KEY_PAIR_CF_NAME,
            },
        },
        types::{AugData, AugDataId, CertifiedAugData, TAugmentedData},
    },
};
use anyhow::Result;
use velor_logger::info;
use velor_schemadb::{batch::SchemaBatch, schema::Schema, Options, DB};
use std::{path::Path, sync::Arc, time::Instant};

pub struct RandDb {
    db: Arc<DB>,
}

pub const RAND_DB_NAME: &str = "rand_db";

impl RandDb {
    pub(crate) fn new<P: AsRef<Path> + Clone>(db_root_path: P) -> Self {
        let column_families = vec![
            KEY_PAIR_CF_NAME,
            AUG_DATA_CF_NAME,
            CERTIFIED_AUG_DATA_CF_NAME,
        ];

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
        let mut batch = SchemaBatch::new();
        batch.put::<S>(key, value)?;
        self.commit(batch)?;
        Ok(())
    }

    fn delete<S: Schema>(&self, mut keys: impl Iterator<Item = S::Key>) -> Result<(), DbError> {
        let mut batch = SchemaBatch::new();
        keys.try_for_each(|key| batch.delete::<S>(&key))?;
        self.commit(batch)
    }

    fn get_all<S: Schema>(&self) -> Result<Vec<(S::Key, S::Value)>, DbError> {
        let mut iter = self.db.iter::<S>()?;
        iter.seek_to_first();
        Ok(iter
            .filter_map(|e| match e {
                Ok((k, v)) => Some((k, v)),
                Err(_) => None,
            })
            .collect::<Vec<(S::Key, S::Value)>>())
    }
}

impl<D: TAugmentedData> RandStorage<D> for RandDb {
    fn save_key_pair_bytes(&self, epoch: u64, key_pair: Vec<u8>) -> Result<()> {
        Ok(self.put::<KeyPairSchema>(&(), &(epoch, key_pair))?)
    }

    fn save_aug_data(&self, aug_data: &AugData<D>) -> Result<()> {
        Ok(self.put::<AugDataSchema<D>>(&aug_data.id(), aug_data)?)
    }

    fn save_certified_aug_data(&self, certified_aug_data: &CertifiedAugData<D>) -> Result<()> {
        Ok(self.put::<CertifiedAugDataSchema<D>>(&certified_aug_data.id(), certified_aug_data)?)
    }

    fn get_key_pair_bytes(&self) -> Result<Option<(u64, Vec<u8>)>> {
        Ok(self.get_all::<KeyPairSchema>()?.pop().map(|(_, v)| v))
    }

    fn get_all_aug_data(&self) -> Result<Vec<(AugDataId, AugData<D>)>> {
        Ok(self.get_all::<AugDataSchema<D>>()?)
    }

    fn get_all_certified_aug_data(&self) -> Result<Vec<(AugDataId, CertifiedAugData<D>)>> {
        Ok(self.get_all::<CertifiedAugDataSchema<D>>()?)
    }

    fn remove_aug_data(&self, aug_data: Vec<AugData<D>>) -> Result<()> {
        Ok(self.delete::<AugDataSchema<D>>(aug_data.into_iter().map(|d| d.id()))?)
    }

    fn remove_certified_aug_data(
        &self,
        certified_aug_data: Vec<CertifiedAugData<D>>,
    ) -> Result<()> {
        Ok(self
            .delete::<CertifiedAugDataSchema<D>>(certified_aug_data.into_iter().map(|d| d.id()))?)
    }
}
