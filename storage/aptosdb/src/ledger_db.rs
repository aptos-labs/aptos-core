// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]
#![allow(dead_code)]

use crate::{
    db_options::{
        event_db_column_families, gen_event_cfds, gen_ledger_cfds, gen_ledger_metadata_cfds,
        gen_transaction_accumulator_cfds, gen_transaction_cfds, gen_transaction_info_cfds,
        gen_write_set_cfds, ledger_db_column_families, ledger_metadata_db_column_families,
        transaction_accumulator_db_column_families, transaction_db_column_families,
        transaction_info_db_column_families, write_set_db_column_families,
    },
    schema::db_metadata::{DbMetadataKey, DbMetadataSchema, DbMetadataValue},
};
use anyhow::Result;
use aptos_config::config::{RocksdbConfig, RocksdbConfigs};
use aptos_logger::prelude::info;
use aptos_rocksdb_options::gen_rocksdb_options;
use aptos_schemadb::{ColumnFamilyDescriptor, ColumnFamilyName, SchemaBatch, DB};
use aptos_types::transaction::Version;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

pub const LEDGER_DB_FOLDER_NAME: &str = "ledger_db";
pub const LEDGER_DB_NAME: &str = "ledger_db";
pub const LEDGER_METADATA_DB_NAME: &str = "ledger_metadata_db";
pub const EVENT_DB_NAME: &str = "event_db";
pub const TRANSACTION_ACCUMULATOR_DB_NAME: &str = "transaction_accumulator_db";
pub const TRANSACTION_DB_NAME: &str = "transaction_db";
pub const TRANSACTION_INFO_DB_NAME: &str = "transaction_info_db";
pub const WRITE_SET_DB_NAME: &str = "write_set_db";

#[derive(Debug)]
pub struct LedgerDbSchemaBatches {
    pub ledger_metadata_db_batches: SchemaBatch,
    pub event_db_batches: SchemaBatch,
    pub transaction_accumulator_db_batches: SchemaBatch,
    pub transaction_db_batches: SchemaBatch,
    pub transaction_info_db_batches: SchemaBatch,
    pub write_set_db_batches: SchemaBatch,
}

impl Default for LedgerDbSchemaBatches {
    fn default() -> Self {
        Self {
            ledger_metadata_db_batches: SchemaBatch::new(),
            event_db_batches: SchemaBatch::new(),
            transaction_accumulator_db_batches: SchemaBatch::new(),
            transaction_db_batches: SchemaBatch::new(),
            transaction_info_db_batches: SchemaBatch::new(),
            write_set_db_batches: SchemaBatch::new(),
        }
    }
}

impl LedgerDbSchemaBatches {
    pub fn new() -> Self {
        Self::default()
    }
}

#[derive(Debug)]
pub struct LedgerDb {
    ledger_metadata_db: Arc<DB>,
    event_db: Arc<DB>,
    transaction_accumulator_db: Arc<DB>,
    transaction_db: Arc<DB>,
    transaction_info_db: Arc<DB>,
    write_set_db: Arc<DB>,
}

impl LedgerDb {
    // TODO(grao): Support more flexible path to make it easier for people to put different shards
    // on different disks.
    pub(crate) fn new<P: AsRef<Path>>(
        db_root_path: P,
        rocksdb_configs: RocksdbConfigs,
        readonly: bool,
    ) -> Result<Self> {
        let ledger_metadata_db_path =
            Self::metadata_db_path(db_root_path.as_ref(), rocksdb_configs.split_ledger_db);
        let ledger_metadata_db = Arc::new(Self::open_rocksdb(
            ledger_metadata_db_path.clone(),
            if rocksdb_configs.split_ledger_db {
                LEDGER_METADATA_DB_NAME
            } else {
                LEDGER_DB_NAME
            },
            &rocksdb_configs.ledger_db_config,
            readonly,
        )?);

        info!(
            ledger_metadata_db_path = ledger_metadata_db_path,
            "Opened ledger metadata db!"
        );

        if !rocksdb_configs.split_ledger_db {
            info!("Individual ledger dbs are not enabled!");
            return Ok(Self {
                ledger_metadata_db: Arc::clone(&ledger_metadata_db),
                event_db: Arc::clone(&ledger_metadata_db),
                transaction_accumulator_db: Arc::clone(&ledger_metadata_db),
                transaction_db: Arc::clone(&ledger_metadata_db),
                transaction_info_db: Arc::clone(&ledger_metadata_db),
                write_set_db: Arc::clone(&ledger_metadata_db),
            });
        }

        let ledger_db_folder = db_root_path.as_ref().join(LEDGER_DB_FOLDER_NAME);

        let event_db = Arc::new(Self::open_rocksdb(
            ledger_db_folder.join(EVENT_DB_NAME),
            EVENT_DB_NAME,
            &rocksdb_configs.ledger_db_config,
            readonly,
        )?);

        let transaction_accumulator_db = Arc::new(Self::open_rocksdb(
            ledger_db_folder.join(TRANSACTION_ACCUMULATOR_DB_NAME),
            TRANSACTION_ACCUMULATOR_DB_NAME,
            &rocksdb_configs.ledger_db_config,
            readonly,
        )?);

        let transaction_db = Arc::new(Self::open_rocksdb(
            ledger_db_folder.join(TRANSACTION_DB_NAME),
            TRANSACTION_DB_NAME,
            &rocksdb_configs.ledger_db_config,
            readonly,
        )?);

        let transaction_info_db = Arc::new(Self::open_rocksdb(
            ledger_db_folder.join(TRANSACTION_INFO_DB_NAME),
            TRANSACTION_INFO_DB_NAME,
            &rocksdb_configs.ledger_db_config,
            readonly,
        )?);

        let write_set_db = Arc::new(Self::open_rocksdb(
            ledger_db_folder.join(WRITE_SET_DB_NAME),
            WRITE_SET_DB_NAME,
            &rocksdb_configs.ledger_db_config,
            readonly,
        )?);

        // TODO(grao): Handle data inconsistency.

        Ok(Self {
            ledger_metadata_db,
            event_db,
            transaction_accumulator_db,
            transaction_db,
            transaction_info_db,
            write_set_db,
        })
    }

