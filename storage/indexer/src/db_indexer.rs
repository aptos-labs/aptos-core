// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::utils::PrefixedStateValueIterator;
use aptos_config::config::internal_indexer_db_config::InternalIndexerDBConfig;
use aptos_db_indexer_schemas::{
    metadata::{MetadataKey, MetadataValue, StateSnapshotProgress},
    schema::{
        event_by_key::EventByKeySchema, event_by_version::EventByVersionSchema,
        indexer_metadata::InternalIndexerMetadataSchema, state_keys::StateKeysSchema,
        transaction_by_account::TransactionByAccountSchema,
        translated_v1_event::TranslatedV1EventSchema,
    },
    utils::{
        error_if_too_many_requested, get_first_seq_num_and_limit, AccountTransactionVersionIter,
        MAX_REQUEST_LIMIT,
    },
};
use aptos_schemadb::{SchemaBatch, DB};
use aptos_storage_interface::{
    db_ensure as ensure, db_other_bail as bail, state_view::LatestDbStateCheckpointView,
    AptosDbError, DbReader, Result,
};
use aptos_types::{
    account_address::AccountAddress,
    account_config::{CoinStoreResource, DepositEvent, DEPOSIT_EVENT_TYPE},
    coin_deposit::{CoinDeposit, COIN_DEPOSIT_TYPE_STR},
    contract_event::{ContractEvent, ContractEventV1, ContractEventV2, EventWithVersion},
    event::EventKey,
    indexer::indexer_db_reader::Order,
    state_store::{
        state_key::{prefix::StateKeyPrefix, StateKey},
        state_value::StateValue,
        TStateView,
    },
    transaction::{AccountTransactionsWithProof, Transaction, Version},
    write_set::{TransactionWrite, WriteSet},
    DummyCoinType,
};
use move_core_types::language_storage::StructTag;
use std::{
    cmp::min,
    str::FromStr,
    sync::{
        mpsc::{self, Receiver, Sender},
        Arc,
    },
    thread,
};

pub struct DBCommitter {
    db: Arc<DB>,
    receiver: Receiver<Option<SchemaBatch>>,
}

impl DBCommitter {
    pub fn new(db: Arc<DB>, receiver: Receiver<Option<SchemaBatch>>) -> Self {
        Self { db, receiver }
    }

