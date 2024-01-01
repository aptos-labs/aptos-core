// Copyright © Aptos Foundation

use aptos_crypto::bls12381::Signature;
use aptos_types::{
    account_address::AccountAddress,
    jwks::{Issuer, ProviderJWKs},
};
use serde::{Deserialize, Serialize};

impl JWKConsensusMsg {
    pub fn name(&self) -> &str {
        match self {
            JWKConsensusMsg::ObservationRequest(_) => "ObservationRequest",
            JWKConsensusMsg::ObservationResponse(_) => "ObservationResponse",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub enum JWKConsensusMsg {
    ObservationRequest(ObservedUpdateRequest),
    ObservationResponse(ObservedUpdateResponse),
}

#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq)]
pub struct ObservedUpdate {
    pub author: AccountAddress,
    pub observed: ProviderJWKs,
    pub signature: Signature,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct ObservedUpdateRequest {
    pub issuer: Issuer,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct ObservedUpdateResponse {
    pub update: ObservedUpdate,
}
