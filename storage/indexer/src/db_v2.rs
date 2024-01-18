// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

/// This file is a copy of the file storage/indexer/src/lib.rs.
/// At the end of the migration to migrate table info mapping
/// from storage critical path to indexer, the other file will be removed
/// and this file will be moved to /ecosystem/indexer-grpc/indexer-grpc-table-info.
use crate::{
    metadata::{MetadataKey, MetadataValue},
    schema::{
        column_families, indexer_metadata::IndexerMetadataSchema, table_info::TableInfoSchema,
    },
};
use aptos_config::config::RocksdbConfig;
use aptos_logger::info;
use aptos_rocksdb_options::gen_rocksdb_options;
use aptos_schemadb::{SchemaBatch, DB};
use aptos_storage_interface::{
    db_other_bail as bail, state_view::DbStateView, AptosDbError, DbReader, Result,
};
use aptos_types::{
    access_path::Path,
    account_address::AccountAddress,
    state_store::{
        state_key::{StateKey, StateKeyInner},
        table::{TableHandle, TableInfo},
    },
    transaction::Version,
    write_set::{WriteOp, WriteSet},
};
use aptos_vm::data_cache::AsMoveResolver;
use bytes::Bytes;
use dashmap::{DashMap, DashSet};
use move_core_types::{
    ident_str,
    language_storage::{StructTag, TypeTag},
    resolver::ModuleResolver,
};
use move_resource_viewer::{AnnotatedMoveValue, MoveValueAnnotator};
use std::{
    collections::{BTreeMap, HashMap},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};

pub const INDEX_ASYNC_V2_DB_NAME: &str = "index_indexer_async_v2_db";
const TABLE_INFO_RETRY_TIME_MILLIS: u64 = 10;

#[derive(Debug)]
pub struct IndexerAsyncV2 {
    db: DB,
    // Next version to be processed
    next_version: AtomicU64,
    // It is used in the context of processing write ops and extracting table information.
    // As the code iterates through the write ops, it checks if the state key corresponds to a table item.
    // If it does, the associated bytes are added to the pending_on map under the corresponding table handle.
    // Later, when the table information becomes available, the pending items can be retrieved and processed accordingly.
    // One example could be a nested table item, parent table contains child table, so when parent table is first met and parsed,
    // is obscure and will be stored as bytes with parent table's handle, once parent table's parsed with instructions,
    // child table handle will be parsed accordingly.
    pending_on: DashMap<TableHandle, DashSet<Bytes>>,
}

impl IndexerAsyncV2 {
    /// Opens up this rocksdb to get ready for read and write when bootstraping the aptosdb
    pub fn open(
        db_root_path: impl AsRef<std::path::Path>,
        rocksdb_config: RocksdbConfig,
        pending_on: DashMap<TableHandle, DashSet<Bytes>>,
    ) -> Result<Self> {
        let db_path = db_root_path.as_ref().join(INDEX_ASYNC_V2_DB_NAME);

        let db = DB::open(
            db_path,
            "index_asnync_v2_db",
            column_families(),
            &gen_rocksdb_options(&rocksdb_config, false),
        )?;

        let next_version = db
            .get::<IndexerMetadataSchema>(&MetadataKey::LatestVersion)?
            .map_or(0, |v| v.expect_version());

        Ok(Self {
            db,
            next_version: AtomicU64::new(next_version),
            pending_on,
        })
    }

    pub fn index_table_info(
        &self,
        db_reader: Arc<dyn DbReader>,
        first_version: Version,
        write_sets: &[&WriteSet],
        end_early_if_pending_on_empty: bool,
    ) -> Result<()> {
        let last_version = first_version + write_sets.len() as Version;
        let state_view = DbStateView {
            db: db_reader,
            version: Some(last_version),
        };
        let resolver = state_view.as_move_resolver();
        let annotator = MoveValueAnnotator::new(&resolver);
        self.index_with_annotator(
            &annotator,
            first_version,
            write_sets,
            end_early_if_pending_on_empty,
        )
    }

