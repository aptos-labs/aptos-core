// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use anyhow::{anyhow, format_err, Result};
use aptos_crypto::{hash::CryptoHash, HashValue};
use aptos_types::account_config::NewBlockEvent;
use aptos_types::state_store::state_storage_usage::StateStorageUsage;
use aptos_types::state_store::table::{TableHandle, TableInfo};
use aptos_types::{
    access_path::AccessPath,
    account_address::AccountAddress,
    account_config::CORE_CODE_ADDRESS,
    contract_event::EventWithVersion,
    epoch_change::EpochChangeProof,
    epoch_state::EpochState,
    event::EventKey,
    ledger_info::LedgerInfoWithSignatures,
    move_resource::MoveStorage,
    on_chain_config::{access_path_for_config, ConfigID},
    proof::{
        AccumulatorConsistencyProof, SparseMerkleProof, SparseMerkleProofExt,
        SparseMerkleRangeProof, TransactionAccumulatorSummary,
    },
    state_proof::StateProof,
    state_store::{
        state_key::StateKey,
        state_key_prefix::StateKeyPrefix,
        state_value::{StateValue, StateValueChunkWithProof},
    },
    transaction::{
        AccountTransactionsWithProof, TransactionInfo, TransactionListWithProof,
        TransactionOutputListWithProof, TransactionToCommit, TransactionWithProof, Version,
    },
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};
use thiserror::Error;

pub mod async_proof_fetcher;
pub mod cached_state_view;
mod executed_trees;
mod metrics;
#[cfg(any(test, feature = "fuzzing"))]
pub mod mock;
pub mod proof_fetcher;
pub mod state_delta;
pub mod state_view;
pub mod sync_proof_fetcher;

use crate::state_delta::StateDelta;
pub use executed_trees::ExecutedTrees;

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

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum Order {
    Ascending,
    Descending,
}

/// Trait that is implemented by a DB that supports certain public (to client) read APIs
/// expected of an Aptos DB
#[allow(unused_variables)]
pub trait DbReader: Send + Sync {
    /// See [AptosDB::get_epoch_ending_ledger_infos].
    ///
    /// [AptosDB::get_epoch_ending_ledger_infos]:
    /// ../aptosdb/struct.AptosDB.html#method.get_epoch_ending_ledger_infos
    fn get_epoch_ending_ledger_infos(
        &self,
        start_epoch: u64,
        end_epoch: u64,
    ) -> Result<EpochChangeProof> {
        unimplemented!()
    }

    /// See [AptosDB::get_transactions].
    ///
    /// [AptosDB::get_transactions]: ../aptosdb/struct.AptosDB.html#method.get_transactions
    fn get_transactions(
        &self,
        start_version: Version,
        batch_size: u64,
        ledger_version: Version,
        fetch_events: bool,
    ) -> Result<TransactionListWithProof> {
        unimplemented!()
    }

    fn get_gas_prices(
        &self,
        start_version: Version,
        limit: u64,
        ledger_version: Version,
    ) -> Result<Vec<u64>> {
        unimplemented!()
    }

    /// See [AptosDB::get_transaction_by_hash].
    ///
    /// [AptosDB::get_transaction_by_hash]: ../aptosdb/struct.AptosDB.html#method.get_transaction_by_hash
    fn get_transaction_by_hash(
        &self,
        hash: HashValue,
        ledger_version: Version,
        fetch_events: bool,
    ) -> Result<Option<TransactionWithProof>> {
        unimplemented!()
    }

    /// See [AptosDB::get_transaction_by_version].
    ///
    /// [AptosDB::get_transaction_by_version]: ../aptosdb/struct.AptosDB.html#method.get_transaction_by_version
    fn get_transaction_by_version(
        &self,
        version: Version,
        ledger_version: Version,
        fetch_events: bool,
    ) -> Result<TransactionWithProof> {
        unimplemented!()
    }

    /// See [AptosDB::get_first_txn_version].
    ///
    /// [AptosDB::get_first_txn_version]: ../aptosdb/struct.AptosDB.html#method.get_first_txn_version
    fn get_first_txn_version(&self) -> Result<Option<Version>> {
        unimplemented!()
    }

    /// See [AptosDB::get_first_viable_txn_version].
    ///
    /// [AptosDB::get_first_viable_txn_version]: ../aptosdb/struct.AptosDB.html#method.get_first_viable_txn_version
    fn get_first_viable_txn_version(&self) -> Result<Version> {
        unimplemented!()
    }

