// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! Identifiers for the Rosetta spec
//!
//! [Spec](https://www.rosetta-api.org/docs/api_identifiers.html)

use crate::{
    common::{to_hex_lower, BLOCKCHAIN},
    error::{ApiError, ApiResult},
};
use aptos_rest_client::aptos_api_types::{BlockInfo, TransactionInfo};
use aptos_types::{account_address::AccountAddress, chain_id::ChainId};
use serde::{Deserialize, Serialize};
use std::{
    convert::{TryFrom, TryInto},
    str::FromStr,
};

/// Account identifier, specified as a hex encoded account address (with leading 0x)
///
/// [API Spec](https://www.rosetta-api.org/docs/models/AccountIdentifier.html)
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AccountIdentifier {
    /// Hex encoded AccountAddress beginning with 0x
    pub address: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sub_account: Option<SubAccountIdentifier>,
}

impl AccountIdentifier {
    /// Convert [`AccountIdentifier`] to an [`AccountAddress`]
    pub fn account_address(&self) -> ApiResult<AccountAddress> {
        self.try_into()
    }
}

impl TryFrom<&AccountIdentifier> for AccountAddress {
    type Error = ApiError;

    fn try_from(account: &AccountIdentifier) -> Result<Self, Self::Error> {
        // Allow 0x in front of account address
        if let Ok(address) = AccountAddress::from_hex_literal(&account.address) {
            Ok(address)
        } else {
            Ok(AccountAddress::from_str(&account.address)
                .map_err(|_| ApiError::AptosError(Some("Invalid account address".to_string())))?)
        }
    }
}

impl From<AccountAddress> for AccountIdentifier {
    fn from(address: AccountAddress) -> Self {
        AccountIdentifier {
            address: to_hex_lower(&address),
            sub_account: None,
        }
    }
}

/// Identifier for a "block".  In aptos, we use a transaction model, so the index
/// represents multiple transactions in a "block" grouping of transactions
///
/// [API Spec](https://www.rosetta-api.org/docs/models/BlockIdentifier.html)
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BlockIdentifier {
    /// Block index, which points to a txn at the beginning of a "block"
    pub index: u64,
    /// Accumulator hash at the beginning of the block
    pub hash: String,
}

impl BlockIdentifier {
    pub fn from_block_info(block_info: BlockInfo) -> BlockIdentifier {
        BlockIdentifier {
            index: block_info.block_height,
            hash: to_hex_lower(&block_info.block_hash),
        }
    }
}

/// Identifier for this specific network deployment
///
/// [API Spec](https://www.rosetta-api.org/docs/models/NetworkIdentifier.html)
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct NetworkIdentifier {
    /// Blockchain name, should always be `aptos` and be hardcoded
    pub blockchain: String,
    /// Network name which we use ChainId for it
    pub network: String,
    /// Can be used in the future for a shard identifier
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sub_network_identifier: Option<SubNetworkIdentifier>,
}

impl NetworkIdentifier {
    pub fn chain_id(&self) -> ApiResult<ChainId> {
        self.try_into()
    }
}

impl TryFrom<&NetworkIdentifier> for ChainId {
    type Error = ApiError;

    fn try_from(network_identifier: &NetworkIdentifier) -> Result<Self, Self::Error> {
        ChainId::from_str(network_identifier.network.trim())
            .map_err(|err| ApiError::AptosError(Some(err.to_string())))
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

/// Identifies a specific [`crate::types::Operation`] within a [`Transaction`]
///
///
/// [API Spec](https://www.rosetta-api.org/docs/models/OperationIdentifier.html)
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct OperationIdentifier {
    /// The unique index of the operation within a transaction
    ///
    /// It must be 0 to n within the transaction.
    pub index: u64,
    /// Only necessary if operation order is required
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network_index: Option<u64>,
}

/// Partial block identifier for querying by version or by hash.  Both should not be
/// provided at the same time.
///
/// [API Spec](https://www.rosetta-api.org/docs/models/PartialBlockIdentifier.html)
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PartialBlockIdentifier {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<u64>,
    /// Hash of the block
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hash: Option<String>,
}

impl PartialBlockIdentifier {
    pub fn latest() -> Self {
        Self {
            index: None,
            hash: None,
        }
    }

    pub fn by_hash(hash: String) -> Self {
        Self {
            index: None,
            hash: Some(hash),
        }
    }

    pub fn by_version(version: u64) -> Self {
        Self {
            index: Some(version),
            hash: None,
        }
    }
}

/// Sub account identifier if there are sub accounts
///
/// [API Spec](https://www.rosetta-api.org/docs/models/SubAccountIdentifier.html)
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SubAccountIdentifier {
    pub address: String,
}

/// Sub network identifier if there are sub networks
///
/// [API Spec](https://www.rosetta-api.org/docs/models/SubNetworkIdentifier.html)
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SubNetworkIdentifier {
    pub network: String,
}

/// TransactionIdentifier to represent a transaction by hash
///
/// [API Spec](https://www.rosetta-api.org/docs/models/TransactionIdentifier.html)
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TransactionIdentifier {
    /// The hash of the transaction so it can be looked up in mempool
    pub hash: String,
}

impl From<&TransactionInfo> for TransactionIdentifier {
    fn from(txn: &TransactionInfo) -> Self {
        TransactionIdentifier {
            hash: to_hex_lower(&txn.hash),
        }
    }
}
