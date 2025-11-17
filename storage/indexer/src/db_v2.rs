// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

/// This file is a copy of the file storage/indexer/src/lib.rs.
/// At the end of the migration to migrate table info mapping
/// from storage critical path to indexer, the other file will be removed
/// and this file will be moved to /ecosystem/indexer-grpc/indexer-grpc-table-info.
use aptos_db_indexer_schemas::{
    metadata::{MetadataKey, MetadataValue},
    schema::{indexer_metadata::IndexerMetadataSchema, table_info::TableInfoSchema},
};
use aptos_logger::{info, sample, sample::SampleRate};
use aptos_resource_viewer::{AptosValueAnnotator, MoveTableInfo};
use aptos_schemadb::{batch::SchemaBatch, DB};
use aptos_storage_interface::{
    db_other_bail as bail, state_store::state_view::db_state_view::DbStateViewAtVersion,
    AptosDbError, DbReader, Result,
};
use aptos_types::{
    access_path::Path,
    state_store::{
        state_key::{inner::StateKeyInner, StateKey},
        table::{TableHandle, TableInfo},
        StateView,
    },
    transaction::Version,
    write_set::{WriteOp, WriteSet},
};
use bytes::Bytes;
use dashmap::{DashMap, DashSet};
use move_core_types::language_storage::{StructTag, TypeTag};
use std::{
    collections::{BTreeMap, HashMap},
    fs,
    path::PathBuf,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};

const TABLE_INFO_RETRY_TIME_MILLIS: u64 = 10;

#[derive(Debug)]
pub struct IndexerAsyncV2 {
    pub db: DB,
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
    pub fn new(db: DB) -> Result<Self> {
        let next_version = db
            .get::<IndexerMetadataSchema>(&MetadataKey::LatestVersion)?
            .map_or(0, |v| v.expect_version());

        Ok(Self {
            db,
            next_version: AtomicU64::new(next_version),
            pending_on: DashMap::new(),
        })
    }

