// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{proof_fetcher::ProofFetcher, DbReader};
use anyhow::{format_err, Result};
use aptos_crypto::{_once_cell::sync::Lazy, hash::CryptoHash, HashValue};
use aptos_metrics::{register_histogram, Histogram};
use aptos_state_view::{StateView, StateViewId};
use aptos_types::{
    proof::SparseMerkleProof,
    state_store::{state_key::StateKey, state_value::StateValue},
    transaction::Version,
    write_set::WriteSet,
};
use parking_lot::RwLock;
use scratchpad::{FrozenSparseMerkleTree, SparseMerkleTree, StateStoreStatus};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

pub static FETCH_STATE_VALUE: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        // metric name
        "fetch_state_value",
        // metric description
        "The total time spent in seconds of block execution in the block executor."
    )
    .unwrap()
});

pub static GET_PROOF_CACHE: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        // metric name
        "get_proof_cache",
        // metric description
        "The total time spent in seconds of block execution in the block executor."
    )
    .unwrap()
});

/// `CachedStateView` is like a snapshot of the global state comprised of state view at two
/// levels, persistent storage and memory.
pub struct CachedStateView {
    /// For logging and debugging purpose, identifies what this view is for.
    id: StateViewId,

    /// A readable state checkpoint in the persistent storage.
    persisted_checkpoint: Option<(Version, HashValue)>,

    /// The in-memory state on top of the persisted checkpoint.
    speculative_state: FrozenSparseMerkleTree<StateValue>,

    /// The cache of verified account states from `reader` and `speculative_state_view`,
    /// represented by a hashmap with an account address as key and a pair of an ordered
    /// account state map and an an optional account state proof as value. When the VM queries an
    /// `access_path`, this cache will first check whether `reader_cache` is hit. If hit, it
    /// will return the corresponding value of that `access_path`; otherwise, the account state
    /// will be loaded into the cache from scratchpad or persistent storage in order as a
    /// deserialized ordered map and then be returned. If the VM queries this account again,
    /// the cached data can be read directly without bothering storage layer. The proofs in
    /// cache are needed by ScratchPad after VM execution to construct an in-memory sparse Merkle
    /// tree.
    /// ```text
    ///                      +----------------------------+
    ///                      | In-memory SparseMerkleTree <------+
    ///                      +-------------^--------------+      |
    ///                                    |                     |
    ///                                write sets                |
    ///                                    |          cached account state map
    ///                            +-------+-------+           proof
    ///                            |      V M      |             |
    ///                            +-------^-------+             |
    ///                                    |                     |
    ///                      value of `account_address/path`     |
    ///                                    |                     |
    ///        +---------------------------+---------------------+-------+
    ///        | +-------------------------+---------------------+-----+ |
    ///        | |           state_cache,     state_key_to_proof_cache   | |
    ///        | +---------------^---------------------------^---------+ |
    ///        |                 |                           |           |
    ///        |     state store values only        state blob proof     |
    ///        |                 |                           |           |
    ///        |                 |                           |           |
    ///        | +---------------+--------------+ +----------+---------+ |
    ///        | |      speculative_state       | |       reader       | |
    ///        | +------------------------------+ +--------------------+ |
    ///        +---------------------------------------------------------+
    /// ```
    /// Cache of state key to state value, which is used in case of fine grained storage object.
    /// Eventually this should replace the `account_to_state_cache` as we deprecate account state blob
    /// completely and migrate to fine grained storage. A value of None in this cache reflects that
    /// the corresponding key has been deleted. This is a temporary hack until we support deletion
    /// in JMT node.
    state_cache: RwLock<HashMap<StateKey, StateValue>>,
    proof_fetcher: Arc<dyn ProofFetcher>,
}

