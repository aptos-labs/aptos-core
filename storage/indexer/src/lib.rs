// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

mod db;
mod metadata;
mod schema;

use crate::{
    db::INDEX_DB_NAME,
    metadata::{MetadataKey, MetadataValue},
    schema::{
        column_families, indexer_metadata::IndexerMetadataSchema, table_info::TableInfoSchema,
    },
};
use anyhow::{bail, ensure, Result};
use aptos_config::config::RocksdbConfig;
use aptos_logger::warn;
use aptos_rocksdb_options::gen_rocksdb_options;
use aptos_types::{
    access_path::Path,
    account_address::AccountAddress,
    state_store::{
        state_key::StateKey,
        table::{TableHandle, TableInfo},
    },
    transaction::{AtomicVersion, Version},
    write_set::{WriteOp, WriteSet},
};
use aptos_vm::data_cache::{AsMoveResolver, StorageAdapter};
use move_deps::{
    move_core_types::{
        identifier::IdentStr,
        language_storage::{StructTag, TypeTag},
    },
    move_resource_viewer::{AnnotatedMoveValue, MoveValueAnnotator},
};
use schemadb::{SchemaBatch, DB};
use std::{
    collections::HashMap,
    convert::TryInto,
    sync::{atomic::Ordering, Arc},
};
use storage_interface::{state_view::DbStateView, DbReader};

#[derive(Debug)]
pub struct Indexer {
    db: DB,
    next_version: AtomicVersion,
}

impl Indexer {
    pub fn open(
        db_root_path: impl AsRef<std::path::Path>,
        rocksdb_config: RocksdbConfig,
    ) -> Result<Self> {
        let db_path = db_root_path.as_ref().join(INDEX_DB_NAME);

        let db = DB::open(
            db_path,
            "index_db",
            column_families(),
            &gen_rocksdb_options(&rocksdb_config, false),
        )?;

        let next_version = db
            .get::<IndexerMetadataSchema>(&MetadataKey::LatestVersion)?
            .map_or(0, |v| v.expect_version());

        Ok(Self {
            db,
            next_version: AtomicVersion::new(next_version),
        })
    }

    pub fn index(
        &self,
        db_reader: Arc<dyn DbReader>,
        first_version: Version,
        write_sets: &[&WriteSet],
    ) -> Result<()> {
        let last_version = first_version + write_sets.len() as Version;
        let state_view = DbStateView {
            db: db_reader,
            version: Some(last_version),
        };
        let resolver = state_view.as_move_resolver();
        let annotator = MoveValueAnnotator::new(&resolver);
        self.index_with_annotator(&annotator, first_version, write_sets)
    }

    pub fn index_with_annotator(
        &self,
        annotator: &MoveValueAnnotator<StorageAdapter<DbStateView>>,
        first_version: Version,
        write_sets: &[&WriteSet],
    ) -> Result<()> {
        let next_version = self.next_version();
        ensure!(
            first_version <= next_version,
            "Indexer expects to see continuous transaction versions. Expecting: {}, got: {}",
            next_version,
            first_version,
        );
        let end_version = first_version + write_sets.len() as Version;
        if end_version <= next_version {
            warn!(
                "Seeing old transactions. Expecting version: {}, got {} transactions starting from version {}.",
                next_version,
                write_sets.len(),
                first_version,
            );
            return Ok(());
        }

        let mut table_info_parser = TableInfoParser::new(self, annotator);
        for write_set in write_sets {
            for (state_key, write_op) in write_set.iter() {
                table_info_parser.parse_write_op(state_key, write_op)?;
            }
        }

        let mut batch = SchemaBatch::new();
        table_info_parser.finish(&mut batch)?;
        batch.put::<IndexerMetadataSchema>(
            &MetadataKey::LatestVersion,
            &MetadataValue::Version(end_version - 1),
        )?;
        self.db.write_schemas(batch)?;
        self.next_version.store(end_version, Ordering::Relaxed);

        Ok(())
    }

    pub fn next_version(&self) -> Version {
        self.next_version.load(Ordering::Relaxed)
    }

    pub fn get_table_info(&self, handle: TableHandle) -> Result<Option<TableInfo>> {
        self.db.get::<TableInfoSchema>(&handle)
    }
}

struct TableInfoParser<'a> {
    indexer: &'a Indexer,
    annotator: &'a MoveValueAnnotator<'a, StorageAdapter<'a, DbStateView>>,
    result: HashMap<TableHandle, TableInfo>,
    pending_on: HashMap<TableHandle, Vec<&'a [u8]>>,
}

