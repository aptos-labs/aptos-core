// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::utils::PrefixedStateValueIterator;
use aptos_config::config::internal_indexer_db_config::InternalIndexerDBConfig;
use aptos_db_indexer_schemas::{
    metadata::{MetadataKey, MetadataValue},
    schema::{
        event_by_key::EventByKeySchema, event_by_version::EventByVersionSchema,
        indexer_metadata::InternalIndexerMetadataSchema, state_keys::StateKeysSchema,
        transaction_by_account::TransactionByAccountSchema,
    },
    utils::{
        error_if_too_many_requested, get_first_seq_num_and_limit, AccountTransactionVersionIter,
        MAX_REQUEST_LIMIT,
    },
};
use aptos_schemadb::{SchemaBatch, DB};
use aptos_storage_interface::{
    db_ensure as ensure, db_other_bail as bail, AptosDbError, DbReader, Result,
};
use aptos_types::{
    account_address::AccountAddress,
    contract_event::{ContractEvent, EventWithVersion},
    event::EventKey,
    indexer::indexer_db_reader::Order,
    state_store::{
        state_key::{prefix::StateKeyPrefix, StateKey},
        state_value::StateValue,
    },
    transaction::{AccountTransactionsWithProof, Transaction, Version},
    write_set::{TransactionWrite, WriteSet},
};
use std::{
    cmp::min,
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

pub struct DBIndexer {
    pub db: Arc<DB>,
    pub main_db_reader: Arc<dyn DbReader>,
    config: InternalIndexerDBConfig,
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
    pub fn new(
        db: Arc<DB>,
        db_reader: Arc<dyn DbReader>,
        config: &InternalIndexerDBConfig,
    ) -> Self {
        let (sender, reciver) = mpsc::channel();

        let db_clone = db.clone();
        let committer_handle = thread::spawn(move || {
            let committer = DBCommitter::new(db, reciver);
            committer.run();
        });

        Self {
            db: db_clone,
            main_db_reader: db_reader,
            config: *config,
            sender,
            committer_handle: Some(committer_handle),
        }
    }

    pub fn ensure_cover_ledger_version(&self, ledger_version: Version) -> Result<()> {
        let indexer_latest_version = self.get_persisted_version()?;
        ensure!(
            indexer_latest_version >= ledger_version,
            "ledger version too new"
        );
        Ok(())
    }

    pub fn get_persisted_version(&self) -> Result<Version> {
        // read the latest key from the db
        self.db
            .get::<InternalIndexerMetadataSchema>(&MetadataKey::LatestVersion)?
            .map_or(Ok(0), |metavalue| Ok(metavalue.expect_version()))
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
        let highest_version = self.main_db_reader.get_synced_version()?;
        if version > highest_version {
            // In case main db is not synced yet or recreated
            return Ok(0);
        }
        // we want to include the last transaction since the iterator interface will is right exclusive.
        let num_of_transaction = min(
            (self.config.batch_size + 1) as u64,
            highest_version + 1 - version,
        );
        Ok(num_of_transaction)
    }

    pub fn process_a_batch(&self, start_version: Option<Version>) -> Result<Version> {
        let mut version = start_version.unwrap_or(0);

        let num_transactions = self.get_num_of_transactions(version)?;
        let mut db_iter = self.get_main_db_iter(version, num_transactions)?;
        let batch = SchemaBatch::new();
        db_iter.try_for_each(|res| {
            let (txn, events, writeset) = res?;
            if let Some(txn) = txn.try_as_signed_user_txn() {
                if self.config.enable_transaction {
                    batch.put::<TransactionByAccountSchema>(
                        &(txn.sender(), txn.sequence_number()),
                        &version,
                    )?;
                }
            }

            if self.config.enable_event {
                events.iter().enumerate().for_each(|(idx, event)| {
                    if let ContractEvent::V1(v1) = event {
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
                });
            }

            if self.config.enable_statekeys {
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
        assert_eq!(num_transactions, version - start_version.unwrap_or(0));
        batch.put::<InternalIndexerMetadataSchema>(
            &MetadataKey::LatestVersion,
            &MetadataValue::Version(version - 1),
        )?;
        self.sender
            .send(Some(batch))
            .map_err(|e| AptosDbError::Other(e.to_string()))?;
        Ok(version)
    }

    pub fn get_account_transaction_version_iter(
        &self,
        address: AccountAddress,
        min_seq_num: u64,
        num_versions: u64,
        ledger_version: Version,
    ) -> Result<AccountTransactionVersionIter> {
        self.ensure_cover_ledger_version(ledger_version)?;
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
        self.ensure_cover_ledger_version(ledger_version)?;

        let mut iter = self.db.iter::<EventByVersionSchema>()?;
        iter.seek_for_prev(&(*event_key, ledger_version, u64::max_value()))?;

        Ok(iter.next().transpose()?.and_then(
            |((key, _version, seq), _idx)| if &key == event_key { Some(seq) } else { None },
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
        self.ensure_cover_ledger_version(ledger_version)?;
        let mut iter = self.db.iter::<EventByKeySchema>()?;
        iter.seek(&(*event_key, start_seq_num))?;

        let mut result = Vec::new();
        let mut cur_seq = start_seq_num;
        for res in iter.take(limit as usize) {
            let ((path, seq), (ver, idx)) = res?;
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
            result.push((seq, ver, idx));
            cur_seq += 1;
        }

        Ok(result)
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

    pub fn get_events(
        &self,
        event_key: &EventKey,
        start: u64,
        order: Order,
        limit: u64,
        ledger_version: Version,
    ) -> Result<Vec<EventWithVersion>> {
        self.ensure_cover_ledger_version(ledger_version)?;
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
        self.ensure_cover_ledger_version(ledger_version)?;
        error_if_too_many_requested(limit, MAX_REQUEST_LIMIT)?;
        let get_latest = order == Order::Descending && start_seq_num == u64::max_value();

        let cursor = if get_latest {
            // Caller wants the latest, figure out the latest seq_num.
            // In the case of no events on that path, use 0 and expect empty result below.
            self.get_latest_sequence_number(ledger_version, event_key)?
                .unwrap_or(0)
        } else {
            start_seq_num
        };

        // Convert requested range and order to a range in ascending order.
        let (first_seq, real_limit) = get_first_seq_num_and_limit(order, cursor, limit)?;

        // Query the index.
        let mut event_indices =
            self.lookup_events_by_key(event_key, first_seq, real_limit, ledger_version)?;

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

        let mut events_with_version = event_indices
            .into_iter()
            .map(|(seq, ver, idx)| {
                let event = self
                    .main_db_reader
                    .get_event_by_version_and_index(ver, idx)?;
                let v0 = match &event {
                    ContractEvent::V1(event) => event,
                    ContractEvent::V2(_) => bail!("Unexpected module event"),
                };
                ensure!(
                    seq == v0.sequence_number(),
                    "Index broken, expected seq:{}, actual:{}",
                    seq,
                    v0.sequence_number()
                );
                Ok(EventWithVersion::new(ver, event))
            })
            .collect::<Result<Vec<_>>>()?;
        if order == Order::Descending {
            events_with_version.reverse();
        }

        Ok(events_with_version)
    }

    pub fn get_account_transactions(
        &self,
        address: AccountAddress,
        start_seq_num: u64,
        limit: u64,
        include_events: bool,
        ledger_version: Version,
    ) -> Result<AccountTransactionsWithProof> {
        self.ensure_cover_ledger_version(ledger_version)?;
        error_if_too_many_requested(limit, MAX_REQUEST_LIMIT)?;

        let txns_with_proofs = self
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
        self.ensure_cover_ledger_version(ledger_version)?;
        PrefixedStateValueIterator::new(
            self.main_db_reader.clone(),
            self.db.as_ref(),
            key_prefix.clone(),
            cursor.cloned(),
            ledger_version,
        )
    }
}
