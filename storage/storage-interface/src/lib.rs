// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use anyhow::{anyhow, format_err, Result};
use aptos_crypto::{hash::SPARSE_MERKLE_PLACEHOLDER_HASH, HashValue};
use aptos_types::{
    access_path::AccessPath,
    account_address::AccountAddress,
    account_config::aptos_root_address,
    contract_event::{ContractEvent, EventByVersionWithProof, EventWithProof},
    epoch_change::EpochChangeProof,
    epoch_state::EpochState,
    event::EventKey,
    ledger_info::LedgerInfoWithSignatures,
    move_resource::MoveStorage,
    on_chain_config::{access_path_for_config, ConfigID},
    proof::{
        definition::LeafCount, AccumulatorConsistencyProof, SparseMerkleProof,
        SparseMerkleRangeProof, TransactionAccumulatorSummary,
    },
    state_proof::StateProof,
    state_store::{
        state_key::StateKey,
        state_key_prefix::StateKeyPrefix,
        state_value::{
            StateKeyAndValue, StateValue, StateValueChunkWithProof, StateValueWithProof,
        },
    },
    transaction::{
        AccountTransactionsWithProof, TransactionInfo, TransactionListWithProof,
        TransactionOutputListWithProof, TransactionToCommit, TransactionWithProof, Version,
    },
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};
use thiserror::Error;

pub mod in_memory_state;
#[cfg(any(feature = "testing", feature = "fuzzing"))]
pub mod mock;
pub mod state_view;
pub mod verified_state_view;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct StartupInfo {
    /// The latest ledger info.
    pub latest_ledger_info: LedgerInfoWithSignatures,
    /// If the above ledger info doesn't carry a validator set, the latest validator set. Otherwise
    /// `None`.
    pub latest_epoch_state: Option<EpochState>,
    pub committed_tree_state: TreeState,
    pub synced_tree_state: Option<TreeState>,
}

impl StartupInfo {
    pub fn new(
        latest_ledger_info: LedgerInfoWithSignatures,
        latest_epoch_state: Option<EpochState>,
        committed_tree_state: TreeState,
        synced_tree_state: Option<TreeState>,
    ) -> Self {
        Self {
            latest_ledger_info,
            latest_epoch_state,
            committed_tree_state,
            synced_tree_state,
        }
    }

    #[cfg(any(feature = "fuzzing"))]
    pub fn new_for_testing() -> Self {
        use aptos_types::on_chain_config::ValidatorSet;

        let latest_ledger_info =
            LedgerInfoWithSignatures::genesis(HashValue::zero(), ValidatorSet::empty());
        let latest_epoch_state = None;
        let committed_tree_state = TreeState {
            num_transactions: 0,
            ledger_frozen_subtree_hashes: Vec::new(),
            state_root_hash: *SPARSE_MERKLE_PLACEHOLDER_HASH,
        };
        let synced_tree_state = None;

        Self {
            latest_ledger_info,
            latest_epoch_state,
            committed_tree_state,
            synced_tree_state,
        }
    }

    pub fn get_epoch_state(&self) -> &EpochState {
        self.latest_ledger_info
            .ledger_info()
            .next_epoch_state()
            .unwrap_or_else(|| {
                self.latest_epoch_state
                    .as_ref()
                    .expect("EpochState must exist")
            })
    }

    pub fn into_latest_tree_state(self) -> TreeState {
        self.synced_tree_state.unwrap_or(self.committed_tree_state)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TreeState {
    pub num_transactions: LeafCount,
    pub ledger_frozen_subtree_hashes: Vec<HashValue>,
    pub state_root_hash: HashValue,
}

impl TreeState {
    pub fn new(
        num_transactions: LeafCount,
        ledger_frozen_subtree_hashes: Vec<HashValue>,
        account_state_root_hash: HashValue,
    ) -> Self {
        Self {
            num_transactions,
            ledger_frozen_subtree_hashes,
            state_root_hash: account_state_root_hash,
        }
    }

    pub fn describe(&self) -> &'static str {
        if self.num_transactions != 0 {
            "DB has been bootstrapped."
        } else if self.state_root_hash != *SPARSE_MERKLE_PLACEHOLDER_HASH {
            "DB has no transaction, but a non-empty pre-genesis state."
        } else {
            "DB is empty, has no transaction or state."
        }
    }
}

pub trait StateSnapshotReceiver<V>: Send {
    fn add_chunk(
        &mut self,
        chunk: Vec<(HashValue, V)>,
        proof: SparseMerkleRangeProof,
    ) -> Result<()>;

    fn finish(self) -> Result<()>;

    fn finish_box(self: Box<Self>) -> Result<()>;
}

#[derive(Debug, Deserialize, Error, PartialEq, Serialize)]
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
    /// See [`AptosDB::get_epoch_ending_ledger_infos`].
    ///
    /// [`AptosDB::get_epoch_ending_ledger_infos`]:
    /// ../aptosdb/struct.AptosDB.html#method.get_epoch_ending_ledger_infos
    fn get_epoch_ending_ledger_infos(
        &self,
        start_epoch: u64,
        end_epoch: u64,
    ) -> Result<EpochChangeProof> {
        unimplemented!()
    }

