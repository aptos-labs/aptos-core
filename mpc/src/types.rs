// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};
use aptos_enum_conversion_derive::EnumConversion;
use aptos_reliable_broadcast::RBMessage;
use aptos_crypto_derive::CryptoHasher;

#[derive(Clone, Serialize, Deserialize, CryptoHasher, Debug, PartialEq)]
pub struct ResultShareRequest {
    dealer_epoch: u64,
    //mpc todo
}

impl ResultShareRequest {
    pub fn new(epoch: u64) -> Self {
        Self {
            dealer_epoch: epoch,
        }
    }
}

#[derive(Clone, Serialize, Deserialize, CryptoHasher, Debug, PartialEq)]
pub struct ResultShareResponse {
    dealer_epoch: u64,
    //mpc todo
}

impl ResultShareResponse {
    pub fn new(epoch: u64) -> Self {
        Self {
            dealer_epoch: epoch,
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug, EnumConversion, PartialEq)]
pub enum MPCMessage {
    ResultShareRequest(ResultShareRequest),
    ResultShareResponse(ResultShareResponse),
}

impl MPCMessage {
    pub fn epoch(&self) -> u64 {
        match self {
            MPCMessage::ResultShareRequest(request) => request.dealer_epoch,
            MPCMessage::ResultShareResponse(response) => response.dealer_epoch,
        }
    }

    pub fn name(&self) -> &str {
        match self {
            MPCMessage::ResultShareRequest(_) => "MPCShareRequest",
            MPCMessage::ResultShareResponse(_) => "MPCShareResponse",
        }
    }
}

impl RBMessage for MPCMessage {}
