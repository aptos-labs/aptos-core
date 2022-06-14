// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    common::BLOCKCHAIN,
    error::{ApiError, ApiResult},
};
use aptos_rest_client::{aptos_api_types::TransactionInfo, Transaction};
use aptos_types::{account_address::AccountAddress, chain_id::ChainId};
use serde::{Deserialize, Serialize};
use std::{convert::TryFrom, str::FromStr};

/// [API Spec](https://www.rosetta-api.org/docs/models/AccountIdentifier.html)
///
/// TODO: Metadata?
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AccountIdentifier {
    pub address: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sub_account: Option<SubAccountIdentifier>,
}

impl AccountIdentifier {
    pub fn account_address(&self) -> ApiResult<AccountAddress> {
        // Allow 0x in front of account address
        Ok(AccountAddress::from_str(
            self.address.strip_prefix("0x").unwrap(),
        )?)
    }
}

impl From<AccountAddress> for AccountIdentifier {
    fn from(address: AccountAddress) -> Self {
        AccountIdentifier {
            address: address.to_string(),
            sub_account: None,
        }
    }
}

/// [API Spec](https://www.rosetta-api.org/docs/models/BlockIdentifier.html)
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BlockIdentifier {
    /// Version number usually known as height
    pub index: u64,
    /// Version Hash
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

impl TryFrom<Transaction> for BlockIdentifier {
    type Error = ApiError;

    fn try_from(txn: Transaction) -> Result<Self, Self::Error> {
        let txn_info = txn
            .transaction_info()
            .map_err(|err| ApiError::AptosError(err.to_string()))?;
        Ok(BlockIdentifier::from(txn_info))
    }
}

impl TryFrom<&PartialBlockIdentifier> for BlockIdentifier {
    type Error = ApiError;

    fn try_from(block: &PartialBlockIdentifier) -> Result<Self, Self::Error> {
        if block.index.is_none() || block.hash.is_none() {
            return Err(ApiError::AptosError(
                "Can't convert partial block identifier to block identifier".to_string(),
            ));
        }

        Ok(BlockIdentifier {
            index: block.index.unwrap(),
            hash: block.hash.as_ref().unwrap().clone(),
        })
    }
}

/// [API Spec](https://www.rosetta-api.org/docs/models/NetworkIdentifier.html)
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct NetworkIdentifier {
    pub blockchain: String,
    pub network: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sub_network_identifier: Option<SubNetworkIdentifier>,
}

impl NetworkIdentifier {
    pub fn chain_id(&self) -> ApiResult<ChainId> {
        ChainId::from_str(self.network.trim()).map_err(|err| ApiError::AptosError(err.to_string()))
    }
}

impl From<ChainId> for NetworkIdentifier {
    fn from(chain_id: ChainId) -> Self {
        NetworkIdentifier {
            blockchain: BLOCKCHAIN.to_string(),
            network: chain_id.to_string(),
            sub_network_identifier: None,
        }
    }
}

/// [API Spec](https://www.rosetta-api.org/docs/models/OperationIdentifier.html)
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct OperationIdentifier {
    pub index: u64,
    pub network_index: Option<u64>,
}

/// [API Spec](https://www.rosetta-api.org/docs/models/PartialBlockIdentifier.html)
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PartialBlockIdentifier {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hash: Option<String>,
}

impl From<&BlockIdentifier> for PartialBlockIdentifier {
    fn from(block: &BlockIdentifier) -> Self {
        PartialBlockIdentifier {
            index: Some(block.index),
            hash: Some(block.hash.clone()),
        }
    }
}

/// [API Spec](https://www.rosetta-api.org/docs/models/SubAccountIdentifier.html)
///
/// TODO: Metadata?
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SubAccountIdentifier {
    pub address: String,
}

/// [API Spec](https://www.rosetta-api.org/docs/models/SubNetworkIdentifier.html)
///
/// TODO: Metadata?
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SubNetworkIdentifier {
    pub network: String,
}

/// [API Spec](https://www.rosetta-api.org/docs/models/TransactionIdentifier.html)
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TransactionIdentifier {
    pub hash: String,
}