    pub fn run(&self) {
        loop {
            let batch_opt = self
                .receiver
                .recv()
                .expect("Failed to receive batch from DB Indexer");
            if let Some(batch) = batch_opt {
                self.db
                    .write_schemas(batch)
                    .expect("Failed to write batch to indexer db");
            } else {
                break;
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct InternalIndexerDB {
    db: Arc<DB>,
    config: InternalIndexerDBConfig,
}

impl InternalIndexerDB {
    pub fn new(db: Arc<DB>, config: InternalIndexerDBConfig) -> Self {
        Self { db, config }
    }

    pub fn write_keys_to_indexer_db(
        &self,
        keys: &Vec<StateKey>,
        snapshot_version: Version,
        progress: StateSnapshotProgress,
    ) -> Result<()> {
        // add state value to internal indexer
        let batch = SchemaBatch::new();
        for state_key in keys {
            batch.put::<StateKeysSchema>(state_key, &())?;
        }

        batch.put::<InternalIndexerMetadataSchema>(
            &MetadataKey::StateSnapshotRestoreProgress(snapshot_version),
            &MetadataValue::StateSnapshotProgress(progress),
        )?;
        self.db.write_schemas(batch)?;
        Ok(())
    }

    pub fn get_persisted_version(&self) -> Result<Option<Version>> {
        self.get_version(&MetadataKey::LatestVersion)
    }

    pub fn get_event_version(&self) -> Result<Option<Version>> {
        self.get_version(&MetadataKey::EventVersion)
    }

    pub fn get_state_version(&self) -> Result<Option<Version>> {
        self.get_version(&MetadataKey::StateVersion)
    }

    pub fn get_transaction_version(&self) -> Result<Option<Version>> {
        self.get_version(&MetadataKey::TransactionVersion)
    }

    pub fn event_enabled(&self) -> bool {
        self.config.enable_event
    }

    pub fn transaction_enabled(&self) -> bool {
        self.config.enable_transaction
    }

    pub fn statekeys_enabled(&self) -> bool {
        self.config.enable_statekeys
    }

    pub fn get_inner_db_ref(&self) -> &Arc<DB> {
        &self.db
    }

    pub fn get_inner_db_clone(&self) -> Arc<DB> {
        Arc::clone(&self.db)
    }

    pub fn get_restore_progress(&self, version: Version) -> Result<Option<StateSnapshotProgress>> {
        Ok(self
            .db
            .get::<InternalIndexerMetadataSchema>(&MetadataKey::StateSnapshotRestoreProgress(
                version,
            ))?
            .map(|e| e.expect_state_snapshot_progress()))
    }

    pub fn ensure_cover_ledger_version(&self, ledger_version: Version) -> Result<()> {
        let indexer_latest_version = self.get_persisted_version()?;
        if let Some(indexer_latest_version) = indexer_latest_version {
            if indexer_latest_version >= ledger_version {
                return Ok(());
            }
        }

        bail!("ledger version too new")
    }

    pub fn get_account_transaction_version_iter(
        &self,
        address: AccountAddress,
        min_seq_num: u64,
        num_versions: u64,
        ledger_version: Version,
    ) -> Result<AccountTransactionVersionIter> {
        let mut iter = self.db.iter::<TransactionByAccountSchema>()?;
        iter.seek(&(address, min_seq_num))?;
        Ok(AccountTransactionVersionIter::new(
            iter,
            address,
            min_seq_num
                .checked_add(num_versions)
                .ok_or(AptosDbError::TooManyRequested(min_seq_num, num_versions))?,
            ledger_version,
        ))
    }

    pub fn get_latest_sequence_number(
        &self,
        ledger_version: Version,
        event_key: &EventKey,
    ) -> Result<Option<u64>> {
        let mut iter = self.db.iter::<EventByVersionSchema>()?;
        iter.seek_for_prev(&(*event_key, ledger_version, u64::max_value()))?;

        Ok(iter.next().transpose()?.and_then(
            |((key, _version, seq), _idx)| if &key == event_key { Some(seq) } else { None },
        ))
    }

    pub fn get_next_sequence_number(&self, event_key: &EventKey) -> Result<u64> {
        let mut iter = self.db.iter::<EventByKeySchema>()?;
        iter.seek_for_prev(&(*event_key, u64::max_value()))?;

        Ok(iter.next().transpose()?.map_or(
            0,
            |((key, seq), _)| if &key == event_key { seq + 1 } else { 0 },
        ))
    }

    /// Given `event_key` and `start_seq_num`, returns events identified by transaction version and
    /// index among all events emitted by the same transaction. Result won't contain records with a
    /// transaction version > `ledger_version` and is in ascending order.
    pub fn lookup_events_by_key(
        &self,
        event_key: &EventKey,
        start_seq_num: u64,
        limit: u64,
        ledger_version: u64,
    ) -> Result<
        Vec<(
            u64,     // sequence number
            Version, // transaction version it belongs to
            u64,     // index among events for the same transaction
        )>,
    > {
        println!("[jpark][db_indexer.rs][lookup_events_by_key] event_key, start_seq_num, limit, ledger_version: {:?}, {:?}, {:?}, {:?}", event_key, start_seq_num, limit, ledger_version);
        let mut iter = self.db.iter::<EventByKeySchema>()?;
        iter.seek(&(*event_key, start_seq_num))?;

        let mut result = Vec::new();
        let mut cur_seq = start_seq_num;
        for res in iter.take(limit as usize) {
            let ((path, seq), (ver, idx)) = res?;
            println!("[jpark][db_indexer.rs][lookup_events_by_key] path, seq, ver, idx: {:?}, {:?}, {:?}, {:?}", path, seq, ver, idx);
            if path != *event_key || ver > ledger_version {
                break;
            }
            if seq != cur_seq {
                let msg = if cur_seq == start_seq_num {
                    "First requested event is probably pruned."
                } else {
                    "DB corruption: Sequence number not continuous."
                };
                bail!("{} expected: {}, actual: {}", msg, cur_seq, seq);
            }
            println!("[jpark][db_indexer.rs][lookup_events_by_key] pushed");
            result.push((seq, ver, idx));
            cur_seq += 1;
        }

        Ok(result)
    }

    #[cfg(any(test, feature = "fuzzing"))]
    pub fn get_restore_version_and_progress(
        &self,
    ) -> Result<Option<(Version, StateSnapshotProgress)>> {
        let mut iter = self.db.iter::<InternalIndexerMetadataSchema>()?;
        iter.seek_to_first();
        let mut last_version = None;
        let mut last_progress = None;
        for res in iter {
            let (key, value) = res?;
            if let MetadataKey::StateSnapshotRestoreProgress(version) = key {
                last_version = Some(version);
                last_progress = Some(value.expect_state_snapshot_progress());
            }
        }
        Ok(last_version.map(|version| (version, last_progress.unwrap())))
    }

    #[cfg(any(test, feature = "fuzzing"))]
    pub fn get_state_keys(&self, prefix: &StateKeyPrefix) -> Result<Vec<StateKey>> {
        let mut iter = self.db.iter::<StateKeysSchema>()?;
        iter.seek_to_first();
        Ok(iter
            .map(|res| res.unwrap().0)
            .filter(|k| prefix.is_prefix(k).unwrap())
            .collect())
    }

    #[cfg(any(test, feature = "fuzzing"))]
    pub fn get_event_by_key_iter(
        &self,
    ) -> Result<Box<dyn Iterator<Item = (EventKey, u64, u64, u64)> + '_>> {
        let mut iter = self.db.iter::<EventByKeySchema>()?;
        iter.seek_to_first();
        Ok(Box::new(iter.map(|res| {
            let ((event_key, seq_num), (txn_version, idx)) = res.unwrap();
            (event_key, txn_version, seq_num, idx)
        })))
    }

    fn get_version(&self, key: &MetadataKey) -> Result<Option<Version>> {
        Ok(self
            .db
            .get::<InternalIndexerMetadataSchema>(key)?
            .map(|v| v.expect_version()))
    }

    pub fn get_translated_v1_event_by_version_and_index(
        &self,
        version: Version,
        index: u64,
    ) -> Result<ContractEventV1> {
        self.db
            .get::<TranslatedV1EventSchema>(&(version, index))?
            .ok_or_else(|| AptosDbError::NotFound(format!("Event {} of Txn {}", index, version)))
    }
}

pub struct DBIndexer {
    pub indexer_db: InternalIndexerDB,
    pub main_db_reader: Arc<dyn DbReader>,
    sender: Sender<Option<SchemaBatch>>,
    committer_handle: Option<thread::JoinHandle<()>>,
}

impl Drop for DBIndexer {
    fn drop(&mut self) {
        if let Some(handle) = self.committer_handle.take() {
            self.sender
                .send(None)
                .expect("Failed to send None to DBIndexer committer");
            handle
                .join()
                .expect("DBIndexer committer thread fails to join");
        }
    }
}

impl DBIndexer {
    pub fn new(indexer_db: InternalIndexerDB, db_reader: Arc<dyn DbReader>) -> Self {
        let (sender, reciver) = mpsc::channel();

        let db = indexer_db.get_inner_db_ref().to_owned();
        let committer_handle = thread::spawn(move || {
            let committer = DBCommitter::new(db, reciver);
            committer.run();
        });

        Self {
            indexer_db,
            main_db_reader: db_reader,
            sender,
            committer_handle: Some(committer_handle),
        }
    }

    pub fn get_main_db_lowest_viable_version(&self) -> Result<Version> {
        self.main_db_reader
            .get_first_txn_version()
            .transpose()
            .expect("main db lowest viable version doesn't exist")
    }

    fn get_main_db_iter(
        &self,
        start_version: Version,
        num_transactions: u64,
    ) -> Result<impl Iterator<Item = Result<(Transaction, Vec<ContractEvent>, WriteSet)>> + '_>
    {
        let txn_iter = self
            .main_db_reader
            .get_transaction_iterator(start_version, num_transactions)?;
        let event_vec_iter = self
            .main_db_reader
            .get_events_iterator(start_version, num_transactions)?;
        let writeset_iter = self
            .main_db_reader
            .get_write_set_iterator(start_version, num_transactions)?;
        let zipped = txn_iter.zip(event_vec_iter).zip(writeset_iter).map(
            |((txn_res, event_vec_res), writeset_res)| {
                let txn = txn_res?;
                let event_vec = event_vec_res?;
                let writeset = writeset_res?;
                Ok((txn, event_vec, writeset))
            },
        );
        Ok(zipped)
    }

    fn get_num_of_transactions(&self, version: Version) -> Result<u64> {
        let highest_version = self.main_db_reader.ensure_synced_version()?;
        if version > highest_version {
            // In case main db is not synced yet or recreated
            return Ok(0);
        }
        // we want to include the last transaction since the iterator interface will is right exclusive.
        let num_of_transaction = min(
            self.indexer_db.config.batch_size as u64,
            highest_version + 1 - version,
        );
        Ok(num_of_transaction)
    }

    pub fn process_a_batch(&self, start_version: Version) -> Result<Version> {
        let mut version = start_version;
        let num_transactions = self.get_num_of_transactions(version)?;
        let mut db_iter = self.get_main_db_iter(version, num_transactions)?;
        let batch = SchemaBatch::new();
        db_iter.try_for_each(|res| {
            let (txn, events, writeset) = res?;
            if let Some(txn) = txn.try_as_signed_user_txn() {
                if self.indexer_db.transaction_enabled() {
                    batch.put::<TransactionByAccountSchema>(
                        &(txn.sender(), txn.sequence_number()),
                        &version,
                    )?;
                }
            }
            // println!("[jpark][db_indexer.rs] check point 0");
            if self.indexer_db.event_enabled() {
                // println!("[jpark][db_indexer.rs] check point 1");
                events.iter().enumerate().for_each(|(idx, event)| {
                    if !event.is_new_epoch_event()
                        && !event.is_new_block_event()
                        && !event.is_new_block_event()
                    {
                        println!("[jpark][db_indexer.rs] event: {:?}", event);
                    }
                    if let ContractEvent::V1(v1) = event {
                        // println!("[jpark][db_indexer.rs] check point 2");
                        batch
                            .put::<EventByKeySchema>(
                                &(*v1.key(), v1.sequence_number()),
                                &(version, idx as u64),
                            )
                            .expect("Failed to put events by key to a batch");
                        batch
                            .put::<EventByVersionSchema>(
                                &(*v1.key(), version, v1.sequence_number()),
                                &(idx as u64),
                            )
                            .expect("Failed to put events by version to a batch");
                    }
                    let is_event_v2_translation_enabled: bool = true;
                    if is_event_v2_translation_enabled {
                        if let ContractEvent::V2(v2) = event {
                            println!("[jpark][db_indexer.rs] check point 3");
                            if let Some(translated_v1_event) = self
                                .translate_event_v2_to_v1(v2)
                                .expect("Failure in translating event")
                            {
                                println!("[jpark][db_indexer.rs] check point 4");
                                println!(
                                    "[jpark][db_indexer.rs] translated_v1_event: {:?}",
                                    translated_v1_event
                                );
                                let key = *translated_v1_event.key();
                                let sequence_number = translated_v1_event.sequence_number();
                                batch
                                    .put::<EventByKeySchema>(
                                        &(key, sequence_number),
                                        &(version, idx as u64),
                                    )
                                    .expect("Failed to put events by key to a batch");
                                batch
                                    .put::<EventByVersionSchema>(
                                        &(key, version, sequence_number),
                                        &(idx as u64),
                                    )
                                    .expect("Failed to put events by version to a batch");
                                batch
                                    .put::<TranslatedV1EventSchema>(
                                        &(version, idx as u64),
                                        &translated_v1_event,
                                    )
                                    .expect("Failed to put translated v1 events to a batch");
                            }
                        }
                    }
                });
            }

            if self.indexer_db.statekeys_enabled() {
                writeset.iter().for_each(|(state_key, write_op)| {
                    if write_op.is_creation() {
                        batch
                            .put::<StateKeysSchema>(state_key, &())
                            .expect("Failed to put state keys to a batch");
                    }
                });
            }
            version += 1;
            Ok::<(), AptosDbError>(())
        })?;
        assert_eq!(num_transactions, version - start_version);
        if self.indexer_db.transaction_enabled() {
            batch.put::<InternalIndexerMetadataSchema>(
                &MetadataKey::TransactionVersion,
                &MetadataValue::Version(version - 1),
            )?;
        }
        if self.indexer_db.event_enabled() {
            batch.put::<InternalIndexerMetadataSchema>(
                &MetadataKey::EventVersion,
                &MetadataValue::Version(version - 1),
            )?;
        }
        if self.indexer_db.statekeys_enabled() {
            batch.put::<InternalIndexerMetadataSchema>(
                &MetadataKey::StateVersion,
                &MetadataValue::Version(version - 1),
            )?;
        }
        batch.put::<InternalIndexerMetadataSchema>(
            &MetadataKey::LatestVersion,
            &MetadataValue::Version(version - 1),
        )?;
        self.sender
            .send(Some(batch))
            .map_err(|e| AptosDbError::Other(e.to_string()))?;
        Ok(version)
    }

    fn get_resource(
        &self,
        address: &AccountAddress,
        struct_tag_str: &str,
    ) -> Result<Option<StateValue>> {
        println!(
            "[jpark][db_indexer.rs][get_resource] address: {:?}, struct_tag_str: {:?}",
            address, struct_tag_str
        );
        let state_view = self
            .main_db_reader
            .latest_state_checkpoint_view()
            .expect("Failed to get state view");
        println!("[jpark][db_indexer.rs][get_resource] check point 0");
        let struct_tag = StructTag::from_str(struct_tag_str)?;
        println!(
            "[jpark][db_indexer.rs][get_resource] struct_tag: {:?}",
            struct_tag
        );
        let state_key = StateKey::resource(address, &struct_tag)?;
        println!(
            "[jpark][db_indexer.rs][get_resource] state_key: {:?}",
            state_key
        );
        let maybe_state_value = state_view.get_state_value(&state_key)?;
        println!(
            "[jpark][db_indexer.rs][get_resource] maybe_state_value: {:?}",
            maybe_state_value
        );
        Ok(maybe_state_value)
    }

    fn translate_event_v2_to_v1(&self, v2: &ContractEventV2) -> Result<Option<ContractEventV1>> {
        println!(
            "[jpark][db_indexer.rs][translate_event_v2_to_v1] type_tag: {:?}",
            v2.type_tag()
        );
        println!(
            "[jpark][db_indexer.rs][translate_event_v2_to_v1] type_tag.as_str: {:?}",
            v2.type_tag().to_canonical_string().as_str()
        );
        match v2.type_tag().to_canonical_string().as_str() {
            COIN_DEPOSIT_TYPE_STR => {
                println!("[jpark][db_indexer.rs][translate_event_v2_to_v1] check point 0");
                let coin_deposit = CoinDeposit::try_from_bytes(v2.event_data())?;
                println!("[jpark][db_indexer.rs][translate_event_v2_to_v1] check point 1");
                println!(
                    "[jpark][db_indexer.rs][translate_event_v2_to_v1] coin_deposit: {:?}",
                    coin_deposit
                );
                let struct_tag_str = format!("0x1::coin::CoinStore<{}>", coin_deposit.coin_type());
                println!(
                    "[jpark][db_indexer.rs][translate_event_v2_to_v1] struct_tag_str: {:?}",
                    struct_tag_str
                );
                // We can use `DummyCoinType` as it does not affect the correctness of deserialization.
                let state_value = self
                    .get_resource(coin_deposit.account(), &struct_tag_str)?
                    .expect("Event handle resource not found");
                println!("[jpark][db_indexer.rs][translate_event_v2_to_v1] check point 2");
                let coin_store_resource: CoinStoreResource<DummyCoinType> =
                    bcs::from_bytes(state_value.bytes())?;
                println!("[jpark][db_indexer.rs][translate_event_v2_to_v1] check point 3");
                let key = *coin_store_resource.deposit_events().key();
                let sequence_number = self.indexer_db.get_next_sequence_number(&key)?;
                println!("[jpark][db_indexer.rs][translate_event_v2_to_v1] check point 4");
                let deposit_event = DepositEvent::new(coin_deposit.amount());
                Ok(Some(ContractEventV1::new(
                    key,
                    sequence_number,
                    DEPOSIT_EVENT_TYPE.clone(),
                    bcs::to_bytes(&deposit_event)?,
                )))
            },
            _ => Ok(None),
        }
    }

    pub fn get_account_transactions(
        &self,
        address: AccountAddress,
        start_seq_num: u64,
        limit: u64,
        include_events: bool,
        ledger_version: Version,
    ) -> Result<AccountTransactionsWithProof> {
        self.indexer_db
            .ensure_cover_ledger_version(ledger_version)?;
        error_if_too_many_requested(limit, MAX_REQUEST_LIMIT)?;

        let txns_with_proofs = self
            .indexer_db
            .get_account_transaction_version_iter(address, start_seq_num, limit, ledger_version)?
            .map(|result| {
                let (_seq_num, txn_version) = result?;
                self.main_db_reader.get_transaction_by_version(
                    txn_version,
                    ledger_version,
                    include_events,
                )
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(AccountTransactionsWithProof::new(txns_with_proofs))
    }

    pub fn get_prefixed_state_value_iterator(
        &self,
        key_prefix: &StateKeyPrefix,
        cursor: Option<&StateKey>,
        ledger_version: Version,
    ) -> Result<impl Iterator<Item = anyhow::Result<(StateKey, StateValue)>> + '_> {
        self.indexer_db
            .ensure_cover_ledger_version(ledger_version)?;
        PrefixedStateValueIterator::new(
            self.main_db_reader.clone(),
            self.indexer_db.get_inner_db_ref(),
            key_prefix.clone(),
            cursor.cloned(),
            ledger_version,
        )
    }

    pub fn get_events(
        &self,
        event_key: &EventKey,
        start: u64,
        order: Order,
        limit: u64,
        ledger_version: Version,
    ) -> Result<Vec<EventWithVersion>> {
        self.indexer_db
            .ensure_cover_ledger_version(ledger_version)?;
        self.get_events_by_event_key(event_key, start, order, limit, ledger_version)
    }

    pub fn get_events_by_event_key(
        &self,
        event_key: &EventKey,
        start_seq_num: u64,
        order: Order,
        limit: u64,
        ledger_version: Version,
    ) -> Result<Vec<EventWithVersion>> {
        self.indexer_db
            .ensure_cover_ledger_version(ledger_version)?;
        error_if_too_many_requested(limit, MAX_REQUEST_LIMIT)?;
        let get_latest = order == Order::Descending && start_seq_num == u64::max_value();

        let cursor = if get_latest {
            // Caller wants the latest, figure out the latest seq_num.
            // In the case of no events on that path, use 0 and expect empty result below.
            self.indexer_db
                .get_latest_sequence_number(ledger_version, event_key)?
                .unwrap_or(0)
        } else {
            start_seq_num
        };

        println!(
            "[jpark][db_indexer.rs][get_events_by_event_key] event_key, start_seq_num, cursor, order, limit, ledger_version: {:?}, {:?}, {:?}, {:?}, {:?}, {:?}",
            event_key, start_seq_num, cursor, order, limit, ledger_version
        );
        // Convert requested range and order to a range in ascending order.
        let (first_seq, real_limit) = get_first_seq_num_and_limit(order, cursor, limit)?;

        // Query the index.
        let mut event_indices = self.indexer_db.lookup_events_by_key(
            event_key,
            first_seq,
            real_limit,
            ledger_version,
        )?;

        // When descending, it's possible that user is asking for something beyond the latest
        // sequence number, in which case we will consider it a bad request and return an empty
        // list.
        // For example, if the latest sequence number is 100, and the caller is asking for 110 to
        // 90, we will get 90 to 100 from the index lookup above. Seeing that the last item
        // is 100 instead of 110 tells us 110 is out of bound.
        if order == Order::Descending {
            if let Some((seq_num, _, _)) = event_indices.last() {
                if *seq_num < cursor {
                    event_indices = Vec::new();
                }
            }
        }

        println!("[jpark][db_indexer.rs][get_events_by_event_key] check point 0");
        let mut events_with_version = event_indices
            .into_iter()
            .map(|(seq, ver, idx)| {
                let event = match self
                    .main_db_reader
                    .get_event_by_version_and_index(ver, idx)?
                {
                    event @ ContractEvent::V1(_) => event,
                    ContractEvent::V2(_) => ContractEvent::V1(
                        self.indexer_db
                            .get_translated_v1_event_by_version_and_index(ver, idx)?,
                    ),
                };
                println!("[jpark][db_indexer.rs][get_events_by_event_key] check point 1");
                println!("[jpark][db_indexer.rs][get_events_by_event_key] event: {:?}", event);
                let v0 = match &event {
                    ContractEvent::V1(event) => event,
                    ContractEvent::V2(_) => bail!("Unexpected module event"),
                };
                println!("[jpark][db_indexer.rs][get_events_by_event_key] check point 2");
                println!("[jpark][db_indexer.rs][get_events_by_event_key] seq: {}, v0.sequence_number(): {}", seq, v0.sequence_number());
                ensure!(
                    seq == v0.sequence_number(),
                    "Index broken, expected seq:{}, actual:{}",
                    seq,
                    v0.sequence_number()
                );
                println!("[jpark][db_indexer.rs][get_events_by_event_key] check point 3");
                Ok(EventWithVersion::new(ver, event))
            })
            .collect::<Result<Vec<_>>>()?;
        if order == Order::Descending {
            events_with_version.reverse();
        }

        Ok(events_with_version)
    }
}