impl CachedStateView {
    /// Constructs a [`CachedStateView`] with persistent state view in the DB and the in-memory
    /// speculative state represented by `speculative_state`. The persistent state view is the
    /// latest one preceding `next_version`
    pub fn new(
        id: StateViewId,
        reader: Arc<dyn DbReader>,
        next_version: Version,
        speculative_state: SparseMerkleTree<StateValue>,
        proof_fetcher: Arc<dyn ProofFetcher>,
    ) -> Result<Self> {
        // n.b. Freeze the state before getting the state checkpoint, otherwise it's possible that
        // after we got the checkpoint, in-mem trees newer than it gets dropped before being frozen,
        // due to a commit happening from another thread.
        let speculative_state = speculative_state.freeze();
        let persisted_checkpoint = reader.get_state_checkpoint_before(next_version)?;

        Ok(Self {
            id,
            persisted_checkpoint,
            speculative_state,
            state_cache: RwLock::new(HashMap::new()),
            proof_fetcher,
        })
    }

    pub fn prime_cache_by_write_set(&self, write_sets: &[WriteSet]) -> Result<()> {
        write_sets
            .iter()
            .flat_map(|write_set| write_set.iter())
            .map(|(key, _)| key)
            .collect::<HashSet<_>>()
            .into_iter()
            .try_for_each(|key| self.get_state_value(key).map(|_| ()))
    }

    pub fn into_state_cache(self) -> StateCache {
        // let timer = GET_PROOF_CACHE.start_timer();
        let proofs = self.proof_fetcher.get_proof_cache();
        // drop(timer);
        StateCache {
            frozen_base: self.speculative_state,
            state_cache: self.state_cache.into_inner(),
            proofs,
        }
    }

    fn get_state_value_internal(&self, state_key: &StateKey) -> Result<Option<StateValue>> {
        // Do most of the work outside the write lock.
        let key_hash = state_key.hash();
        let state_value_option = match self.speculative_state.get(key_hash) {
            StateStoreStatus::ExistsInScratchPad(value) => Some(value),
            StateStoreStatus::DoesNotExist => None,
            // No matter it is in db or unknown, we have to query from db since even the
            // former case, we don't have the blob data but only its hash.
            StateStoreStatus::ExistsInDB | StateStoreStatus::Unknown => {
                match self.persisted_checkpoint {
                    Some((version, root_hash)) => {
                        let (value, proof) = self
                            .proof_fetcher
                            .fetch_state_value_and_proof(state_key, version)?;
                        // TODO: proof verification can be opted out, for performance
                        if let Some(proof) = proof {
                            proof
                                .verify(root_hash, key_hash, value.as_ref())
                                .map_err(|err| {
                                    format_err!(
                                    "Proof is invalid for key {:?} with state root hash {:?}: {}",
                                    state_key,
                                    root_hash,
                                    err
                                )
                                })?;
                        }
                        value
                    }
                    None => None,
                }
            }
        };

        Ok(state_value_option)
    }
}

pub struct StateCache {
    pub frozen_base: FrozenSparseMerkleTree<StateValue>,
    pub state_cache: HashMap<StateKey, StateValue>,
    pub proofs: HashMap<HashValue, SparseMerkleProof>,
}

impl StateView for CachedStateView {
    fn id(&self) -> StateViewId {
        self.id
    }

    fn get_state_value(&self, state_key: &StateKey) -> Result<Option<Vec<u8>>> {
        // First check if the cache has the state value.
        if let Some(contents) = self.state_cache.read().get(state_key) {
            // This can return None, which means the value has been deleted from the DB.
            return Ok(contents.maybe_bytes.as_ref().cloned());
        }
        let state_value_option = self.get_state_value_internal(state_key)?;
        // Update the cache if still empty
        let mut cache = self.state_cache.write();
        let new_value = cache
            .entry(state_key.clone())
            .or_insert_with(|| state_value_option.unwrap_or_default());
        Ok(new_value.maybe_bytes.as_ref().cloned())
    }

    fn is_genesis(&self) -> bool {
        self.persisted_checkpoint.is_none()
    }
}