impl<'a> TableInfoParser<'a> {
    pub fn new(
        indexer: &'a Indexer,
        annotator: &'a MoveValueAnnotator<StorageAdapter<DbStateView>>,
    ) -> Self {
        Self {
            indexer,
            annotator,
            result: HashMap::new(),
            pending_on: HashMap::new(),
        }
    }

    pub fn parse_write_op(&mut self, state_key: &'a StateKey, write_op: &'a WriteOp) -> Result<()> {
        match write_op {
            WriteOp::Modification(bytes) | WriteOp::Creation(bytes) => match state_key {
                StateKey::AccessPath(access_path) => {
                    let path: Path = (&access_path.path).try_into()?;
                    match path {
                        Path::Code(_) => (),
                        Path::Resource(struct_tag) => self.parse_struct(struct_tag, bytes)?,
                    }
                }
                StateKey::TableItem { handle, .. } => self.parse_table_item(*handle, bytes)?,
                StateKey::Raw(_) => (),
            },
            WriteOp::Deletion => (),
        }
        Ok(())
    }

    fn parse_struct(&mut self, struct_tag: StructTag, bytes: &[u8]) -> Result<()> {
        self.parse_move_value(
            &self
                .annotator
                .view_value(&TypeTag::Struct(struct_tag), bytes)?,
        )
    }

    fn parse_table_item(&mut self, handle: TableHandle, bytes: &'a [u8]) -> Result<()> {
        match self.get_table_info(handle)? {
            Some(table_info) => {
                self.parse_move_value(&self.annotator.view_value(&table_info.value_type, bytes)?)?;
            }
            None => {
                self.pending_on
                    .entry(handle)
                    .or_insert_with(Vec::new)
                    .push(bytes);
            }
        }
        Ok(())
    }

    fn parse_move_value(&mut self, move_value: &AnnotatedMoveValue) -> Result<()> {
        match move_value {
            AnnotatedMoveValue::Vector(_type_tag, items) => {
                for item in items {
                    self.parse_move_value(item)?;
                }
            }
            AnnotatedMoveValue::Struct(struct_value) => {
                let struct_tag = &struct_value.type_;
                if Self::is_table(struct_tag) {
                    assert_eq!(struct_tag.type_params.len(), 2);
                    let table_info = TableInfo {
                        key_type: struct_tag.type_params[0].clone(),
                        value_type: struct_tag.type_params[1].clone(),
                    };
                    let table_handle = match &struct_value.value[0] {
                        (name, AnnotatedMoveValue::Address(handle)) => {
                            assert_eq!(name.as_ref(), IdentStr::new("handle").unwrap());
                            TableHandle(*handle)
                        }
                        _ => bail!("Table struct malformed. {:?}", struct_value),
                    };
                    self.save_table_info(table_handle, table_info)?;
                } else {
                    for (_identifier, field) in &struct_value.value {
                        self.parse_move_value(field)?;
                    }
                }
            }

            // there won't be tables in primitives
            AnnotatedMoveValue::U8(_) => {}
            AnnotatedMoveValue::U64(_) => {}
            AnnotatedMoveValue::U128(_) => {}
            AnnotatedMoveValue::Bool(_) => {}
            AnnotatedMoveValue::Address(_) => {}
            AnnotatedMoveValue::Bytes(_) => {}
        }
        Ok(())
    }

    fn save_table_info(&mut self, handle: TableHandle, info: TableInfo) -> Result<()> {
        if self.get_table_info(handle)?.is_none() {
            self.result.insert(handle, info);
            if let Some(pending_items) = self.pending_on.remove(&handle) {
                for bytes in pending_items {
                    self.parse_table_item(handle, bytes)?;
                }
            }
        }
        Ok(())
    }

    fn is_table(struct_tag: &StructTag) -> bool {
        struct_tag.address == AccountAddress::ONE
            && struct_tag.module.as_ref() == IdentStr::new("table").unwrap()
            && struct_tag.name.as_ref() == IdentStr::new("Table").unwrap()
    }

    fn get_table_info(&self, handle: TableHandle) -> Result<Option<TableInfo>> {
        match self.result.get(&handle) {
            Some(table_info) => Ok(Some(table_info.clone())),
            None => self.indexer.get_table_info(handle),
        }
    }

    fn finish(self, batch: &mut SchemaBatch) -> Result<bool> {
        ensure!(
            self.pending_on.is_empty(),
            "There is still pending table items to parse due to unknown table info for table handles: {:?}",
            self.pending_on.keys(),
        );

        if self.result.is_empty() {
            Ok(false)
        } else {
            self.result
                .into_iter()
                .try_for_each(|(table_handle, table_info)| {
                    batch.put::<TableInfoSchema>(&table_handle, &table_info)
                })?;
            Ok(true)
        }
    }
}