    pub(crate) fn create_checkpoint(
        db_root_path: impl AsRef<Path>,
        cp_root_path: impl AsRef<Path>,
        split_ledger_db: bool,
    ) -> Result<()> {
        let rocksdb_configs = RocksdbConfigs {
            split_ledger_db,
            ..Default::default()
        };
        let ledger_db = Self::new(db_root_path, rocksdb_configs, /*readonly=*/ false)?;
        let cp_ledger_db_folder = cp_root_path.as_ref().join(LEDGER_DB_FOLDER_NAME);

        info!(
            split_ledger_db = split_ledger_db,
            "Creating ledger_db checkpoint at: {cp_ledger_db_folder:?}"
        );

        std::fs::remove_dir_all(&cp_ledger_db_folder).unwrap_or(());
        if split_ledger_db {
            std::fs::create_dir_all(&cp_ledger_db_folder).unwrap_or(());
        }

        ledger_db
            .metadata_db()
            .create_checkpoint(Self::metadata_db_path(
                cp_root_path.as_ref(),
                split_ledger_db,
            ))?;

        if split_ledger_db {
            ledger_db
                .event_db()
                .create_checkpoint(cp_ledger_db_folder.join(EVENT_DB_NAME))?;
            ledger_db
                .transaction_accumulator_db()
                .create_checkpoint(cp_ledger_db_folder.join(TRANSACTION_ACCUMULATOR_DB_NAME))?;
            ledger_db
                .transaction_db()
                .create_checkpoint(cp_ledger_db_folder.join(TRANSACTION_DB_NAME))?;
            ledger_db
                .transaction_info_db()
                .create_checkpoint(cp_ledger_db_folder.join(TRANSACTION_INFO_DB_NAME))?;
            ledger_db
                .write_set_db()
                .create_checkpoint(cp_ledger_db_folder.join(WRITE_SET_DB_NAME))?;
        }

        Ok(())
    }

    // Only expect to be used by fast sync when it is finished.
    pub(crate) fn write_pruner_progress(&self, version: Version) -> Result<()> {
        info!("Fast sync is done, writing pruner progress {version} for all ledger sub pruners.");
        self.event_db.put::<DbMetadataSchema>(
            &DbMetadataKey::EventPrunerProgress,
            &DbMetadataValue::Version(version),
        )?;
        self.transaction_accumulator_db.put::<DbMetadataSchema>(
            &DbMetadataKey::TransactionAccumulatorPrunerProgress,
            &DbMetadataValue::Version(version),
        )?;
        self.transaction_db.put::<DbMetadataSchema>(
            &DbMetadataKey::TransactionPrunerProgress,
            &DbMetadataValue::Version(version),
        )?;
        self.transaction_info_db.put::<DbMetadataSchema>(
            &DbMetadataKey::TransactionInfoPrunerProgress,
            &DbMetadataValue::Version(version),
        )?;
        self.write_set_db.put::<DbMetadataSchema>(
            &DbMetadataKey::WriteSetPrunerProgress,
            &DbMetadataValue::Version(version),
        )?;
        self.ledger_metadata_db.put::<DbMetadataSchema>(
            &DbMetadataKey::LedgerPrunerProgress,
            &DbMetadataValue::Version(version),
        )
    }