    pub fn index_table_info(
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

    /// Index write sets with the move annotator to parse obscure table handle and key value types
    /// After the current batch's parsed, write the mapping to the rocksdb, also update the next version to be processed
    pub fn index_with_annotator<R: StateView>(
        &self,
        annotator: &AptosValueAnnotator<R>,
        first_version: Version,
        write_sets: &[&WriteSet],
    ) -> Result<()> {
        let end_version = first_version + write_sets.len() as Version;
        let mut table_info_parser = TableInfoParser::new(self, annotator, &self.pending_on);
        for write_set in write_sets {
            for (state_key, write_op) in write_set.write_op_iter() {
                table_info_parser.collect_table_info_from_write_op(state_key, write_op)?;
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
        self.db.put::<IndexerMetadataSchema>(
            &MetadataKey::LatestVersion,
            &MetadataValue::Version(end_version - 1),
        )?;
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

    pub fn next_version(&self) -> Version {
        self.db
            .get::<IndexerMetadataSchema>(&MetadataKey::LatestVersion)
            .unwrap()
            .map_or(0, |v| v.expect_version())
    }

    pub fn get_table_info(&self, handle: TableHandle) -> Result<Option<TableInfo>> {
        self.db.get::<TableInfoSchema>(&handle)
    }

    pub fn get_table_info_with_retry(&self, handle: TableHandle) -> Result<Option<TableInfo>> {
        let mut retried = 0;
        loop {
            if let Ok(Some(table_info)) = self.get_table_info(handle) {
                return Ok(Some(table_info));
            }

            // Log the first failure, and then sample subsequent failures to avoid log spam
            if retried == 0 {
                log_table_info_failure(handle, retried);
            } else {
                sample!(
                    SampleRate::Duration(Duration::from_secs(1)),
                    log_table_info_failure(handle, retried)
                );
            }

            retried += 1;
            std::thread::sleep(Duration::from_millis(TABLE_INFO_RETRY_TIME_MILLIS));
        }
    }

    pub fn is_indexer_async_v2_pending_on_empty(&self) -> bool {
        if !self.pending_on.is_empty() {
            let pending_keys: Vec<TableHandle> =
                self.pending_on.iter().map(|entry| *entry.key()).collect();
            aptos_logger::warn!(
                "There are still pending table items to parse due to unknown table info for table handles: {:?}",
                pending_keys
            );
            false
        } else {
            true
        }
    }

    pub fn clear_pending_on(&self) {
        self.pending_on.clear()
    }

    pub fn create_checkpoint(&self, path: &PathBuf) -> Result<()> {
        fs::remove_dir_all(path).unwrap_or(());
        self.db.create_checkpoint(path)
    }
}

/// Logs a failure to retrieve table information
fn log_table_info_failure(handle: TableHandle, retried: u64) {
    info!(
        retry_count = retried,
        table_handle = handle.0.to_canonical_string(),
        "[DB] Failed to get table info",
    )
}

struct TableInfoParser<'a, R> {
    indexer_async_v2: &'a IndexerAsyncV2,
    annotator: &'a AptosValueAnnotator<'a, R>,
    result: HashMap<TableHandle, TableInfo>,
    pending_on: &'a DashMap<TableHandle, DashSet<Bytes>>,
}

impl<'a, R: StateView> TableInfoParser<'a, R> {
    pub fn new(
        indexer_async_v2: &'a IndexerAsyncV2,
        annotator: &'a AptosValueAnnotator<R>,
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
    pub fn collect_table_info_from_write_op(
        &mut self,
        state_key: &'a StateKey,
        write_op: &'a WriteOp,
    ) -> Result<()> {
        if let Some(bytes) = write_op.bytes() {
            match state_key.inner() {
                StateKeyInner::AccessPath(access_path) => {
                    let path: Path = (&access_path.path).try_into()?;
                    match path {
                        Path::Code(_) => (),
                        Path::Resource(struct_tag) => {
                            self.collect_table_info_from_struct(struct_tag, bytes)?
                        },
                        Path::ResourceGroup(_struct_tag) => {
                            self.collect_table_info_from_resource_group(bytes)?
                        },
                    }
                },
                StateKeyInner::TableItem { handle, .. } => {
                    self.collect_table_info_from_table_item(*handle, bytes)?
                },
                StateKeyInner::Raw(_) => (),
            }
        }
        Ok(())
    }

    fn collect_table_info_from_struct(
        &mut self,
        struct_tag: StructTag,
        bytes: &Bytes,
    ) -> Result<()> {
        let ty_tag = TypeTag::Struct(Box::new(struct_tag));
        let mut infos = vec![];
        self.annotator
            .collect_table_info(&ty_tag, bytes, &mut infos)?;
        self.process_table_infos(infos)
    }

    fn collect_table_info_from_resource_group(&mut self, bytes: &Bytes) -> Result<()> {
        type ResourceGroup = BTreeMap<StructTag, Bytes>;

        for (struct_tag, bytes) in bcs::from_bytes::<ResourceGroup>(bytes)? {
            self.collect_table_info_from_struct(struct_tag, &bytes)?;
        }
        Ok(())
    }

    fn collect_table_info_from_table_item(
        &mut self,
        handle: TableHandle,
        bytes: &Bytes,
    ) -> Result<()> {
        match self.get_table_info(handle)? {
            Some(table_info) => {
                let mut infos = vec![];
                self.annotator
                    .collect_table_info(&table_info.value_type, bytes, &mut infos)?;
                self.process_table_infos(infos)?
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

    fn process_table_infos(&mut self, infos: Vec<MoveTableInfo>) -> Result<()> {
        for MoveTableInfo {
            key_type,
            value_type,
            handle,
        } in infos
        {
            self.save_table_info(TableHandle(handle), TableInfo {
                key_type,
                value_type,
            })?
        }
        Ok(())
    }

    fn save_table_info(&mut self, handle: TableHandle, info: TableInfo) -> Result<()> {
        if self.get_table_info(handle)?.is_none() {
            self.result.insert(handle, info);
            if let Some(pending_items) = self.pending_on.remove(&handle) {
                for bytes in pending_items.1 {
                    self.collect_table_info_from_table_item(handle, &bytes)?;
                }
            }
        }
        Ok(())
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
