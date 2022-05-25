// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{proof_fetcher::ProofFetcher, DbReader};
use aptos_crypto::HashValue;
use aptos_types::{
    proof::SparseMerkleProof,
    state_store::{state_key::StateKey, state_value::StateValue},
    transaction::Version,
};
use std::{collections::HashMap, sync::Arc};

/// An implementation of proof fetcher, which just reads the state value without fetching the proof.
/// This can be useful for mempool validation, when we don't need to fetch the proof.
pub struct NoProofFetcher {
    reader: Arc<dyn DbReader>,
}

impl NoProofFetcher {
    pub fn new(reader: Arc<dyn DbReader>) -> Self {
        Self { reader }
    }
}

impl ProofFetcher for NoProofFetcher {
    fn fetch_state_value_and_proof(
        &self,
        state_key: &StateKey,
        version: Version,
    ) -> anyhow::Result<(Option<StateValue>, Option<SparseMerkleProof>)> {
        Ok((
            self.reader.get_state_value_by_version(state_key, version)?,
            None,
        ))
    }

    fn get_proof_cache(&self) -> HashMap<HashValue, SparseMerkleProof> {
        unimplemented!()
    }
}
