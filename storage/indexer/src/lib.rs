// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

/// TODO(jill): deprecate Indexer once Indexer Async V2 is ready
mod db;
pub mod db_indexer;
pub mod db_ops;
pub mod db_v2;
pub mod event_v2_translator;
pub mod indexer_reader;
mod metrics;
pub mod utils;

use crate::db::INDEX_DB_NAME;
use aptos_config::config::RocksdbConfig;
use aptos_db_indexer_schemas::{
    metadata::{MetadataKey, MetadataValue},
    schema::{
        column_families, indexer_metadata::IndexerMetadataSchema, table_info::TableInfoSchema,
    },
};
use aptos_logger::warn;
use aptos_resource_viewer::{AnnotatedMoveValue, AptosValueAnnotator};
use aptos_rocksdb_options::gen_rocksdb_options;
use aptos_schemadb::{batch::SchemaBatch, DB};
use aptos_storage_interface::{
    db_ensure, db_other_bail, state_store::state_view::db_state_view::DbStateViewAtVersion,
    AptosDbError, DbReader, Result,
};
use aptos_types::{
    access_path::Path,
    account_address::AccountAddress,
    state_store::{
        state_key::{inner::StateKeyInner, StateKey},
        table::{TableHandle, TableInfo},
        StateView,
    },
    transaction::{AtomicVersion, Version},
    write_set::{WriteOp, WriteSet},
};
use bytes::Bytes;
use move_core_types::{
    ident_str,
    language_storage::{StructTag, TypeTag},
};
use std::{
    collections::{BTreeMap, HashMap},
    convert::TryInto,
    sync::{atomic::Ordering, Arc},
};

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
        let state_view = db_reader.state_view_at_version(Some(last_version))?;
        let annotator = AptosValueAnnotator::new(&state_view);
        self.index_with_annotator(&annotator, first_version, write_sets)
    }

    pub fn index_with_annotator<R: StateView>(
        &self,
        annotator: &AptosValueAnnotator<R>,
        first_version: Version,
        write_sets: &[&WriteSet],
    ) -> Result<()> {
        let next_version = self.next_version();
        db_ensure!(
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
            for (state_key, write_op) in write_set.write_op_iter() {
                table_info_parser.parse_write_op(state_key, write_op)?;
            }
        }

        let mut batch = SchemaBatch::new();
        match table_info_parser.finish(&mut batch) {
            Ok(_) => {},
            Err(err) => {
                aptos_logger::error!(first_version = first_version, end_version = end_version, error = ?&err);
                write_sets
                    .iter()
                    .enumerate()
                    .for_each(|(i, write_set)| {
                        aptos_logger::error!(version = first_version as usize + i, write_set = ?write_set);
                    });
                db_other_bail!("Failed to parse table info: {:?}", err);
            },
        };
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

struct TableInfoParser<'a, R> {
    indexer: &'a Indexer,
    annotator: &'a AptosValueAnnotator<'a, R>,
    result: HashMap<TableHandle, TableInfo>,
    pending_on: HashMap<TableHandle, Vec<Bytes>>,
}

impl<'a, R: StateView> TableInfoParser<'a, R> {
    pub fn new(indexer: &'a Indexer, annotator: &'a AptosValueAnnotator<R>) -> Self {
        Self {
            indexer,
            annotator,
            result: HashMap::new(),
            pending_on: HashMap::new(),
        }
    }

    pub fn parse_write_op(&mut self, state_key: &'a StateKey, write_op: &'a WriteOp) -> Result<()> {
        if let Some(bytes) = write_op.bytes() {
            match state_key.inner() {
                StateKeyInner::AccessPath(access_path) => {
                    let path: Path = (&access_path.path).try_into()?;
                    match path {
                        Path::Code(_) => (),
                        Path::Resource(struct_tag) => self.parse_struct(struct_tag, bytes)?,
                        Path::ResourceGroup(_struct_tag) => self.parse_resource_group(bytes)?,
                    }
                },
                StateKeyInner::TableItem { handle, .. } => self.parse_table_item(*handle, bytes)?,
                StateKeyInner::Raw(_) => (),
            }
        }
        Ok(())
    }