    /// Index write sets with the move annotator to parse obscure table handle and key value types
    /// After the current batch's parsed, write the mapping to the rocksdb, also update the next version to be processed
    pub fn index_with_annotator<R: ModuleResolver>(
        &self,
        annotator: &MoveValueAnnotator<R>,
        first_version: Version,
        write_sets: &[&WriteSet],
        end_early_if_pending_on_empty: bool,
    ) -> Result<()> {
        let end_version = first_version + write_sets.len() as Version;
        let mut table_info_parser = TableInfoParser::new(self, annotator, &self.pending_on);
        'outer_loop: for write_set in write_sets {
            for (state_key, write_op) in write_set.iter() {
                table_info_parser.parse_write_op(state_key, write_op)?;
                // In the second sequential retry to parse write sets, we will end early if all pending on items are parsed
                if end_early_if_pending_on_empty && self.is_indexer_async_v2_pending_on_empty() {
                    break 'outer_loop; // This breaks out of both loops
                }
            }
        }
        let mut batch = SchemaBatch::new();
        match self.finish_table_info_parsing(&mut batch, &table_info_parser.result) {
            Ok(_) => {},
            Err(err) => {
                aptos_logger::error!(
                    first_version = first_version,
                    end_version = end_version,
                    error = ?&err,
                    "[DB] Failed to parse table info"
                );
                bail!("{}", err);
            },
        };
        self.db.write_schemas(batch)?;
        Ok(())
    }

    pub fn update_next_version(&self, end_version: u64) -> Result<()> {
        let batch = SchemaBatch::new();
        batch.put::<IndexerMetadataSchema>(
            &MetadataKey::LatestVersion,
            &MetadataValue::Version(end_version - 1),
        )?;
        self.db.write_schemas(batch)?;
        self.next_version.store(end_version, Ordering::Relaxed);
        Ok(())
    }

    /// Finishes the parsing process and writes the parsed table information to a SchemaBatch.
    pub fn finish_table_info_parsing(
        &self,
        batch: &mut SchemaBatch,
        result: &HashMap<TableHandle, TableInfo>,
    ) -> Result<()> {
        result.iter().try_for_each(|(table_handle, table_info)| {
            info!(
                table_handle = table_handle.0.to_canonical_string(),
                "[DB] Table handle written to the rocksdb successfully",
            );
            batch.put::<TableInfoSchema>(table_handle, table_info)
        })?;
        Ok(())
    }

    /// After multiple threads have processed batches of write sets, clean up the pending on items to
    /// remove any handles that have already been successfully parsed
    /// ideally pending on items should be empty after threads join, meaning that all batches have done the work
    pub fn cleanup_pending_on_items(&self) -> Result<()> {
        let pending_keys: Vec<TableHandle> =
            self.pending_on.iter().map(|entry| *entry.key()).collect();

        for handle in pending_keys.iter() {
            if self.get_table_info(*handle)?.is_some() {
                self.pending_on.remove(handle);
            }
        }

        if !self.pending_on.is_empty() {
            aptos_logger::warn!(
                "There are still pending table items to parse due to unknown table info for table handles: {:?}",
                pending_keys
            );
        }

        Ok(())
    }

    pub fn next_version(&self) -> Version {
        self.next_version.load(Ordering::Relaxed)
    }

    pub fn get_table_info(&self, handle: TableHandle) -> Result<Option<TableInfo>> {
        self.db.get::<TableInfoSchema>(&handle).map_err(Into::into)
    }

    pub fn get_table_info_with_retry(&self, handle: TableHandle) -> Result<Option<TableInfo>> {
        let mut retried = 0;
        loop {
            if let Ok(Some(table_info)) = self.get_table_info(handle) {
                return Ok(Some(table_info));
            }
            retried += 1;
            info!(
                retry_count = retried,
                table_handle = handle.0.to_canonical_string(),
                "[DB] Failed to get table info",
            );
            std::thread::sleep(Duration::from_millis(TABLE_INFO_RETRY_TIME_MILLIS));
        }
    }

    pub fn is_indexer_async_v2_pending_on_empty(&self) -> bool {
        self.pending_on.is_empty()
    }
}

struct TableInfoParser<'a, R> {
    indexer_async_v2: &'a IndexerAsyncV2,
    annotator: &'a MoveValueAnnotator<'a, R>,
    result: HashMap<TableHandle, TableInfo>,
    pending_on: &'a DashMap<TableHandle, DashSet<Bytes>>,
}

impl<'a, R: ModuleResolver> TableInfoParser<'a, R> {
    pub fn new(
        indexer_async_v2: &'a IndexerAsyncV2,
        annotator: &'a MoveValueAnnotator<R>,
        pending_on: &'a DashMap<TableHandle, DashSet<Bytes>>,
    ) -> Self {
        Self {
            indexer_async_v2,
            annotator,
            result: HashMap::new(),
            pending_on,
        }
    }

    /// Parses a write operation and extracts table information from it.
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
                    .insert(bytes.clone());
            },
        }
        Ok(())
    }

    /// Parses a write operation and extracts table information from it.
    ///
    /// The `parse_move_value` function is a recursive method that traverses
    /// through the `AnnotatedMoveValue` structure. Depending on the type of
    /// `AnnotatedMoveValue` (e.g., Vector, Struct, or primitive types), it
    /// performs different parsing actions. For Vector and Struct, it recursively
    /// calls itself to parse each element or field. This recursive approach allows
    /// the function to handle nested data structures in Move values.
    fn parse_move_value(&mut self, move_value: &AnnotatedMoveValue) -> Result<()> {
        match move_value {
            AnnotatedMoveValue::Vector(_type_tag, items) => {
                for item in items {
                    self.parse_move_value(item)?;
                }
            },
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
                            assert_eq!(name.as_ref(), ident_str!("handle"));
                            TableHandle(*handle)
                        },
                        _ => bail!("Table struct malformed. {:?}", struct_value),
                    };
                    self.save_table_info(table_handle, table_info)?;
                } else {
                    for (_identifier, field) in &struct_value.value {
                        self.parse_move_value(field)?;
                    }
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
                for bytes in pending_items.1 {
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

    /// Retrieves table information either from the in-memory results or from the database.
    ///
    /// This method first checks if the table information for the given handle exists in the
    /// in-memory `result` Dashmap. If it is found, it returns the information directly from
    /// there. If not, it fetches the table information from the database using the `IndexerAsyncV2`
    /// instance. This approach of checking in-memory cache first improves performance by avoiding
    /// unnecessary database reads.
    fn get_table_info(&self, handle: TableHandle) -> Result<Option<TableInfo>> {
        match self.result.get(&handle) {
            Some(table_info) => Ok(Some(table_info.clone())),
            None => self.indexer_async_v2.get_table_info(handle),
        }
    }
}
