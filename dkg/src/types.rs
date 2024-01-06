// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::bail;
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use aptos_enum_conversion_derive::EnumConversion;
use aptos_reliable_broadcast::RBMessage;
pub use aptos_types::dkg::DKGAggNode;
use move_core_types::account_address::AccountAddress;
use serde::{Deserialize, Serialize};

/// Represents the metadata about the node, without payload and parents from Node
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, CryptoHasher, BCSCryptoHash)]
pub struct DKGNodeMetadata {
    epoch: u64,
    author: AccountAddress,
}

impl DKGNodeMetadata {
    #[cfg(test)]
    pub fn new_for_test(epoch: u64, author: AccountAddress) -> Self {
        Self { epoch, author }
    }

    pub fn author(&self) -> &AccountAddress {
        &self.author
    }

    pub fn epoch(&self) -> u64 {
        self.epoch
    }
}

#[derive(Clone, Serialize, Deserialize, CryptoHasher, Debug, PartialEq)]
pub struct DKGNodeRequest {
    dealer_epoch: u64,
}

impl DKGNodeRequest {
    pub fn new(epoch: u64) -> Self {
        Self {
            dealer_epoch: epoch,
        }
    }
}
#[derive(Clone, Serialize, Deserialize, CryptoHasher, Debug, PartialEq)]
pub struct DKGNode {
    //TODO
}

impl DKGNode {
    pub fn epoch(&self) -> u64 {
        //TODO
        0
    }

    pub fn author(&self) -> AccountAddress {
        //TODO
        AccountAddress::ZERO
    }
}

#[derive(Clone, Serialize, Deserialize, Debug, EnumConversion, PartialEq)]
pub enum DKGMessage {
    NodeRequest(DKGNodeRequest),
    NodeResponse(DKGNode),
}

impl DKGMessage {
    pub fn name(&self) -> &str {
        match self {
            DKGMessage::NodeRequest(_) => "DKGNodeRequest",
            DKGMessage::NodeResponse(_) => "DKGNodeResponse",
        }
    }

    pub fn author(&self) -> anyhow::Result<AccountAddress> {
        match self {
            DKGMessage::NodeResponse(node) => Ok(node.author()),
            _ => bail!("message does not support author field"),
        }
    }
}

impl RBMessage for DKGMessage {}

impl DKGMessage {
    pub fn epoch(&self) -> u64 {
        match self {
            DKGMessage::NodeRequest(req) => req.dealer_epoch,
            DKGMessage::NodeResponse(node) => node.epoch(),
        }
    }
}
