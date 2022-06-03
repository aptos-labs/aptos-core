// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::error::ApiError;
use aptos_rest_client::aptos_api_types::TransactionInfo;
use aptos_types::account_address::AccountAddress;
use serde::{Deserialize, Serialize};
use std::{
    fmt::{Display, Formatter},
    str::FromStr,
};

const MAINNET: &str = "mainnet";
const TESTNET: &str = "testnet";
const DEVNET: &str = "devnet";
const LOCAL: &str = "local";

/// Network identifier
///
/// TODO: Should this just be ChainId?
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Network {
    Mainnet,
    Testnet,
    Devnet,
    Local,
}

impl Display for Network {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Network::Mainnet => MAINNET,
            Network::Testnet => TESTNET,
            Network::Devnet => DEVNET,
            Network::Local => LOCAL,
        })
    }
}

impl FromStr for Network {
    type Err = ApiError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            MAINNET => Ok(Network::Mainnet),
            TESTNET => Ok(Network::Testnet),
            DEVNET => Ok(Network::Devnet),
            LOCAL => Ok(Network::Local),
            _ => Err(ApiError::BadNetwork),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Amount {
    pub value: String,
    pub currency: Currency,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Currency {
    pub symbol: String,
    pub decimals: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Error {
    pub code: u64,
    pub message: String,
    pub retriable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<ErrorDetails>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AccountIdentifier {
    pub address: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sub_account: Option<SubAccountIdentifier>,
}

impl From<AccountAddress> for AccountIdentifier {
    fn from(address: AccountAddress) -> Self {
        AccountIdentifier {
            address: address.to_string(),
            sub_account: None,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BlockIdentifier {
    pub index: u64,
    pub hash: String,
}

impl From<&TransactionInfo> for BlockIdentifier {
    fn from(info: &TransactionInfo) -> Self {
        BlockIdentifier {
            index: info.version.0,
            hash: info.hash.to_string(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct NetworkIdentifier {
    pub blockchain: String,
    pub network: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sub_network_identifier: Option<SubNetworkIdentifier>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PartialBlockIdentifier {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hash: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SubAccountIdentifier {
    pub address: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SubNetworkIdentifier {
    pub network: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AccountBalanceRequest {
    pub network_identifier: NetworkIdentifier,
    pub account_identifier: AccountIdentifier,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_identifier: Option<PartialBlockIdentifier>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AccountBalanceResponse {
    pub block_identifier: BlockIdentifier,
    pub balances: Vec<Amount>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ErrorDetails {
    /// The detailed error
    pub error: String,
}
