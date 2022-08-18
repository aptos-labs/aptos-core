// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_crypto::HashValue;
use aptos_types::{
    proof::SparseMerkleProofExt,
    state_store::{state_key::StateKey, state_value::StateValue},
    transaction::Version,
};
use std::collections::HashMap;

/// Defines the trait for fetching proof from the DB
pub trait ProofFetcher: Sync + Send {
    /// API to fetch the state value along with proof
    fn fetch_state_value_and_proof(
        &self,
        state_key: &StateKey,
        version: Version,
    ) -> anyhow::Result<(Option<StateValue>, Option<SparseMerkleProofExt>)>;

    /// API to return all the proofs fetched by the proof fetcher so far.
    fn get_proof_cache(&self) -> HashMap<HashValue, SparseMerkleProofExt>;
}
