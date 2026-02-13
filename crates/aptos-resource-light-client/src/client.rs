// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::proof_provider::{LightClientError, ProofProvider};
use crate::{MonitoredResource, ResourceLightClientConfig};
use anyhow::Context;
use aptos_crypto::HashValue;
use aptos_types::{
    ledger_info::LedgerInfo,
    state_store::state_key::StateKey,
    state_store::state_value::StateValue,
    trusted_state::{TrustedState, TrustedStateChange},
    state_proof::StateProof,
    transaction::Version,
};
use std::sync::Arc;

/// Light client that maintains a trusted view of the chain and fetches one resource with proofs.
pub struct ResourceLightClient {
    chain_id: aptos_types::chain_id::ChainId,
    resource: MonitoredResource,
    proof_provider: Arc<dyn ProofProvider>,
    trusted_state: TrustedState,
    /// Ledger info from the last successful ratchet; used to get state root for resource proof verification.
    latest_ledger_info: Option<LedgerInfo>,
}

impl ResourceLightClient {
    /// Create a new light client from config. Starts from the given trusted state (e.g. waypoint).
    pub fn new(config: ResourceLightClientConfig) -> Self {
        Self {
            chain_id: config.chain_id,
            resource: config.resource,
            proof_provider: config.proof_provider,
            trusted_state: config.initial_trusted_state,
            latest_ledger_info: None,
        }
    }

    /// Create a light client from a previously saved trusted state (e.g. after restart).
    pub fn from_snapshot(config: ResourceLightClientConfig, snapshot: TrustedState) -> Self {
        Self {
            chain_id: config.chain_id,
            resource: config.resource,
            proof_provider: config.proof_provider,
            trusted_state: snapshot,
            latest_ledger_info: None,
        }
    }

    /// Current trusted ledger version.
    pub fn trusted_version(&self) -> Version {
        self.trusted_state.version()
    }

    /// Current trusted state; can be persisted and restored with [Self::from_snapshot].
    pub fn trusted_state_snapshot(&self) -> &TrustedState {
        &self.trusted_state
    }

    /// Ratchet trusted state to the latest available. Call this periodically or before queries.
    /// Returns the new trusted version on success.
    pub fn update_trusted_state(&mut self) -> Result<Version, LightClientError> {
        let known_version = self.trusted_state.version();
        let state_proof = self
            .proof_provider
            .get_state_proof(known_version)
            .map_err(LightClientError::ProofProvider)?;

        let change = self
            .trusted_state
            .verify_and_ratchet(&state_proof)
            .map_err(LightClientError::VerifyStateProof)?;

        if let Some(new_state) = change.new_state() {
            self.trusted_state = new_state;
            self.latest_ledger_info = Some(state_proof.latest_ledger_info().clone());
        }

        Ok(self.trusted_state.version())
    }

    /// Get the monitored resource at the current trusted version. The value is verified
    /// against the trusted state root. Returns (resource bytes, version).
    ///
    /// Call [Self::update_trusted_state] at least once before this so that we have a
    /// trusted state root to verify against.
    pub fn get_resource(&self) -> Result<(Vec<u8>, Version), LightClientError> {
        self.get_resource_at_version(self.trusted_state.version())
    }

    /// Get the monitored resource at a specific version. The version must equal the current
    /// trusted version (we only have a state root for that version). Returns (resource bytes, version).
    pub fn get_resource_at_version(&self, version: Version) -> Result<(Vec<u8>, Version), LightClientError> {
        let trusted = self.trusted_state.version();
        if version > trusted {
            return Err(LightClientError::VersionTooNew {
                requested: version,
                trusted,
            });
        }

        let ledger_info = self
            .latest_ledger_info
            .as_ref()
            .ok_or(LightClientError::NoTrustedLedgerInfo)?;

        if ledger_info.version() < version {
            return Err(LightClientError::NoTrustedLedgerInfo);
        }

        let state_root = ledger_info.commit_info().executed_state_id();

        let (value_opt, proof) = self
            .proof_provider
            .get_resource_with_proof(
                self.resource.address,
                &self.resource.resource_type,
                version,
            )
            .map_err(LightClientError::ProofProvider)?;

        let state_key = StateKey::resource(&self.resource.address, &self.resource.resource_type)
            .context("StateKey::resource")
            .map_err(LightClientError::VerifyResourceProof)?;
        let key_hash = *state_key.crypto_hash_ref();

        proof
            .verify(state_root, key_hash, value_opt.as_ref())
            .map_err(LightClientError::VerifyResourceProof)?;

        let bytes = value_opt
            .map(|v| v.bytes().to_vec())
            .unwrap_or_default();

        Ok((bytes, version))
    }
}

/// Configuration for [ResourceLightClient].
pub struct ResourceLightClientConfig {
    pub chain_id: aptos_types::chain_id::ChainId,
    pub initial_trusted_state: TrustedState,
    pub resource: MonitoredResource,
    pub proof_provider: Arc<dyn ProofProvider>,
}
