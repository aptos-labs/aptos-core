// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

/// TODO(#20246): this file was planned to move to `indexer-grpc-table-info` as part
/// of a migration that stalled; consolidate code placement of the indexer stack.
use aptos_db_indexer_schemas::{
    metadata::{MetadataKey, MetadataValue},
    schema::{indexer_metadata::IndexerMetadataSchema, table_info::TableInfoSchema},
};
use aptos_logger::{info, sample, sample::SampleRate, warn};
use aptos_resource_viewer::{AptosValueAnnotator, MoveTableInfo};
use aptos_schemadb::{batch::SchemaBatch, DB};
use aptos_storage_interface::{
    db_other_bail as bail, state_store::state_view::db_state_view::DbStateViewAtVersion, DbReader,
    Result,
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
    time::{Duration, Instant},
};

const TABLE_INFO_RETRY_TIME_MILLIS: u64 = 10;

/// How long `get_table_info_with_retry` waits for the info of a table to be indexed.
///
/// A reader can race the async parser: a transaction is committed before the infos of its tables
/// are indexed, which takes milliseconds when the indexer keeps up. The info of some handles is
/// never indexed though: the tables created by a simulated transaction are in no committed
/// transaction, and a handle that the parser couldn't resolve is dropped (see `finalize_batch`).
const TABLE_INFO_RETRY_TIMEOUT: Duration = Duration::from_secs(1);

/// How long after giving up on the info of a table `get_table_info_with_retry` doesn't wait for it
/// again. A simulated transaction can write many items to a table that it creates, and a lookup
/// for each of them would wait for the info of the table: they would add up to far more than
/// `TABLE_INFO_RETRY_TIMEOUT`.
const TABLE_INFO_GIVE_UP_MEMORY: Duration = Duration::from_secs(10);