    /// See [AptosDB::get_first_write_set_version].
    ///
    /// [AptosDB::get_first_write_set_version]: ../aptosdb/struct.AptosDB.html#method.get_first_write_set_version
    fn get_first_write_set_version(&self) -> Result<Option<Version>> {
        unimplemented!()
    }

    /// See [AptosDB::get_transaction_outputs].
    ///
    /// [AptosDB::get_transaction_outputs]: ../aptosdb/struct.AptosDB.html#method.get_transaction_outputs
    fn get_transaction_outputs(
        &self,
        start_version: Version,
        limit: u64,
        ledger_version: Version,
    ) -> Result<TransactionOutputListWithProof> {
        unimplemented!()
    }

    /// Returns events by given event key
    fn get_events(
        &self,
        event_key: &EventKey,
        start: u64,
        order: Order,
        limit: u64,
        ledger_version: Version,
    ) -> Result<Vec<EventWithVersion>> {
        unimplemented!()
    }

    /// See [AptosDB::get_block_timestamp].
    ///
    /// [AptosDB::get_block_timestamp]:
    /// ../aptosdb/struct.AptosDB.html#method.get_block_timestamp
    fn get_block_timestamp(&self, version: Version) -> Result<u64> {
        unimplemented!()
    }

    fn get_next_block_event(&self, version: Version) -> Result<(Version, NewBlockEvent)> {
        unimplemented!()
    }

    /// Returns the start_version, end_version and NewBlockEvent of the block containing the input
    /// transaction version.
    fn get_block_info_by_version(
        &self,
        version: Version,
    ) -> Result<(Version, Version, NewBlockEvent)> {
        unimplemented!()
    }

    /// Returns the start_version, end_version and NewBlockEvent of the block containing the input
    /// transaction version.
    fn get_block_info_by_height(&self, height: u64) -> Result<(Version, Version, NewBlockEvent)> {
        unimplemented!()
    }

    /// Gets the version of the last transaction committed before timestamp,
    /// a committed block at or after the required timestamp must exist (otherwise it's possible
    /// the next block committed as a timestamp smaller than the one in the request).
    fn get_last_version_before_timestamp(
        &self,
        _timestamp: u64,
        _ledger_version: Version,
    ) -> Result<Version> {
        unimplemented!()
    }

    /// Gets the latest epoch state currently held in storage.
    fn get_latest_epoch_state(&self) -> Result<EpochState> {
        unimplemented!()
    }

    /// Returns the key, value pairs for a particular state key prefix at at desired version. This
    /// API can be used to get all resources of an account by passing the account address as the
    /// key prefix.
    fn get_state_values_by_key_prefix(
        &self,
        key_prefix: &StateKeyPrefix,
        version: Version,
    ) -> Result<HashMap<StateKey, StateValue>> {
        unimplemented!()
    }

    /// Returns the latest ledger info, if any.
    fn get_latest_ledger_info_option(&self) -> Result<Option<LedgerInfoWithSignatures>> {
        unimplemented!()
    }

    /// Returns the latest ledger info.
    fn get_latest_ledger_info(&self) -> Result<LedgerInfoWithSignatures> {
        self.get_latest_ledger_info_option()
            .and_then(|opt| opt.ok_or_else(|| format_err!("Latest LedgerInfo not found.")))
    }

    /// Returns the latest version, error on on non-bootstrapped DB.
    fn get_latest_version(&self) -> Result<Version> {
        Ok(self.get_latest_ledger_info()?.ledger_info().version())
    }

    /// Returns the latest state checkpoint version if any.
    fn get_latest_state_checkpoint_version(&self) -> Result<Option<Version>> {
        unimplemented!()
    }

    /// Returns the latest state snapshot strictly before `next_version` if any.
    fn get_state_snapshot_before(
        &self,
        next_version: Version,
    ) -> Result<Option<(Version, HashValue)>> {
        unimplemented!()
    }

    /// Returns the latest version and committed block timestamp
    fn get_latest_commit_metadata(&self) -> Result<(Version, u64)> {
        let ledger_info_with_sig = self.get_latest_ledger_info()?;
        let ledger_info = ledger_info_with_sig.ledger_info();
        Ok((ledger_info.version(), ledger_info.timestamp_usecs()))
    }

