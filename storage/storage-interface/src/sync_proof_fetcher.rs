// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{proof_fetcher::ProofFetcher, DbReader};
use anyhow::format_err;
use aptos_crypto::{hash::CryptoHash, HashValue};
use aptos_types::{
    proof::SparseMerkleProofExt,
    state_store::{state_key::StateKey, state_value::StateValue},
    transaction::Version,
};
use parking_lot::RwLock;
use std::{collections::HashMap, sync::Arc};

/// An implementation of proof fetcher, which synchronously fetches proofs from the underlying persistent
/// storage.
pub struct SyncProofFetcher {
    reader: Arc<dyn DbReader>,
    state_proof_cache: RwLock<HashMap<HashValue, SparseMerkleProofExt>>,
}

impl SyncProofFetcher {
    pub fn new(reader: Arc<dyn DbReader>) -> Self {
        Self {
            reader,
            state_proof_cache: RwLock::new(HashMap::new()),
        }
    }
}

impl ProofFetcher for SyncProofFetcher {
    fn fetch_state_value_and_proof(
        &self,
        state_key: &StateKey,
        version: Version,
        root_hash: Option<HashValue>,
    ) -> anyhow::Result<(Option<StateValue>, Option<SparseMerkleProofExt>)> {
        let (state_value, proof) = self
            .reader
            .get_state_value_with_proof_by_version_ext(state_key, version)?;
        if let Some(root_hash) = root_hash {
            proof
                .verify(root_hash, state_key.hash(), state_value.as_ref())
                .map_err(|err| {
                    format_err!(
                        "Proof is invalid for key {:?} with state root hash {:?}: {}.",
                        state_key,
                        root_hash,
                        err
                    )
                })?;
        }
        // multiple threads may enter this code, and another thread might add
        // an address before this one. Thus the insertion might return a None here.
        self.state_proof_cache
            .write()
            .insert(state_key.hash(), proof.clone());

        Ok((state_value, Some(proof)))
    }

    fn get_proof_cache(&self) -> HashMap<HashValue, SparseMerkleProofExt> {
        self.state_proof_cache
            .read()
            .iter()
            .map(|(x, y)| (*x, y.clone()))
            .collect()
    }
}
