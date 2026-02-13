// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Light client that monitors a single on-chain resource.
//!
//! The client maintains a [TrustedState] (waypoint + optional validator set), ratchets it
//! forward using [StateProof]s from a [ProofProvider], and can then fetch and verify the
//! value of one resource at the trusted version.

mod client;
mod proof_provider;

pub use client::{ResourceLightClient, ResourceLightClientConfig};
pub use proof_provider::{LightClientError, ProofProvider};

// Re-export key types so callers can use them without depending on aptos-types directly
// for the light client API.
pub use aptos_types::{
    chain_id::ChainId,
    ledger_info::LedgerInfo,
    state_proof::StateProof,
    state_store::state_value::StateValue,
    trusted_state::TrustedState,
    waypoint::Waypoint,
};
pub use move_core_types::language_storage::StructTag;
pub use aptos_types::account_address::AccountAddress;

/// The single resource to monitor: (address, resource type).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MonitoredResource {
    pub address: AccountAddress,
    pub resource_type: StructTag,
}

impl MonitoredResource {
    pub fn new(address: AccountAddress, resource_type: StructTag) -> Self {
        Self {
            address,
            resource_type,
        }
    }
}