    pub fn metadata_db(&self) -> &DB {
        &self.ledger_metadata_db
    }

    pub(crate) fn metadata_db_arc(&self) -> Arc<DB> {
        Arc::clone(&self.ledger_metadata_db)
    }

    pub(crate) fn event_db(&self) -> &DB {
        &self.event_db
    }

    pub(crate) fn event_db_arc(&self) -> Arc<DB> {
        Arc::clone(&self.event_db)
    }

    pub(crate) fn transaction_accumulator_db(&self) -> &DB {
        &self.transaction_accumulator_db
    }

    pub(crate) fn transaction_accumulator_db_arc(&self) -> Arc<DB> {
        Arc::clone(&self.transaction_accumulator_db)
    }

    pub(crate) fn transaction_db(&self) -> &DB {
        &self.transaction_db
    }

    pub(crate) fn transaction_db_arc(&self) -> Arc<DB> {
        Arc::clone(&self.transaction_db)
    }

    pub(crate) fn transaction_info_db(&self) -> &DB {
        &self.transaction_info_db
    }

    pub(crate) fn transaction_info_db_arc(&self) -> Arc<DB> {
        Arc::clone(&self.transaction_info_db)
    }

    pub(crate) fn write_set_db(&self) -> &DB {
        &self.write_set_db
    }

    pub(crate) fn write_set_db_arc(&self) -> Arc<DB> {
        Arc::clone(&self.write_set_db)
    }

    fn open_rocksdb(
        path: PathBuf,
        name: &str,
        db_config: &RocksdbConfig,
        readonly: bool,
    ) -> Result<DB> {
        let db = if readonly {
            DB::open_cf_readonly(
                &gen_rocksdb_options(db_config, true),
                path.clone(),
                name,
                Self::get_column_families_by_name(name),
            )?
        } else {
            DB::open_cf(
                &gen_rocksdb_options(db_config, false),
                path.clone(),
                name,
                Self::gen_cfds_by_name(db_config, name),
            )?
        };

        info!("Opened {name} at {path:?}!");

        Ok(db)
    }

    fn get_column_families_by_name(name: &str) -> Vec<ColumnFamilyName> {
        match name {
            LEDGER_DB_NAME => ledger_db_column_families(),
            LEDGER_METADATA_DB_NAME => ledger_metadata_db_column_families(),
            EVENT_DB_NAME => event_db_column_families(),
            TRANSACTION_ACCUMULATOR_DB_NAME => transaction_accumulator_db_column_families(),
            TRANSACTION_DB_NAME => transaction_db_column_families(),
            TRANSACTION_INFO_DB_NAME => transaction_info_db_column_families(),
            WRITE_SET_DB_NAME => write_set_db_column_families(),
            _ => unreachable!(),
        }
    }

    fn gen_cfds_by_name(db_config: &RocksdbConfig, name: &str) -> Vec<ColumnFamilyDescriptor> {
        match name {
            LEDGER_DB_NAME => gen_ledger_cfds(db_config),
            LEDGER_METADATA_DB_NAME => gen_ledger_metadata_cfds(db_config),
            EVENT_DB_NAME => gen_event_cfds(db_config),
            TRANSACTION_ACCUMULATOR_DB_NAME => gen_transaction_accumulator_cfds(db_config),
            TRANSACTION_DB_NAME => gen_transaction_cfds(db_config),
            TRANSACTION_INFO_DB_NAME => gen_transaction_info_cfds(db_config),
            WRITE_SET_DB_NAME => gen_write_set_cfds(db_config),
            _ => unreachable!(),
        }
    }

    fn metadata_db_path<P: AsRef<Path>>(db_root_path: P, split_ledger_db: bool) -> PathBuf {
        let ledger_db_folder = db_root_path.as_ref().join(LEDGER_DB_FOLDER_NAME);
        if split_ledger_db {
            ledger_db_folder.join("metadata")
        } else {
            ledger_db_folder
        }
    }

    pub fn write_schemas(&self, schemas: LedgerDbSchemaBatches) -> Result<()> {
        self.write_set_db
            .write_schemas(schemas.write_set_db_batches)?;
        self.transaction_info_db
            .write_schemas(schemas.transaction_info_db_batches)?;
        self.transaction_db
            .write_schemas(schemas.transaction_db_batches)?;
        self.ledger_metadata_db
            .write_schemas(schemas.ledger_metadata_db_batches)?;
        self.event_db.write_schemas(schemas.event_db_batches)?;
        self.transaction_accumulator_db
            .write_schemas(schemas.transaction_accumulator_db_batches)
    }
}