    /// See [`AptosDB::get_transactions`].
    ///
    /// [`AptosDB::get_transactions`]: ../aptosdb/struct.AptosDB.html#method.get_transactions
    fn get_transactions(
        &self,
        start_version: Version,
        batch_size: u64,
        ledger_version: Version,
        fetch_events: bool,
    ) -> Result<TransactionListWithProof> {
        unimplemented!()
    }

    /// See [`AptosDB::get_transaction_by_hash`].
    ///
    /// [`AptosDB::get_transaction_by_hash`]: ../aptosdb/struct.AptosDB.html#method.get_transaction_by_hash
    fn get_transaction_by_hash(
        &self,
        hash: HashValue,
        ledger_version: Version,
        fetch_events: bool,
    ) -> Result<Option<TransactionWithProof>> {
        unimplemented!()
    }

    /// See [`AptosDB::get_transaction_by_version`].
    ///
    /// [`AptosDB::get_transaction_by_version`]: ../aptosdb/struct.AptosDB.html#method.get_transaction_by_version
    fn get_transaction_by_version(
        &self,
        version: Version,
        ledger_version: Version,
        fetch_events: bool,
    ) -> Result<TransactionWithProof> {
        unimplemented!()
    }

    /// See [`AptosDB::get_txn_set_version`].
    ///
    /// [`AptosDB::get_first_txn_version`]: ../aptosdb/struct.AptosDB.html#method.get_first_txn_version
    fn get_first_txn_version(&self) -> Result<Option<Version>> {
        unimplemented!()
    }

    /// See [`AptosDB::get_first_write_set_version`].
    ///
    /// [`AptosDB::get_first_write_set_version`]: ../aptosdb/struct.AptosDB.html#method.get_first_write_set_version
    fn get_first_write_set_version(&self) -> Result<Option<Version>> {
        unimplemented!()
    }