    fn parse_struct(&mut self, struct_tag: StructTag, bytes: &Bytes) -> Result<()> {
        self.parse_move_value(
            &self
                .annotator
                .view_value(&TypeTag::Struct(Box::new(struct_tag)), bytes)?,
        )
    }

    fn parse_resource_group(&mut self, bytes: &Bytes) -> Result<()> {
        type ResourceGroup = BTreeMap<StructTag, Bytes>;

        for (struct_tag, bytes) in bcs::from_bytes::<ResourceGroup>(bytes)? {
            self.parse_struct(struct_tag, &bytes)?;
        }
        Ok(())
    }

    fn parse_table_item(&mut self, handle: TableHandle, bytes: &Bytes) -> Result<()> {
        match self.get_table_info(handle)? {
            Some(table_info) => {
                self.parse_move_value(&self.annotator.view_value(&table_info.value_type, bytes)?)?;
            },
            None => {
                self.pending_on
                    .entry(handle)
                    .or_default()
                    .push(bytes.clone());
            },
        }
        Ok(())
    }

    fn parse_move_value(&mut self, move_value: &AnnotatedMoveValue) -> Result<()> {
        match move_value {
            AnnotatedMoveValue::Vector(_type_tag, items) => {
                for item in items {
                    self.parse_move_value(item)?;
                }
            },
            AnnotatedMoveValue::Struct(struct_value) => {
                let struct_tag = &struct_value.ty_tag;
                if Self::is_table(struct_tag) {
                    assert_eq!(struct_tag.type_args.len(), 2);
                    let table_info = TableInfo {
                        key_type: struct_tag.type_args[0].clone(),
                        value_type: struct_tag.type_args[1].clone(),
                    };
                    let table_handle = match &struct_value.value[0] {
                        (name, AnnotatedMoveValue::Address(handle)) => {
                            assert_eq!(name.as_ref(), ident_str!("handle"));
                            TableHandle(*handle)
                        },
                        _ => db_other_bail!("Table struct malformed. {:?}", struct_value),
                    };
                    self.save_table_info(table_handle, table_info)?;
                } else {
                    for (_identifier, field) in &struct_value.value {
                        self.parse_move_value(field)?;
                    }
                }
            },
            AnnotatedMoveValue::RawStruct(struct_value) => {
                for val in &struct_value.field_values {
                    self.parse_move_value(val)?
                }
            },
            AnnotatedMoveValue::Closure(closure_value) => {
                for capture in &closure_value.captured {
                    self.parse_move_value(capture)?
                }
            },

            // there won't be tables in primitives
            AnnotatedMoveValue::U8(_) => {},
            AnnotatedMoveValue::U16(_) => {},
            AnnotatedMoveValue::U32(_) => {},
            AnnotatedMoveValue::U64(_) => {},
            AnnotatedMoveValue::U128(_) => {},
            AnnotatedMoveValue::U256(_) => {},
            AnnotatedMoveValue::Bool(_) => {},
            AnnotatedMoveValue::Address(_) => {},
            AnnotatedMoveValue::Bytes(_) => {},
        }
        Ok(())
    }

    fn save_table_info(&mut self, handle: TableHandle, info: TableInfo) -> Result<()> {
        if self.get_table_info(handle)?.is_none() {
            self.result.insert(handle, info);
            if let Some(pending_items) = self.pending_on.remove(&handle) {
                for bytes in pending_items {
                    self.parse_table_item(handle, &bytes)?;
                }
            }
        }
        Ok(())
    }

    fn is_table(struct_tag: &StructTag) -> bool {
        struct_tag.address == AccountAddress::ONE
            && struct_tag.module.as_ident_str() == ident_str!("table")
            && struct_tag.name.as_ident_str() == ident_str!("Table")
    }

    fn get_table_info(&self, handle: TableHandle) -> Result<Option<TableInfo>> {
        match self.result.get(&handle) {
            Some(table_info) => Ok(Some(table_info.clone())),
            None => self.indexer.get_table_info(handle),
        }
    }

    fn finish(self, batch: &mut SchemaBatch) -> Result<bool> {
        db_ensure!(
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
