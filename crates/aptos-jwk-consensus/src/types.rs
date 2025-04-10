// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_crypto::bls12381::Signature;
use aptos_enum_conversion_derive::EnumConversion;
use aptos_reliable_broadcast::RBMessage;
use aptos_types::{
    account_address::AccountAddress,
    jwks::{Issuer, KeyLevelUpdate, ProviderJWKs, QuorumCertifiedUpdate, KID},
};
use aptos_validator_transaction_pool::TxnGuard;
use futures_util::future::AbortHandle;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

#[derive(Clone, Debug, EnumConversion, Deserialize, Serialize, PartialEq)]
pub enum JWKConsensusMsg {
    ObservationRequest(ObservedUpdateRequest),
    ObservationResponse(ObservedUpdateResponse),
    KeyLevelObservationRequest(ObservedKeyLevelUpdateRequest),
    // In per-key mode we can reuse `ObservationResponse` and don't have a `KeyLevelObservationResponse`.
}

impl JWKConsensusMsg {
    pub fn name(&self) -> &str {
        match self {
            JWKConsensusMsg::ObservationRequest(_) => "ObservationRequest",
            JWKConsensusMsg::ObservationResponse(_) => "ObservationResponse",
            JWKConsensusMsg::KeyLevelObservationRequest(_) => "KeyLevelObservationResponse",
        }
    }

    pub fn epoch(&self) -> u64 {
        match self {
            JWKConsensusMsg::ObservationRequest(request) => request.epoch,
            JWKConsensusMsg::ObservationResponse(response) => response.epoch,
            JWKConsensusMsg::KeyLevelObservationRequest(request) => request.epoch,
        }
    }
}

impl RBMessage for JWKConsensusMsg {}

#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq)]
pub struct ObservedUpdate {
    pub author: AccountAddress,
    pub observed: ProviderJWKs,
    pub signature: Signature,
}

#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq)]
pub struct ObservedKeyLevelUpdate {
    pub author: AccountAddress,
    pub observed: KeyLevelUpdate,
    pub signature: Signature,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct ObservedUpdateRequest {
    pub epoch: u64,
    pub issuer: Issuer,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct ObservedUpdateResponse {
    pub epoch: u64,
    pub update: ObservedUpdate,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct ObservedKeyLevelUpdateRequest {
    pub epoch: u64,
    pub issuer: Issuer,
    pub kid: KID,
}

/// An instance of this resource is created when `JWKManager` starts the QC update building process for an issuer.
/// Then `JWKManager` needs to hold it. Once this resource is dropped, the corresponding QC update process will be cancelled.
#[derive(Clone, Debug)]
pub struct QuorumCertProcessGuard {
    pub handle: AbortHandle,
}

impl QuorumCertProcessGuard {
    pub fn new(handle: AbortHandle) -> Self {
        Self { handle }
    }

    #[cfg(test)]
    pub fn dummy() -> Self {
        let (handle, _) = AbortHandle::new_pair();
        Self { handle }
    }
}

impl Drop for QuorumCertProcessGuard {
    fn drop(&mut self) {
        let QuorumCertProcessGuard { handle } = self;
        handle.abort();
    }
}

#[derive(Debug, Clone)]
pub enum ConsensusState<T: Debug + Clone + Eq + PartialEq> {
    NotStarted,
    InProgress {
        my_proposal: T,
        abort_handle_wrapper: QuorumCertProcessGuard,
    },
    Finished {
        vtxn_guard: TxnGuard,
        my_proposal: T,
        quorum_certified: QuorumCertifiedUpdate,
    },
}

impl<T: Debug + Clone + Eq + PartialEq> PartialEq for ConsensusState<T> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (ConsensusState::NotStarted, ConsensusState::NotStarted) => true,
            (
                ConsensusState::InProgress {
                    my_proposal: update_0,
                    ..
                },
                ConsensusState::InProgress {
                    my_proposal: update_1,
                    ..
                },
            ) if update_0 == update_1 => true,
            (
                ConsensusState::Finished {
                    my_proposal: update_0,
                    ..
                },
                ConsensusState::Finished {
                    my_proposal: update_1,
                    ..
                },
            ) if update_0 == update_1 => true,
            _ => false,
        }
    }
}

impl<T: Debug + Clone + Eq + PartialEq> Eq for ConsensusState<T> {}

impl<T: Debug + Clone + Eq + PartialEq> ConsensusState<T> {
    pub fn name(&self) -> &str {
        match self {
            ConsensusState::NotStarted => "NotStarted",
            ConsensusState::InProgress { .. } => "InProgress",
            ConsensusState::Finished { .. } => "Finished",
        }
    }

    #[cfg(test)]
    pub fn my_proposal_cloned(&self) -> T {
        match self {
            ConsensusState::InProgress { my_proposal, .. }
            | ConsensusState::Finished { my_proposal, .. } => my_proposal.clone(),
            _ => panic!("my_proposal unavailable"),
        }
    }
}

impl<T: Debug + Clone + Eq + PartialEq> Default for ConsensusState<T> {
    fn default() -> Self {
        Self::NotStarted
    }
}
