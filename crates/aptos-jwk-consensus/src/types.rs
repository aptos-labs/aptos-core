// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_crypto::bls12381::Signature;
use aptos_enum_conversion_derive::EnumConversion;
use aptos_reliable_broadcast::RBMessage;
use aptos_types::{
    account_address::AccountAddress,
    jwks::{Issuer, ProviderJWKs},
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, EnumConversion, Deserialize, Serialize, PartialEq)]
pub enum JWKConsensusMsg {
    ObservationRequest(ObservedUpdateRequest),
    ObservationResponse(ObservedUpdateResponse),
}

impl JWKConsensusMsg {
    pub fn name(&self) -> &str {
        match self {
            JWKConsensusMsg::ObservationRequest(_) => "ObservationRequest",
            JWKConsensusMsg::ObservationResponse(_) => "ObservationResponse",
        }
    }

    pub fn epoch(&self) -> u64 {
        match self {
            JWKConsensusMsg::ObservationRequest(request) => request.epoch,
            JWKConsensusMsg::ObservationResponse(response) => response.epoch,
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
