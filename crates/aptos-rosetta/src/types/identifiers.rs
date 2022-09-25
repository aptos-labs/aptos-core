// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! Identifiers for the Rosetta spec
//!
//! [Spec](https://www.rosetta-api.org/docs/api_identifiers.html)

use crate::common::BlockHash;
use crate::{
    common::{to_hex_lower, BLOCKCHAIN},
    error::{ApiError, ApiResult},
};
use aptos_types::transaction::TransactionInfo;
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
    /// Sub account only used for staking
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sub_account: Option<SubAccountIdentifier>,
}

impl AccountIdentifier {
    /// Convert [`AccountIdentifier`] to an [`AccountAddress`]
    pub fn account_address(&self) -> ApiResult<AccountAddress> {
        str_to_account_address(self.address.as_str())
    }

    pub fn base_account(address: AccountAddress) -> Self {
        AccountIdentifier {
            address: to_hex_lower(&address),
            sub_account: None,
        }
    }

    pub fn total_stake_account(address: AccountAddress) -> Self {
        AccountIdentifier {
            address: to_hex_lower(&address),
            sub_account: Some(SubAccountIdentifier::new_total_stake()),
        }
    }

    pub fn operator_stake_account(
        address: AccountAddress,
        operator_address: AccountAddress,
    ) -> Self {
        AccountIdentifier {
            address: to_hex_lower(&address),
            sub_account: Some(SubAccountIdentifier::new_operator_stake(operator_address)),
        }
    }

    pub fn is_base_account(&self) -> bool {
        self.sub_account.is_none()
    }
    pub fn is_total_stake(&self) -> bool {
        if let Some(ref inner) = self.sub_account {
            inner.is_total_stake()
        } else {
            false
        }
    }

    pub fn is_operator_stake(&self) -> bool {
        if let Some(ref inner) = self.sub_account {
            !inner.is_total_stake()
        } else {
            false
        }
    }

    pub fn operator_address(&self) -> ApiResult<AccountAddress> {
        if let Some(ref inner) = self.sub_account {
            inner.operator_address()
        } else {
            Err(ApiError::InternalError(Some(
                "Can't get operator address of a non-operator stake account".to_string(),
            )))
        }
    }
}

fn str_to_account_address(address: &str) -> Result<AccountAddress, ApiError> {
    AccountAddress::from_str(address)
        .map_err(|_| ApiError::InvalidInput(Some("Invalid account address".to_string())))
}

/// There are two types of SubAccountIdentifiers
/// 1. "stake" which is the total stake
/// 2. "stake-<operator>" which is the stake on the operator
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SubAccountIdentifier {
    /// Hex encoded AccountAddress beginning with 0x
    pub address: String,
}

const STAKE: &str = "stake";
const ACCOUNT_SEPARATOR: char = '-';

impl SubAccountIdentifier {
    pub fn new_total_stake() -> SubAccountIdentifier {
        SubAccountIdentifier {
            address: STAKE.to_string(),
        }
    }

    pub fn new_operator_stake(operator: AccountAddress) -> SubAccountIdentifier {
        SubAccountIdentifier {
            address: format!("{}-{}", STAKE, to_hex_lower(&operator)),
        }
    }

    pub fn is_total_stake(&self) -> bool {
        self.address.as_str() == STAKE
    }

    pub fn operator_address(&self) -> ApiResult<AccountAddress> {
        let mut parts = self.address.split(ACCOUNT_SEPARATOR);

        if let Some(stake) = parts.next() {
            if stake == STAKE {
                if let Some(operator) = parts.next() {
                    return str_to_account_address(operator);
                }
            }
        }

        Err(ApiError::InvalidInput(Some(format!(
            "Sub account isn't an operator address {:?}",
            self
        ))))
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
    pub fn from_block(
        block: &aptos_rest_client::aptos_api_types::BcsBlock,
        chain_id: ChainId,
    ) -> BlockIdentifier {
        BlockIdentifier {
            index: block.block_height,
            hash: BlockHash::new(chain_id, block.block_height).to_string(),
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
            .map_err(|err| ApiError::InvalidInput(Some(err.to_string())))
    }
}

impl From<ChainId> for NetworkIdentifier {
    fn from(chain_id: ChainId) -> Self {
        NetworkIdentifier {
            blockchain: BLOCKCHAIN.to_string(),
            network: chain_id.to_string(),
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

    pub fn block_index(index: u64) -> Self {
        Self {
            index: Some(index),
            hash: None,
        }
    }
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
            hash: to_hex_lower(&txn.transaction_hash()),
        }
    }
}

impl From<aptos_crypto::HashValue> for TransactionIdentifier {
    fn from(hash: aptos_crypto::HashValue) -> Self {
        TransactionIdentifier {
            hash: to_hex_lower(&hash),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_account_id() {
        let account = AccountAddress::ONE;
        let operator = AccountAddress::ZERO;

        let base_account = AccountIdentifier::base_account(account);
        let total_stake_account = AccountIdentifier::total_stake_account(account);
        let operator_stake_account = AccountIdentifier::operator_stake_account(account, operator);

        assert!(base_account.is_base_account());
        assert!(!operator_stake_account.is_base_account());
        assert!(!total_stake_account.is_base_account());

        assert!(!base_account.is_operator_stake());
        assert!(operator_stake_account.is_operator_stake());
        assert!(!total_stake_account.is_operator_stake());

        assert!(!base_account.is_total_stake());
        assert!(!operator_stake_account.is_total_stake());
        assert!(total_stake_account.is_total_stake());

        assert_eq!(Ok(account), base_account.account_address());
        assert_eq!(Ok(account), operator_stake_account.account_address());
        assert_eq!(Ok(account), total_stake_account.account_address());

        assert!(base_account.operator_address().is_err());
        assert_eq!(Ok(operator), operator_stake_account.operator_address());
        assert!(total_stake_account.operator_address().is_err());
    }

    #[test]
    fn test_sub_account_id() {
        let stake = SubAccountIdentifier::new_total_stake();
        assert!(stake.is_total_stake());

        let operator_address = AccountAddress::ZERO;
        let operator = SubAccountIdentifier::new_operator_stake(operator_address);
        assert!(!operator.is_total_stake());
        assert_eq!(Ok(operator_address), operator.operator_address());

        assert!(stake.operator_address().is_err());
    }
}