    /// Returns a transaction that is the `seq_num`-th one associated with the given account. If
    /// the transaction with given `seq_num` doesn't exist, returns `None`.
    fn get_account_transaction(
        &self,
        address: AccountAddress,
        seq_num: u64,
        include_events: bool,
        ledger_version: Version,
    ) -> Result<Option<TransactionWithProof>> {
        unimplemented!()
    }

    /// Returns the list of transactions sent by an account with `address` starting
    /// at sequence number `seq_num`. Will return no more than `limit` transactions.
    /// Will ignore transactions with `txn.version > ledger_version`. Optionally
    /// fetch events for each transaction when `fetch_events` is `true`.
    fn get_account_transactions(
        &self,
        address: AccountAddress,
        seq_num: u64,
        limit: u64,
        include_events: bool,
        ledger_version: Version,
    ) -> Result<AccountTransactionsWithProof> {
        unimplemented!()
    }

    /// Returns proof of new state for a given ledger info with signatures relative to version known
    /// to client
    fn get_state_proof_with_ledger_info(
        &self,
        known_version: u64,
        ledger_info: LedgerInfoWithSignatures,
    ) -> Result<StateProof> {
        unimplemented!()
    }

    /// Returns proof of new state relative to version known to client
    fn get_state_proof(&self, known_version: u64) -> Result<StateProof> {
        unimplemented!()
    }

    /// Gets an account state by account address.
    /// See [AptosDB::get_state_value_by_version].
    ///
    /// [AptosDB::get_state_value_by_version]:
    /// ../aptosdb/struct.AptosDB.html#method.get_state_value_by_version
    fn get_state_value_by_version(
        &self,
        state_key: &StateKey,
        version: Version,
    ) -> Result<Option<StateValue>> {
        unimplemented!()
    }

    /// Returns the proof of the given state key and version.
    fn get_state_proof_by_version_ext(
        &self,
        state_key: &StateKey,
        version: Version,
    ) -> Result<SparseMerkleProofExt> {
        unimplemented!()
    }

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
        state_key: &StateKey,
        version: Version,
    ) -> Result<(Option<StateValue>, SparseMerkleProofExt)> {
        unimplemented!()
    }

    fn get_state_value_with_proof_by_version(
        &self,
        state_key: &StateKey,
        version: Version,
    ) -> Result<(Option<StateValue>, SparseMerkleProof)> {
        self.get_state_value_with_proof_by_version_ext(state_key, version)
            .map(|(value, proof_ext)| (value, proof_ext.into()))
    }

    /// Gets the latest ExecutedTrees no matter if db has been bootstrapped.
    /// Used by the Db-bootstrapper.
    fn get_latest_executed_trees(&self) -> Result<ExecutedTrees> {
        unimplemented!()
    }

    /// Get the ledger info of the epoch that `known_version` belongs to.
    fn get_epoch_ending_ledger_info(&self, known_version: u64) -> Result<LedgerInfoWithSignatures> {
        unimplemented!()
    }

    /// Gets the latest transaction info.
    /// N.B. Unlike get_startup_info(), even if the db is not bootstrapped, this can return `Some`
    /// -- those from a db-restore run.
    fn get_latest_transaction_info_option(&self) -> Result<Option<(Version, TransactionInfo)>> {
        unimplemented!()
    }

    /// Gets the transaction accumulator root hash at specified version.
    /// Caller must guarantee the version is not greater than the latest version.
    fn get_accumulator_root_hash(&self, _version: Version) -> Result<HashValue> {
        unimplemented!()
    }

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
    ) -> Result<AccumulatorConsistencyProof> {
        unimplemented!()
    }

    /// A convenience function for building a [`TransactionAccumulatorSummary`]
    /// at the given `ledger_version`.
    ///
    /// Note: this is roughly equivalent to calling
    /// `DbReader::get_accumulator_consistency_proof(None, ledger_version)`.
    fn get_accumulator_summary(
        &self,
        ledger_version: Version,
    ) -> Result<TransactionAccumulatorSummary> {
        unimplemented!()
    }

    /// Returns total number of leaves in state store at given version.
    fn get_state_leaf_count(&self, version: Version) -> Result<usize> {
        unimplemented!()
    }

    /// Get a chunk of state store value, addressed by the index.
    fn get_state_value_chunk_with_proof(
        &self,
        version: Version,
        start_idx: usize,
        chunk_size: usize,
    ) -> Result<StateValueChunkWithProof> {
        unimplemented!()
    }

    /// Returns if the state store pruner is enabled.
    fn is_state_pruner_enabled(&self) -> Result<bool> {
        unimplemented!()
    }

    /// Get the state prune window config value.
    fn get_epoch_snapshot_prune_window(&self) -> Result<usize> {
        unimplemented!()
    }

    /// Returns if the ledger pruner is enabled.
    fn is_ledger_pruner_enabled(&self) -> Result<bool> {
        unimplemented!()
    }

    /// Get the ledger prune window config value.
    fn get_ledger_prune_window(&self) -> Result<usize> {
        unimplemented!()
    }

    /// Get table info from the internal indexer.
    fn get_table_info(&self, handle: TableHandle) -> Result<TableInfo> {
        unimplemented!()
    }

    /// Returns whether the internal indexer DB has been enabled or not
    fn indexer_enabled(&self) -> bool {
        unimplemented!()
    }

    /// Returns state storage usage at the end of an epoch.
    fn get_state_storage_usage(&self, version: Option<Version>) -> Result<StateStorageUsage> {
        unimplemented!()
    }
}

