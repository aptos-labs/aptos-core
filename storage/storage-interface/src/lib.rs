// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::state_store::state_view::hot_state_view::HotStateView;
use aptos_crypto::HashValue;
pub use aptos_types::indexer::indexer_db_reader::Order;
use aptos_types::{
    account_address::AccountAddress,
    account_config::NewBlockEvent,
    contract_event::{ContractEvent, EventWithVersion},
    epoch_change::EpochChangeProof,
    epoch_state::EpochState,
    event::EventKey,
    ledger_info::LedgerInfoWithSignatures,
    proof::{
        AccumulatorConsistencyProof, SparseMerkleProof, SparseMerkleProofExt,
        SparseMerkleRangeProof, TransactionAccumulatorRangeProof, TransactionAccumulatorSummary,
    },
    state_proof::StateProof,
    state_store::{
        state_key::StateKey,
        state_storage_usage::StateStorageUsage,
        state_value::{StateValue, StateValueChunkWithProof},
        table::{TableHandle, TableInfo},
    },
    transaction::{
        AccountOrderedTransactionsWithProof, IndexedTransactionSummary, PersistedAuxiliaryInfo,
        Transaction, TransactionAuxiliaryData, TransactionInfo, TransactionListWithProofV2,
        TransactionOutputListWithProofV2, TransactionToCommit, TransactionWithProof, Version,
    },
    write_set::WriteSet,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;

pub mod block_info;
pub mod chunk_to_commit;
pub mod errors;
mod ledger_summary;
mod metrics;
#[cfg(any(test, feature = "fuzzing"))]
pub mod mock;
pub mod state_store;

use crate::{
    chunk_to_commit::ChunkToCommit,
    state_store::{state::State, state_summary::StateSummary},
};
pub use aptos_types::block_info::BlockHeight;
use aptos_types::state_store::state_key::prefix::StateKeyPrefix;
pub use errors::AptosDbError;
pub use ledger_summary::LedgerSummary;

pub type Result<T, E = AptosDbError> = std::result::Result<T, E>;
// This is last line of defense against large queries slipping through external facing interfaces,
// like the API and State Sync, etc.
pub const MAX_REQUEST_LIMIT: u64 = 20_000;

pub trait StateSnapshotReceiver<K, V>: Send {
    fn add_chunk(&mut self, chunk: Vec<(K, V)>, proof: SparseMerkleRangeProof) -> Result<()>;

    fn finish(self) -> Result<()>;

    fn finish_box(self: Box<Self>) -> Result<()>;
}

#[derive(Debug, Deserialize, Error, PartialEq, Eq, Serialize)]
pub enum Error {
    #[error("Service error: {:?}", error)]
    ServiceError { error: String },

    #[error("Serialization error: {0}")]
    SerializationError(String),
}

impl From<anyhow::Error> for Error {
    fn from(error: anyhow::Error) -> Self {
        Self::ServiceError {
            error: format!("{}", error),
        }
    }
}

impl From<bcs::Error> for Error {
    fn from(error: bcs::Error) -> Self {
        Self::SerializationError(format!("{}", error))
    }
}

impl From<aptos_secure_net::Error> for Error {
    fn from(error: aptos_secure_net::Error) -> Self {
        Self::ServiceError {
            error: format!("{}", error),
        }
    }
}

macro_rules! delegate_read {
    ($(
        $(#[$($attr:meta)*])*
        fn $name:ident(&self $(, $arg: ident : $ty: ty)* $(,)?) -> $return_type:ty;
    )+) => {
        $(
            $(#[$($attr)*])*
            fn $name(&self, $($arg: $ty),*) -> $return_type {
                self.get_read_delegatee().$name($($arg),*)
            }
        )+
    };
}

/// Trait that is implemented by a DB that supports certain public (to client) read APIs
/// expected of an Aptos DB
#[allow(unused_variables)]
pub trait DbReader: Send + Sync {
    fn get_read_delegatee(&self) -> &dyn DbReader {
        unimplemented!("Implement desired method or get_delegatee().");
    }

    delegate_read!(
        /// See [AptosDB::get_epoch_ending_ledger_infos].
        ///
        /// [AptosDB::get_epoch_ending_ledger_infos]:
        /// ../aptosdb/struct.AptosDB.html#method.get_epoch_ending_ledger_infos
        fn get_epoch_ending_ledger_infos(
            &self,
            start_epoch: u64,
            end_epoch: u64,
        ) -> Result<EpochChangeProof>;

        /// See [AptosDB::get_transactions].
        ///
        /// [AptosDB::get_transactions]: ../aptosdb/struct.AptosDB.html#method.get_transactions
        fn get_transactions(
            &self,
            start_version: Version,
            batch_size: u64,
            ledger_version: Version,
            fetch_events: bool,
        ) -> Result<TransactionListWithProofV2>;

        /// See [AptosDB::get_transaction_by_hash].
        ///
        /// [AptosDB::get_transaction_by_hash]: ../aptosdb/struct.AptosDB.html#method.get_transaction_by_hash
        fn get_transaction_by_hash(
            &self,
            hash: HashValue,
            ledger_version: Version,
            fetch_events: bool,
        ) -> Result<Option<TransactionWithProof>>;

        /// See [AptosDB::get_transaction_by_version].
        ///
        /// [AptosDB::get_transaction_by_version]: ../aptosdb/struct.AptosDB.html#method.get_transaction_by_version
        fn get_transaction_by_version(
            &self,
            version: Version,
            ledger_version: Version,
            fetch_events: bool,
        ) -> Result<TransactionWithProof>;

        fn get_transaction_auxiliary_data_by_version(
            &self,
            version: Version,
        ) -> Result<Option<TransactionAuxiliaryData>>;

        /// See [AptosDB::get_persisted_auxiliary_info_iterator].
        ///
        /// [AptosDB::get_persisted_auxiliary_info_iterator]: ../aptosdb/struct.AptosDB.html#method.get_persisted_auxiliary_info_iterator
        fn get_persisted_auxiliary_info_iterator(
            &self,
            start_version: Version,
            num_persisted_auxiliary_info: usize,
        ) -> Result<Box<dyn Iterator<Item = Result<PersistedAuxiliaryInfo>> + '_>>;

        /// See [AptosDB::get_first_txn_version].
        ///
        /// [AptosDB::get_first_txn_version]: ../aptosdb/struct.AptosDB.html#method.get_first_txn_version
        fn get_first_txn_version(&self) -> Result<Option<Version>>;

        /// See [AptosDB::get_first_viable_block].
        ///
        /// [AptosDB::get_first_viable_block]: ../aptosdb/struct.AptosDB.html#method.get_first_viable_block
        fn get_first_viable_block(&self) -> Result<(Version, BlockHeight)>;

        /// See [AptosDB::get_first_write_set_version].
        ///
        /// [AptosDB::get_first_write_set_version]: ../aptosdb/struct.AptosDB.html#method.get_first_write_set_version
        fn get_first_write_set_version(&self) -> Result<Option<Version>>;

        /// See [AptosDB::get_transaction_outputs].
        ///
        /// [AptosDB::get_transaction_outputs]: ../aptosdb/struct.AptosDB.html#method.get_transaction_outputs
        fn get_transaction_outputs(
            &self,
            start_version: Version,
            limit: u64,
            ledger_version: Version,
        ) -> Result<TransactionOutputListWithProofV2>;

        /// Returns events by given event key
        fn get_events(
            &self,
            event_key: &EventKey,
            start: u64,
            order: Order,
            limit: u64,
            ledger_version: Version,
        ) -> Result<Vec<EventWithVersion>>;

        fn get_transaction_iterator(
            &self,
            start_version: Version,
            limit: u64,
        ) -> Result<Box<dyn Iterator<Item = Result<Transaction>> + '_>>;

        fn get_transaction_info_iterator(
            &self,
            start_version: Version,
            limit: u64,
        ) -> Result<Box<dyn Iterator<Item = Result<TransactionInfo>> + '_>>;

        fn get_events_iterator(
            &self,
            start_version: Version,
            limit: u64,
        ) -> Result<Box<dyn Iterator<Item = Result<Vec<ContractEvent>>> + '_>>;

        fn get_write_set_iterator(
            &self,
            start_version: Version,
            limit: u64,
        ) -> Result<Box<dyn Iterator<Item = Result<WriteSet>> + '_>>;

        fn get_transaction_accumulator_range_proof(
            &self,
            start_version: Version,
            limit: u64,
            ledger_version: Version,
        ) -> Result<TransactionAccumulatorRangeProof>;

        /// See [AptosDB::get_block_timestamp].
        ///
        /// [AptosDB::get_block_timestamp]:
        /// ../aptosdb/struct.AptosDB.html#method.get_block_timestamp
        fn get_block_timestamp(&self, version: Version) -> Result<u64>;

        /// See `AptosDB::get_latest_block_events`.
        fn get_latest_block_events(&self, num_events: usize) -> Result<Vec<EventWithVersion>>;

        /// Returns the start_version, end_version and NewBlockEvent of the block containing the input
        /// transaction version.
        fn get_block_info_by_version(
            &self,
            version: Version,
        ) -> Result<(Version, Version, NewBlockEvent)>;

        /// Returns the start_version, end_version and NewBlockEvent of the block containing the input
        /// transaction version.
        fn get_block_info_by_height(
            &self,
            height: u64,
        ) -> Result<(Version, Version, NewBlockEvent)>;

        /// Gets the version of the last transaction committed before timestamp,
        /// a committed block at or after the required timestamp must exist (otherwise it's possible
        /// the next block committed as a timestamp smaller than the one in the request).
        fn get_last_version_before_timestamp(
            &self,
            _timestamp: u64,
            _ledger_version: Version,
        ) -> Result<Version>;

        /// Gets the latest epoch state currently held in storage.
        fn get_latest_epoch_state(&self) -> Result<EpochState>;

        /// Returns the (key, value) iterator for a particular state key prefix at at desired version. This
        /// API can be used to get all resources of an account by passing the account address as the
        /// key prefix.
        fn get_prefixed_state_value_iterator(
            &self,
            key_prefix: &StateKeyPrefix,
            cursor: Option<&StateKey>,
            version: Version,
        ) -> Result<Box<dyn Iterator<Item = Result<(StateKey, StateValue)>> + '_>>;

        /// Returns the latest ledger info, if any.
        fn get_latest_ledger_info_option(&self) -> Result<Option<LedgerInfoWithSignatures>>;

        /// Returns the latest "synced" transaction version, potentially not "committed" yet.
        fn get_synced_version(&self) -> Result<Option<Version>>;

        /// Returns the latest "pre-committed" transaction version, which includes those written to
        /// the DB but yet to be certified by consensus or a verified LedgerInfo from a state sync
        /// peer.
        fn get_pre_committed_version(&self) -> Result<Option<Version>>;

        /// Returns the latest state checkpoint version if any.
        fn get_latest_state_checkpoint_version(&self) -> Result<Option<Version>>;

        /// Returns the latest state snapshot strictly before `next_version` if any.
        fn get_state_snapshot_before(
            &self,
            next_version: Version,
        ) -> Result<Option<(Version, HashValue)>>;

        /// Returns a transaction that is the `sequence_number`-th one associated with the given account. If
        /// the transaction with given `sequence_number` doesn't exist, returns `None`.
        fn get_account_ordered_transaction(
            &self,
            address: AccountAddress,
            seq_num: u64,
            include_events: bool,
            ledger_version: Version,
        ) -> Result<Option<TransactionWithProof>>;

        /// Returns the list of ordered transactions (transactions that include a sequence number)
        /// sent by an account with `address` starting
        /// at sequence number `seq_num`. Will return no more than `limit` transactions.
        /// Will ignore transactions with `txn.version > ledger_version`. Optionally
        /// fetch events for each transaction when `fetch_events` is `true`.
        fn get_account_ordered_transactions(
            &self,
            address: AccountAddress,
            seq_num: u64,
            limit: u64,
            include_events: bool,
            ledger_version: Version,
        ) -> Result<AccountOrderedTransactionsWithProof>;

        /// Returns the list of summaries of transactions committed by an account.
        /// Each transaction summary contains the sender address, transaction hash, version, replay protector
        /// of the committed transaction.
        /// If `start_version` is provided, the returned list contains transactions starting from `start_version`.
        /// Or else if `end_version` is provided, the returned list contains transactions ending at `end_version`.
        /// The returned list contains at most `limit` transactions.
        /// The returned list is always sorted by version in ascending order.
        fn get_account_transaction_summaries(
            &self,
            address: AccountAddress,
            start_version: Option<u64>,
            end_version: Option<u64>,
            limit: u64,
            ledger_version: Version,
        ) -> Result<Vec<IndexedTransactionSummary>>;

        /// Returns proof of new state for a given ledger info with signatures relative to version known
        /// to client
        fn get_state_proof_with_ledger_info(
            &self,
            known_version: u64,
            ledger_info: LedgerInfoWithSignatures,
        ) -> Result<StateProof>;

        /// Returns proof of new state relative to version known to client
        fn get_state_proof(&self, known_version: u64) -> Result<StateProof>;

        /// Gets the state value by state key at version.
        /// See [AptosDB::get_state_value_by_version].
        ///
        /// [AptosDB::get_state_value_by_version]:
        /// ../aptosdb/struct.AptosDB.html#method.get_state_value_by_version
        fn get_state_value_by_version(
            &self,
            state_key: &StateKey,
            version: Version,
        ) -> Result<Option<StateValue>>;

        /// Get the latest state value and its corresponding version when it's of the given key up
        /// to the given version.
        /// See [AptosDB::get_state_value_with_version_by_version].
        ///
        /// [AptosDB::get_state_value_with_version_by_version]:
        /// ../aptosdb/struct.AptosDB.html#method.get_state_value_with_version_by_version
        fn get_state_value_with_version_by_version(
            &self,
            state_key: &StateKey,
            version: Version,
        ) -> Result<Option<(Version, StateValue)>>;

        /// Returns the proof of the given state key and version.
        fn get_state_proof_by_version_ext(
            &self,
            key_hash: &HashValue,
            version: Version,
            root_depth: usize,
        ) -> Result<SparseMerkleProofExt>;

        /// Gets a state value by state key along with the proof, out of the ledger state indicated by the state
        /// Merkle tree root with a sparse merkle proof proving state tree root.
        /// See [AptosDB::get_account_state_with_proof_by_version].
        ///
        /// [AptosDB::get_account_state_with_proof_by_version]:
        /// ../aptosdb/struct.AptosDB.html#method.get_account_state_with_proof_by_version
        ///
        /// This is used by aptos core (executor) internally.
        fn get_state_value_with_proof_by_version_ext(
            &self,
            key_hash: &HashValue,
            version: Version,
            root_depth: usize,
        ) -> Result<(Option<StateValue>, SparseMerkleProofExt)>;

        /// Gets the latest LedgerView no matter if db has been bootstrapped.
        /// Used by the Db-bootstrapper.
        fn get_pre_committed_ledger_summary(&self) -> Result<LedgerSummary>;

        fn get_persisted_state(&self) -> Result<(Arc<dyn HotStateView>, State)>;

        fn get_persisted_state_summary(&self) -> Result<StateSummary>;

        /// Get the ledger info of the epoch that `known_version` belongs to.
        fn get_epoch_ending_ledger_info(
            &self,
            known_version: u64,
        ) -> Result<LedgerInfoWithSignatures>;

        /// Gets the transaction accumulator root hash at specified version.
        /// Caller must guarantee the version is not greater than the latest version.
        fn get_accumulator_root_hash(&self, _version: Version) -> Result<HashValue>;

        /// Gets an [`AccumulatorConsistencyProof`] starting from `client_known_version`
        /// (or pre-genesis if `None`) until `ledger_version`.
        ///
        /// In other words, if the client has an accumulator summary for
        /// `client_known_version`, they can use the result from this API to efficiently
        /// extend their accumulator to `ledger_version` and prove that the new accumulator
        /// is consistent with their old accumulator. By consistent, we mean that by
        /// appending the actual `ledger_version - client_known_version` transactions
        /// to the old accumulator summary you get the new accumulator summary.
        ///
        /// If the client is starting up for the first time and has no accumulator
        /// summary yet, they can call this with `client_known_version=None`, i.e.,
        /// pre-genesis, to get the complete accumulator summary up to `ledger_version`.
        fn get_accumulator_consistency_proof(
            &self,
            _client_known_version: Option<Version>,
            _ledger_version: Version,
        ) -> Result<AccumulatorConsistencyProof>;

        /// A convenience function for building a [`TransactionAccumulatorSummary`]
        /// at the given `ledger_version`.
        ///
        /// Note: this is roughly equivalent to calling
        /// `DbReader::get_accumulator_consistency_proof(None, ledger_version)`.
        fn get_accumulator_summary(
            &self,
            ledger_version: Version,
        ) -> Result<TransactionAccumulatorSummary>;

        /// Returns total number of state items in state store at given version.
        fn get_state_item_count(&self, version: Version) -> Result<usize>;

        /// Get a chunk of state store value, addressed by the index.
        fn get_state_value_chunk_with_proof(
            &self,
            version: Version,
            start_idx: usize,
            chunk_size: usize,
        ) -> Result<StateValueChunkWithProof>;

        /// Returns if the state store pruner is enabled.
        fn is_state_merkle_pruner_enabled(&self) -> Result<bool>;

        /// Get the state prune window config value.
        fn get_epoch_snapshot_prune_window(&self) -> Result<usize>;

        /// Returns if the ledger pruner is enabled.
        fn is_ledger_pruner_enabled(&self) -> Result<bool>;

        /// Get the ledger prune window config value.
        fn get_ledger_prune_window(&self) -> Result<usize>;

        /// Get table info from the internal indexer.
        fn get_table_info(&self, handle: TableHandle) -> Result<TableInfo>;

        /// Returns whether the internal indexer DB has been enabled or not
        fn indexer_enabled(&self) -> bool;

        /// Returns state storage usage at the end of an epoch.
        fn get_state_storage_usage(&self, version: Option<Version>) -> Result<StateStorageUsage>;

        fn get_event_by_version_and_index(
            &self,
            version: Version,
            index: u64,
        ) -> Result<ContractEvent>;
    ); // end delegated

    /// Returns the latest ledger info.
    fn get_latest_ledger_info(&self) -> Result<LedgerInfoWithSignatures> {
        self.get_latest_ledger_info_option().and_then(|opt| {
            opt.ok_or_else(|| AptosDbError::Other("Latest LedgerInfo not found.".to_string()))
        })
    }

    /// Returns the latest committed version, error on on non-bootstrapped/empty DB.
    /// N.b. different from `get_synced_version()`.
    fn get_latest_ledger_info_version(&self) -> Result<Version> {
        self.get_latest_ledger_info()
            .map(|li| li.ledger_info().version())
    }

    /// Returns the latest version and committed block timestamp
    fn get_latest_commit_metadata(&self) -> Result<(Version, u64)> {
        let ledger_info_with_sig = self.get_latest_ledger_info()?;
        let ledger_info = ledger_info_with_sig.ledger_info();
        Ok((ledger_info.version(), ledger_info.timestamp_usecs()))
    }

    fn get_state_value_with_proof_by_version(
        &self,
        state_key: &StateKey,
        version: Version,
    ) -> Result<(Option<StateValue>, SparseMerkleProof)> {
        self.get_state_value_with_proof_by_version_ext(state_key.crypto_hash_ref(), version, 0)
            .map(|(value, proof_ext)| (value, proof_ext.into()))
    }

    fn ensure_synced_version(&self) -> Result<Version> {
        self.get_synced_version()?
            .ok_or_else(|| AptosDbError::NotFound("Synced version not found.".to_string()))
    }

    fn expect_synced_version(&self) -> Version {
        self.ensure_synced_version()
            .expect("Failed to get synced version.")
    }

    fn ensure_pre_committed_version(&self) -> Result<Version> {
        self.get_pre_committed_version()?
            .ok_or_else(|| AptosDbError::NotFound("Pre-committed version not found.".to_string()))
    }
}

/// Trait that is implemented by a DB that supports certain public (to client) write APIs
/// expected of an Aptos DB. This adds write APIs to DbReader.
#[allow(unused_variables)]
pub trait DbWriter: Send + Sync {
    /// Get a (stateful) state snapshot receiver.
    ///
    /// Chunk of accounts need to be added via `add_chunk()` before finishing up with `finish_box()`
    fn get_state_snapshot_receiver(
        &self,
        version: Version,
        expected_root_hash: HashValue,
    ) -> Result<Box<dyn StateSnapshotReceiver<StateKey, StateValue>>> {
        unimplemented!()
    }

    /// Finalizes a state snapshot that has already been restored to the database through
    /// a state snapshot receiver. This is required to bootstrap the transaction accumulator,
    /// populate transaction information, save the epoch ending ledger infos and delete genesis.
    ///
    /// Note: this assumes that the output with proof has already been verified and that the
    /// state snapshot was restored at the same version.
    fn finalize_state_snapshot(
        &self,
        version: Version,
        output_with_proof: TransactionOutputListWithProofV2,
        ledger_infos: &[LedgerInfoWithSignatures],
    ) -> Result<()> {
        unimplemented!()
    }

    /// Persist transactions. Called by state sync to save verified transactions to the DB.
    fn save_transactions(
        &self,
        chunk: ChunkToCommit,
        ledger_info_with_sigs: Option<&LedgerInfoWithSignatures>,
        sync_commit: bool,
    ) -> Result<()> {
        // For reconfig suffix.
        if ledger_info_with_sigs.is_none() && chunk.is_empty() {
            return Ok(());
        }

        if !chunk.is_empty() {
            self.pre_commit_ledger(chunk.clone(), sync_commit)?;
        }
        let version_to_commit = if let Some(ledger_info_with_sigs) = ledger_info_with_sigs {
            ledger_info_with_sigs.ledger_info().version()
        } else {
            chunk.expect_last_version()
        };
        self.commit_ledger(version_to_commit, ledger_info_with_sigs, Some(chunk))
    }

    /// Optimistically persist transactions to the ledger.
    ///
    /// Called by consensus to pre-commit blocks before execution result is agreed on by the
    /// validators.
    ///
    ///   If these blocks are later confirmed to be included in the ledger, commit_ledger should be
    ///       called with a `LedgerInfoWithSignatures`.
    ///   If not, the consensus needs to panic, resulting in a reboot of the node where the DB will
    ///       truncate the unconfirmed data.
    fn pre_commit_ledger(&self, chunk: ChunkToCommit, sync_commit: bool) -> Result<()> {
        unimplemented!()
    }

    /// Commit pre-committed transactions to the ledger.
    ///
    /// If a LedgerInfoWithSigs is provided, both the "synced version" and "committed version" will
    /// advance, otherwise only the synced version will advance.
    fn commit_ledger(
        &self,
        version: Version,
        ledger_info_with_sigs: Option<&LedgerInfoWithSignatures>,
        chunk_opt: Option<ChunkToCommit>,
    ) -> Result<()> {
        unimplemented!()
    }
}

#[derive(Clone)]
pub struct DbReaderWriter {
    pub reader: Arc<dyn DbReader>,
    pub writer: Arc<dyn DbWriter>,
}

impl DbReaderWriter {
    pub fn new<D: 'static + DbReader + DbWriter>(db: D) -> Self {
        let reader = Arc::new(db);
        let writer = Arc::clone(&reader);

        Self { reader, writer }
    }

    pub fn from_arc<D: 'static + DbReader + DbWriter>(arc_db: Arc<D>) -> Self {
        let reader = Arc::clone(&arc_db);
        let writer = Arc::clone(&arc_db);

        Self { reader, writer }
    }

    pub fn wrap<D: 'static + DbReader + DbWriter>(db: D) -> (Arc<D>, Self) {
        let arc_db = Arc::new(db);
        (Arc::clone(&arc_db), Self::from_arc(arc_db))
    }
}