    /// See [`AptosDB::get_transaction_outputs`].
    ///
    /// [`AptosDB::get_transaction_outputs`]: ../aptosdb/struct.AptosDB.html#method.get_transaction_outputs
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
    ) -> Result<Vec<(u64, ContractEvent)>> {
        unimplemented!()
    }

    /// Returns events by given event key
    fn get_events_with_proofs(
        &self,
        event_key: &EventKey,
        start: u64,
        order: Order,
        limit: u64,
        known_version: Option<u64>,
    ) -> Result<Vec<EventWithProof>> {
        unimplemented!()
    }

    /// See [`AptosDB::get_block_timestamp`].
    ///
    /// [`AptosDB::get_block_timestamp`]:
    /// ../aptosdb/struct.AptosDB.html#method.get_block_timestamp
    fn get_block_timestamp(&self, version: u64) -> Result<u64> {
        unimplemented!()
    }

    /// Returns the [`NewBlockEvent`] for the block containing the requested
    /// `version` and proof that the block actually contains the `version`.
    fn get_event_by_version_with_proof(
        &self,
        event_key: &EventKey,
        event_version: u64,
        proof_version: u64,
    ) -> Result<EventByVersionWithProof> {
        unimplemented!()
    }

    /// Gets the version of the last transaction committed before timestamp,
    /// a commited block at or after the required timestamp must exist (otherwise it's possible
    /// the next block committed as a timestamp smaller than the one in the request).
    fn get_last_version_before_timestamp(
        &self,
        _timestamp: u64,
        _ledger_version: Version,
    ) -> Result<Version> {
        unimplemented!()
    }

    /// See [`AptosDB::get_latest_account_state`].
    ///
    /// [`AptosDB::get_latest_account_state`]:
    /// ../aptosdb/struct.AptosDB.html#method.get_latest_account_state
    fn get_latest_state_value(&self, state_key: StateKey) -> Result<Option<StateValue>> {
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

    /// Returns the latest version, None for non-bootstrapped DB.
    fn get_latest_version_option(&self) -> Result<Option<Version>> {
        Ok(self
            .get_latest_ledger_info_option()?
            .map(|li| li.ledger_info().version()))
    }

    /// Returns the latest version, error on on non-bootstrapped DB.
    fn get_latest_version(&self) -> Result<Version> {
        Ok(self.get_latest_ledger_info()?.ledger_info().version())
    }

    /// Returns the latest version and committed block timestamp
    fn get_latest_commit_metadata(&self) -> Result<(Version, u64)> {
        let ledger_info_with_sig = self.get_latest_ledger_info()?;
        let ledger_info = ledger_info_with_sig.ledger_info();
        Ok((ledger_info.version(), ledger_info.timestamp_usecs()))
    }

    /// Gets information needed from storage during the main node startup.
    /// See [`AptosDB::get_startup_info`].
    ///
    /// [`AptosDB::get_startup_info`]:
    /// ../aptosdb/struct.AptosDB.html#method.get_startup_info
    fn get_startup_info(&self) -> Result<Option<StartupInfo>> {
        unimplemented!()
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

    /// Returns the account state corresponding to the given version and account address with proof
    /// based on `ledger_version`
    fn get_state_value_with_proof(
        &self,
        state_key: StateKey,
        version: Version,
        ledger_version: Version,
    ) -> Result<StateValueWithProof> {
        unimplemented!()
    }

    // Gets an account state by account address, out of the ledger state indicated by the state
    // Merkle tree root with a sparse merkle proof proving state tree root.
    // See [`AptosDB::get_account_state_with_proof_by_version`].
    //
    // [`AptosDB::get_account_state_with_proof_by_version`]:
    // ../aptosdb/struct.AptosDB.html#method.get_account_state_with_proof_by_version
    //
    // This is used by aptos core (executor) internally.
    fn get_state_value_with_proof_by_version(
        &self,
        state_key: &StateKey,
        version: Version,
    ) -> Result<(Option<StateValue>, SparseMerkleProof<StateValue>)> {
        unimplemented!()
    }

    /// Gets the latest TreeState no matter if db has been bootstrapped.
    /// Used by the Db-bootstrapper.
    fn get_latest_tree_state(&self) -> Result<TreeState> {
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
        let genesis_consistency_proof =
            self.get_accumulator_consistency_proof(None, ledger_version)?;
        TransactionAccumulatorSummary::try_from_genesis_proof(
            genesis_consistency_proof,
            ledger_version,
        )
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

    /// Get the state prune window config value.
    fn get_state_prune_window(&self) -> Result<Option<usize>> {
        unimplemented!()
    }

    /// Get the ledger prune window config value.
    fn get_ledger_prune_window(&self) -> Result<Option<usize>> {
        unimplemented!()
    }
}

impl MoveStorage for &dyn DbReader {
    fn fetch_resource(&self, access_path: AccessPath) -> Result<Vec<u8>> {
        self.fetch_resource_by_version(access_path, self.fetch_synced_version()?)
    }

    fn fetch_resource_by_version(
        &self,
        access_path: AccessPath,
        version: Version,
    ) -> Result<Vec<u8>> {
        let (state_value, _) = self
            .get_state_value_with_proof_by_version(&StateKey::AccessPath(access_path), version)?;

        state_value
            .ok_or_else(|| format_err!("no value found in DB"))?
            .maybe_bytes
            .ok_or_else(|| format_err!("no value found in DB"))
    }

    fn fetch_config_by_version(&self, config_id: ConfigID, version: Version) -> Result<Vec<u8>> {
        let config_value_option = self
            .get_state_value_with_proof_by_version(
                &StateKey::AccessPath(AccessPath::new(
                    aptos_root_address(),
                    access_path_for_config(config_id).path,
                )),
                version,
            )?
            .0;
        config_value_option
            .and_then(|x| x.maybe_bytes)
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
    ) -> Result<Box<dyn StateSnapshotReceiver<StateKeyAndValue>>> {
        unimplemented!()
    }

    /// Finalizes a state snapshot that has already been restored to the database through
    /// a state snapshot receiver. This is required to bootstrap the transaction accumulator
    /// and populate transaction and event information.
    ///
    /// Note: this assumes that the output with proof has already been verified and that the
    /// state snapshot was restored at the same version.
    fn finalize_state_snapshot(
        &self,
        version: Version,
        output_with_proof: TransactionOutputListWithProof,
    ) -> Result<()> {
        unimplemented!()
    }

    /// Persists the specified ledger infos.
    ///
    /// Note: this assumes that the ledger infos have already been verified.
    fn save_ledger_infos(&self, ledger_infos: &[LedgerInfoWithSignatures]) -> Result<()> {
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
        ledger_info_with_sigs: Option<&LedgerInfoWithSignatures>,
    ) -> Result<()> {
        unimplemented!()
    }

    /// Deletes transaction data associated with the genesis transaction. This is useful for
    /// cleaning up the database after a node has bootstrapped all accounts through state sync.
    ///
    /// TODO(joshlind): find a cleaner (long term) solution to avoid us having to expose this...
    fn delete_genesis(&self) -> Result<()> {
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
    GetStateValueWithProofByVersionRequest(Box<GetStateValueWithProofByVersionRequest>),
    GetStartupInfoRequest,
    SaveTransactionsRequest(Box<SaveTransactionsRequest>),
}

#[derive(Debug, PartialEq, Eq, Clone, Deserialize, Serialize)]
pub struct GetStateValueWithProofByVersionRequest {
    /// The access key for the resource
    pub state_key: StateKey,

    /// The version the query is based on.
    pub version: Version,
}

impl GetStateValueWithProofByVersionRequest {
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
