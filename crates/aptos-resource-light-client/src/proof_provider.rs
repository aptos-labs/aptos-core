// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use anyhow::Result;
use aptos_types::{
    account_address::AccountAddress,
    proof::SparseMerkleProofExt,
    state_proof::StateProof,
    state_store::state_value::StateValue,
    transaction::Version,
};
use move_core_types::language_storage::StructTag;
use thiserror::Error;

/// Error returned by the light client.
#[derive(Debug, Error)]
pub enum LightClientError {
    #[error("Proof provider error: {0}")]
    ProofProvider(#[from] anyhow::Error),

    #[error("State proof verification failed: {0}")]
    VerifyStateProof(#[from] anyhow::Error),

    #[error("Resource proof verification failed: {0}")]
    VerifyResourceProof(#[from] anyhow::Error),

    #[error("No trusted ledger info yet; call update_trusted_state first")]
    NoTrustedLedgerInfo,

    #[error("Requested version {requested} exceeds trusted version {trusted}")]
    VersionTooNew { requested: Version, trusted: Version },
}

/// Abstraction for fetching proofs and state from the chain.
///
/// The light client calls this to get (1) a [StateProof] to ratchet [TrustedState],
/// and (2) a resource value + Merkle proof at a given version. Implement this to
/// talk to one or more full nodes (e.g. over HTTP or the storage service).
pub trait ProofProvider: Send + Sync {
    /// Fetch a state proof for ratcheting. `known_version` is the client's current
    /// trusted version; the server returns a proof from that version to its latest.
    fn get_state_proof(&self, known_version: Version) -> Result<StateProof>;

    /// Fetch the resource value and a Sparse Merkle proof for the given key at `version`.
    /// The proof can be verified against the state root at `version`.
    fn get_resource_with_proof(
        &self,
        address: AccountAddress,
        resource_type: &StructTag,
        version: Version,
    ) -> Result<(Option<StateValue>, SparseMerkleProofExt)>;
}