impl MoveStorage for &dyn DbReader {
    fn fetch_resource(&self, access_path: AccessPath) -> Result<Vec<u8>> {
        self.fetch_resource_by_version(access_path, self.fetch_latest_state_checkpoint_version()?)
    }

    fn fetch_resource_by_version(
        &self,
        access_path: AccessPath,
        version: Version,
    ) -> Result<Vec<u8>> {
        let state_value =
            self.get_state_value_by_version(&StateKey::AccessPath(access_path), version)?;

        state_value
            .ok_or_else(|| format_err!("no value found in DB"))
            .map(|value| value.into_bytes())
    }

    fn fetch_config_by_version(&self, config_id: ConfigID, version: Version) -> Result<Vec<u8>> {
        let config_value_option = self.get_state_value_by_version(
            &StateKey::AccessPath(AccessPath::new(
                CORE_CODE_ADDRESS,
                access_path_for_config(config_id).path,
            )),
            version,
        )?;
        config_value_option
            .map(|x| x.into_bytes())
            .ok_or_else(|| anyhow!("no config {} found in aptos root account state", config_id))
    }

    fn fetch_synced_version(&self) -> Result<u64> {
        let (synced_version, _) = self
            .get_latest_transaction_info_option()
            .map_err(|e| {
                format_err!(
                    "[MoveStorage] Failed fetching latest transaction info: {}",
                    e
                )
            })?
            .ok_or_else(|| format_err!("[MoveStorage] Latest transaction info not found."))?;
        Ok(synced_version)
    }

    fn fetch_latest_state_checkpoint_version(&self) -> Result<Version> {
        self.get_latest_state_checkpoint_version()?
            .ok_or_else(|| format_err!("[MoveStorage] Latest state checkpoint not found."))
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
        output_with_proof: TransactionOutputListWithProof,
        ledger_infos: &[LedgerInfoWithSignatures],
    ) -> Result<()> {
        unimplemented!()
    }

    /// Persist transactions. Called by the executor module when either syncing nodes or committing
    /// blocks during normal operation.
    /// See [`AptosDB::save_transactions`].
    ///
    /// [`AptosDB::save_transactions`]: ../aptosdb/struct.AptosDB.html#method.save_transactions
    fn save_transactions(
        &self,
        txns_to_commit: &[TransactionToCommit],
        first_version: Version,
        base_state_version: Option<Version>,
        ledger_info_with_sigs: Option<&LedgerInfoWithSignatures>,
        sync_commit: bool,
        latest_in_memory_state: StateDelta,
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

pub fn jmt_updates(
    state_updates: &HashMap<StateKey, Option<StateValue>>,
) -> Vec<(HashValue, Option<(HashValue, StateKey)>)> {
    state_updates
        .iter()
        .map(|(k, v_opt)| (k.hash(), v_opt.as_ref().map(|v| (v.hash(), k.clone()))))
        .collect()
}

pub fn jmt_update_refs<K>(
    jmt_updates: &[(HashValue, Option<(HashValue, K)>)],
) -> Vec<(HashValue, Option<&(HashValue, K)>)> {
    jmt_updates.iter().map(|(x, y)| (*x, y.as_ref())).collect()
}