/// The most tables that are remembered as given up on. The tables of simulated transactions are
/// new every time, so the memory is cleaned up when it is full.
const MAX_GIVEN_UP_TABLES: usize = 1024;

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
    // The tables whose info wasn't indexed in time, and when `get_table_info_with_timeout` gave up
    // waiting for it. At most `MAX_GIVEN_UP_TABLES`.
    given_up_on: DashMap<TableHandle, Instant>,
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
            given_up_on: DashMap::new(),
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

    /// Advances past a processed batch. Any table handles that couldn't be resolved are
    /// dropped (their items left undecoded) instead of blocking progress, so parsing never
    /// gets stuck on a transaction it can't fully handle.
    pub fn finalize_batch(&self, end_version: Version) -> Result<()> {
        if !self.is_indexer_async_v2_pending_on_empty() {
            warn!(
                end_version = end_version,
                "[DB] Dropping unresolved table handles and advancing past this batch."
            );
            self.clear_pending_on();
        }
        self.update_next_version(end_version + 1)
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

    /// Looks up the info of a table, waiting up to `TABLE_INFO_RETRY_TIMEOUT` for it to be
    /// indexed. See `get_table_info_with_timeout`.
    pub fn get_table_info_with_retry(&self, handle: TableHandle) -> Result<Option<TableInfo>> {
        self.get_table_info_with_timeout(handle, TABLE_INFO_RETRY_TIMEOUT)
    }

    /// Looks up the info of a table, retrying until it is indexed or `timeout` has passed.
    ///
    /// If the info isn't there by then, the result is that of the last lookup: `Ok(None)`, or the
    /// error if the database can't be read.
    pub fn get_table_info_with_timeout(
        &self,
        handle: TableHandle,
        timeout: Duration,
    ) -> Result<Option<TableInfo>> {
        let deadline = Instant::now() + timeout;
        let mut retried = 0;
        loop {
            let lookup = self.get_table_info(handle);
            if let Ok(Some(_)) = lookup {
                return lookup;
            }

            if retried == 0 && self.gave_up_on_recently(handle) {
                return lookup;
            }

            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                self.remember_giving_up_on(handle);
                sample!(
                    SampleRate::Duration(Duration::from_secs(1)),
                    log_table_info_given_up(handle, retried, timeout)
                );
                return lookup;
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
            std::thread::sleep(remaining.min(Duration::from_millis(TABLE_INFO_RETRY_TIME_MILLIS)));
        }
    }

    fn gave_up_on_recently(&self, handle: TableHandle) -> bool {
        self.given_up_on
            .get(&handle)
            .is_some_and(|gave_up_at| gave_up_at.elapsed() < TABLE_INFO_GIVE_UP_MEMORY)
    }

    fn remember_giving_up_on(&self, handle: TableHandle) {
        if self.given_up_on.len() >= MAX_GIVEN_UP_TABLES {
            self.given_up_on
                .retain(|_, gave_up_at| gave_up_at.elapsed() < TABLE_INFO_GIVE_UP_MEMORY);
            if self.given_up_on.len() >= MAX_GIVEN_UP_TABLES {
                // All of them are recent. Not remembering this one only means that it is waited
                // for again, which is what happened before they were remembered.
                return;
            }
        }
        self.given_up_on.insert(handle, Instant::now());
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

/// Logs that the wait for the information of a table was given up
fn log_table_info_given_up(handle: TableHandle, retried: u64, timeout: Duration) {
    warn!(
        retry_count = retried,
        timeout_ms = timeout.as_millis() as u64,
        table_handle = handle.0.to_canonical_string(),
        "[DB] Gave up waiting for table info to be indexed",
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
                // Out of scope for table-info collection.
                StateKeyInner::TradingNative(_) => (),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db_ops::open_db;
    use aptos_config::config::RocksdbConfig;
    use aptos_temppath::TempPath;
    use move_core_types::account_address::AccountAddress;

    fn new_test_indexer(tmp: &TempPath) -> IndexerAsyncV2 {
        tmp.create_as_dir().unwrap();
        let db = open_db(
            tmp.path(),
            &RocksdbConfig::default(),
            /*readonly=*/ false,
        )
        .unwrap();
        IndexerAsyncV2::new(db).unwrap()
    }

    fn table_info() -> TableInfo {
        TableInfo {
            key_type: TypeTag::U64,
            value_type: TypeTag::Vector(Box::new(TypeTag::U8)),
        }
    }

    fn table_handle(i: u64) -> TableHandle {
        let mut bytes = [0u8; AccountAddress::LENGTH];
        bytes[..8].copy_from_slice(&i.to_be_bytes());
        TableHandle(AccountAddress::new(bytes))
    }

    /// A time `age` ago, for entries of `given_up_on` that were made a while ago.
    fn ago(age: Duration) -> Instant {
        Instant::now()
            .checked_sub(age)
            .expect("the system has been up for longer than the age")
    }

    /// An unresolved handle is dropped and the processed version advances, rather than
    /// blocking progress.
    #[test]
    fn finalize_batch_advances_past_unresolvable_handle() {
        let tmp = TempPath::new();
        let indexer = new_test_indexer(&tmp);

        let handle = TableHandle(AccountAddress::ONE);
        indexer
            .pending_on
            .entry(handle)
            .or_default()
            .insert(Bytes::from_static(b"orphan-table-item"));
        assert!(
            !indexer.is_indexer_async_v2_pending_on_empty(),
            "precondition: the orphan handle should be parked"
        );

        let end_version: Version = 41;
        indexer
            .finalize_batch(end_version)
            .expect("finalize_batch must not fail on an unresolvable handle");

        assert!(
            indexer.is_indexer_async_v2_pending_on_empty(),
            "the unresolvable handle should have been dropped"
        );
        assert_eq!(indexer.next_version(), end_version);
    }

    /// The info of a table that is never indexed, e.g. one that a simulated transaction
    /// creates, must not make the reader wait forever.
    #[test]
    fn lookup_of_a_table_that_is_never_indexed_returns() {
        let tmp = TempPath::new();
        let indexer = Arc::new(new_test_indexer(&tmp));

        // Look it up on another thread, so that a lookup that never returns fails the test
        // instead of hanging it.
        let (sender, receiver) = std::sync::mpsc::channel();
        {
            let indexer = indexer.clone();
            std::thread::spawn(move || {
                let handle = TableHandle(AccountAddress::ONE);
                let _ = sender.send(indexer.get_table_info_with_retry(handle));
            });
        }

        let result = receiver
            .recv_timeout(Duration::from_secs(10))
            .expect("the lookup of a table that is never indexed should give up waiting");
        assert_eq!(result.unwrap(), None);
    }

    /// The table that a simulated transaction creates can have many items, and the lookup of
    /// each of them would wait for the info of the table. They must not add up.
    #[test]
    fn giving_up_on_a_table_is_remembered() {
        let tmp = TempPath::new();
        let indexer = new_test_indexer(&tmp);
        let handle = TableHandle(AccountAddress::ONE);

        let timeout = Duration::from_millis(300);
        let started = Instant::now();
        assert_eq!(
            indexer
                .get_table_info_with_timeout(handle, timeout)
                .unwrap(),
            None
        );
        assert!(
            started.elapsed() >= timeout,
            "the first lookup should wait for the info of the table"
        );

        // The next lookup doesn't wait, however long the caller is willing to wait.
        let started = Instant::now();
        assert_eq!(
            indexer
                .get_table_info_with_timeout(handle, Duration::from_secs(10))
                .unwrap(),
            None
        );
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "a table that was given up on a moment ago should not be waited for again"
        );
    }

    /// Indexing a table takes a moment after its transaction is committed, and a reader that
    /// asks in that moment has to get the info.
    #[test]
    fn table_info_that_is_indexed_while_waiting_is_returned() {
        let tmp = TempPath::new();
        let indexer = new_test_indexer(&tmp);
        let handle = table_handle(1);

        std::thread::scope(|scope| {
            scope.spawn(|| {
                std::thread::sleep(Duration::from_millis(100));
                indexer
                    .db
                    .put::<TableInfoSchema>(&handle, &table_info())
                    .unwrap();
            });

            let started = Instant::now();
            let found = indexer
                .get_table_info_with_timeout(handle, Duration::from_secs(10))
                .unwrap();
            assert_eq!(found, Some(table_info()));
            assert!(started.elapsed() < Duration::from_secs(5));
        });
    }

    #[test]
    fn indexed_table_info_is_returned_without_waiting() {
        let tmp = TempPath::new();
        let indexer = new_test_indexer(&tmp);
        let handle = table_handle(1);
        indexer
            .db
            .put::<TableInfoSchema>(&handle, &table_info())
            .unwrap();

        let started = Instant::now();
        let found = indexer
            .get_table_info_with_timeout(handle, Duration::from_secs(60))
            .unwrap();
        assert_eq!(found, Some(table_info()));
        assert!(started.elapsed() < Duration::from_secs(5));
    }

    /// Remembering that a table was given up on only skips the waiting: the table is returned as
    /// soon as it is indexed.
    #[test]
    fn table_that_was_given_up_on_is_returned_once_it_is_indexed() {
        let tmp = TempPath::new();
        let indexer = new_test_indexer(&tmp);
        let handle = table_handle(1);

        assert_eq!(
            indexer
                .get_table_info_with_timeout(handle, Duration::from_millis(50))
                .unwrap(),
            None
        );
        assert!(indexer.gave_up_on_recently(handle));

        indexer
            .db
            .put::<TableInfoSchema>(&handle, &table_info())
            .unwrap();
        assert_eq!(
            indexer
                .get_table_info_with_timeout(handle, Duration::ZERO)
                .unwrap(),
            Some(table_info())
        );
    }

    #[test]
    fn table_that_was_given_up_on_long_ago_is_waited_for_again() {
        let tmp = TempPath::new();
        let indexer = new_test_indexer(&tmp);
        let handle = table_handle(1);
        let long_ago = ago(TABLE_INFO_GIVE_UP_MEMORY + Duration::from_secs(1));
        indexer.given_up_on.insert(handle, long_ago);
        assert!(!indexer.gave_up_on_recently(handle));

        let timeout = Duration::from_millis(200);
        let started = Instant::now();
        assert_eq!(
            indexer
                .get_table_info_with_timeout(handle, timeout)
                .unwrap(),
            None
        );
        assert!(started.elapsed() >= timeout);
    }

    #[test]
    fn tables_given_up_on_are_remembered_up_to_a_limit() {
        let tmp = TempPath::new();
        let indexer = new_test_indexer(&tmp);

        for i in 0..(MAX_GIVEN_UP_TABLES as u64 + 50) {
            // Doesn't wait: gives up at once.
            let handle = table_handle(i);
            let found = indexer.get_table_info_with_timeout(handle, Duration::ZERO);
            assert_eq!(found.unwrap(), None);
        }
        assert_eq!(indexer.given_up_on.len(), MAX_GIVEN_UP_TABLES);
    }

    #[test]
    fn old_tables_are_forgotten_to_make_room_for_new_ones() {
        let tmp = TempPath::new();
        let indexer = new_test_indexer(&tmp);
        let long_ago = ago(TABLE_INFO_GIVE_UP_MEMORY + Duration::from_secs(1));
        for i in 0..MAX_GIVEN_UP_TABLES as u64 {
            indexer.given_up_on.insert(table_handle(i), long_ago);
        }

        let new_handle = table_handle(MAX_GIVEN_UP_TABLES as u64);
        let found = indexer.get_table_info_with_timeout(new_handle, Duration::ZERO);
        assert_eq!(found.unwrap(), None);

        assert!(indexer.gave_up_on_recently(new_handle));
        assert!(indexer.given_up_on.len() <= MAX_GIVEN_UP_TABLES);
    }
}