/// Network types for storage service
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum StorageRequest {
    GetStateValueByVersionRequest(Box<GetStateValueByVersionRequest>),
    GetStartupInfoRequest,
    SaveTransactionsRequest(Box<SaveTransactionsRequest>),
}

#[derive(Debug, PartialEq, Eq, Clone, Deserialize, Serialize)]
pub struct GetStateValueByVersionRequest {
    /// The access key for the resource
    pub state_key: StateKey,

    /// The version the query is based on.
    pub version: Version,
}

impl GetStateValueByVersionRequest {
    /// Constructor.
    pub fn new(state_key: StateKey, version: Version) -> Self {
        Self { state_key, version }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct SaveTransactionsRequest {
    pub txns_to_commit: Vec<TransactionToCommit>,
    pub first_version: Version,
    pub ledger_info_with_signatures: Option<LedgerInfoWithSignatures>,
}

impl SaveTransactionsRequest {
    /// Constructor.
    pub fn new(
        txns_to_commit: Vec<TransactionToCommit>,
        first_version: Version,
        ledger_info_with_signatures: Option<LedgerInfoWithSignatures>,
    ) -> Self {
        SaveTransactionsRequest {
            txns_to_commit,
            first_version,
            ledger_info_with_signatures,
        }
    }
}

pub fn jmt_update_refs<K>(
    jmt_updates: &[(HashValue, Option<(HashValue, K)>)],
) -> Vec<(HashValue, Option<&(HashValue, K)>)> {
    jmt_updates.iter().map(|(x, y)| (*x, y.as_ref())).collect()
}

#[macro_export]
macro_rules! db_anyhow {
    ($($arg:tt)*) => {
        AptosDbError::Other(format!($($arg)*))
    };
}

#[macro_export]
macro_rules! db_not_found_bail {
    ($($arg:tt)*) => {
        return Err(AptosDbError::NotFound(format!($($arg)*)))
    };
}

#[macro_export]
macro_rules! db_other_bail {
    ($($arg:tt)*) => {
        return Err(AptosDbError::Other(format!($($arg)*)))
    };
}

#[macro_export]
macro_rules! db_ensure {
    ($cond:expr, $($arg:tt)*) => {
        if !$cond {
            return Err(AptosDbError::Other(format!($($arg)*)));
        }
    };
}
