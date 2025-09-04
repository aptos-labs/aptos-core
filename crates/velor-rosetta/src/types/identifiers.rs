// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

//! Identifiers for the Rosetta spec
//!
//! [Spec](https://www.rosetta-api.org/docs/api_identifiers.html)

use crate::{
    common::{to_hex_lower, BlockHash, BLOCKCHAIN},
    error::{ApiError, ApiResult},
};
use velor_types::{
    account_address::AccountAddress, chain_id::ChainId, transaction::TransactionInfo,
};
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

    /// Retrieve the pool address from an [`AccountIdentifier`], if it exists
    pub fn pool_address(&self) -> ApiResult<Option<AccountAddress>> {
        if let Some(sub_account) = &self.sub_account {
            if let Some(metadata) = &sub_account.metadata {
                return str_to_account_address(metadata.pool_address.as_str()).map(Some);
            }
        }

        Ok(None)
    }

    /// Builds a normal account [`AccountIdentifier`] for a given address
    pub fn base_account(address: AccountAddress) -> Self {
        AccountIdentifier {
            address: to_hex_lower(&address),
            sub_account: None,
        }
    }

    /// Builds a stake account [`AccountIdentifier`] for a given address to retrieve stake balances
    pub fn total_stake_account(address: AccountAddress) -> Self {
        AccountIdentifier {
            address: to_hex_lower(&address),
            sub_account: Some(SubAccountIdentifier::new_total_stake()),
        }
    }

    /// Builds a pending active stake account [`AccountIdentifier`] for a given address to retrieve pending active stake balances
    pub fn pending_active_stake_account(address: AccountAddress) -> Self {
        AccountIdentifier {
            address: to_hex_lower(&address),
            sub_account: Some(SubAccountIdentifier::new_pending_active_stake()),
        }
    }

    /// Builds a active stake account [`AccountIdentifier`] for a given address to retrieve active stake balances
    pub fn active_stake_account(address: AccountAddress) -> Self {
        AccountIdentifier {
            address: to_hex_lower(&address),
            sub_account: Some(SubAccountIdentifier::new_active_stake()),
        }
    }

    /// Builds a pending inactive stake account [`AccountIdentifier`] for a given address to retrieve pending inactive stake balances
    pub fn pending_inactive_stake_account(address: AccountAddress) -> Self {
        AccountIdentifier {
            address: to_hex_lower(&address),
            sub_account: Some(SubAccountIdentifier::new_pending_inactive_stake()),
        }
    }

    /// Builds a inactive stake account [`AccountIdentifier`] for a given address to retrieve inactive stake balances
    pub fn inactive_stake_account(address: AccountAddress) -> Self {
        AccountIdentifier {
            address: to_hex_lower(&address),
            sub_account: Some(SubAccountIdentifier::new_inactive_stake()),
        }
    }

    /// Builds an operator stake account [`AccountIdentifier`] for a given address to retrieve operator stake balances
    pub fn operator_stake_account(
        address: AccountAddress,
        operator_address: AccountAddress,
    ) -> Self {
        AccountIdentifier {
            address: to_hex_lower(&address),
            sub_account: Some(SubAccountIdentifier::new_operator_stake(operator_address)),
        }
    }

    /// Returns true if the account doesn't have a sub account
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

    pub fn is_commission(&self) -> bool {
        if let Some(ref inner) = self.sub_account {
            inner.is_commission()
        } else {
            false
        }
    }

    pub fn is_rewards(&self) -> bool {
        if let Some(ref inner) = self.sub_account {
            inner.is_rewards()
        } else {
            false
        }
    }

    pub fn is_pending_active_stake(&self) -> bool {
        if let Some(ref inner) = self.sub_account {
            inner.is_pending_active_stake()
        } else {
            false
        }
    }

    pub fn is_active_stake(&self) -> bool {
        if let Some(ref inner) = self.sub_account {
            inner.is_active_stake()
        } else {
            false
        }
    }

    pub fn is_pending_inactive_stake(&self) -> bool {
        if let Some(ref inner) = self.sub_account {
            inner.is_pending_inactive_stake()
        } else {
            false
        }
    }

    pub fn is_inactive_stake(&self) -> bool {
        if let Some(ref inner) = self.sub_account {
            inner.is_inactive_stake()
        } else {
            false
        }
    }

    pub fn is_delegator_active_stake(&self) -> bool {
        if let Some(ref inner) = self.sub_account {
            inner.is_delegator_active_stake()
        } else {
            false
        }
    }

    pub fn is_delegator_inactive_stake(&self) -> bool {
        if let Some(ref inner) = self.sub_account {
            inner.is_delegator_inactive_stake()
        } else {
            false
        }
    }

    pub fn is_delegator_pending_inactive_stake(&self) -> bool {
        if let Some(ref inner) = self.sub_account {
            inner.is_delegator_pending_inactive_stake()
        } else {
            false
        }
    }

    pub fn is_operator_stake(&self) -> bool {
        if let Some(ref inner) = self.sub_account {
            !(inner.is_total_stake()
                || inner.is_active_stake()
                || inner.is_pending_active_stake()
                || inner.is_inactive_stake()
                || inner.is_pending_inactive_stake())
        } else {
            false
        }
    }

    /// Retrieves the operator address if it has one in the sub-account
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

/// Converts a string to an account address with error handling
fn str_to_account_address(address: &str) -> Result<AccountAddress, ApiError> {
    AccountAddress::from_str(address)
        .map_err(|_| ApiError::InvalidInput(Some("Invalid account address".to_string())))
}

/// There are many types of SubAccountIdentifiers
/// 1. `stake` which is the total stake
/// 2. `stake-<operator>` which is the stake on the operator
/// 3. And more for pool addresses and various stake types
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SubAccountIdentifier {
    /// Hex encoded AccountAddress beginning with 0x
    pub address: String,
    /// Metadata only used for delegated staking
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<SubAccountIdentifierMetadata>,
}

const STAKE: &str = "stake";
const PENDING_ACTIVE_STAKE: &str = "pending_active_stake";
const ACTIVE_STAKE: &str = "active_stake";
const PENDING_INACTIVE_STAKE: &str = "pending_inactive_stake";
const INACTIVE_STAKE: &str = "inactive_stake";
const COMMISSION: &str = "commission";
const REWARDS: &str = "rewards";
const ACCOUNT_SEPARATOR: char = '-';

impl SubAccountIdentifier {
    pub fn new_total_stake() -> SubAccountIdentifier {
        SubAccountIdentifier {
            address: STAKE.to_string(),
            metadata: None,
        }
    }

    pub fn new_pending_active_stake() -> SubAccountIdentifier {
        SubAccountIdentifier {
            address: PENDING_ACTIVE_STAKE.to_string(),
            metadata: None,
        }
    }

    pub fn new_active_stake() -> SubAccountIdentifier {
        SubAccountIdentifier {
            address: ACTIVE_STAKE.to_string(),
            metadata: None,
        }
    }

    pub fn new_pending_inactive_stake() -> SubAccountIdentifier {
        SubAccountIdentifier {
            address: PENDING_INACTIVE_STAKE.to_string(),
            metadata: None,
        }
    }

    pub fn new_inactive_stake() -> SubAccountIdentifier {
        SubAccountIdentifier {
            address: INACTIVE_STAKE.to_string(),
            metadata: None,
        }
    }

    pub fn new_delegated_total_stake(pool: &str) -> SubAccountIdentifier {
        SubAccountIdentifier {
            address: STAKE.to_string(),
            metadata: Some(SubAccountIdentifierMetadata::new_pool_address(
                AccountAddress::from_str(pool).unwrap(),
            )),
        }
    }

    pub fn new_delegated_active_stake(pool: &str) -> SubAccountIdentifier {
        SubAccountIdentifier {
            address: ACTIVE_STAKE.to_string(),
            metadata: Some(SubAccountIdentifierMetadata::new_pool_address(
                AccountAddress::from_str(pool).unwrap(),
            )),
        }
    }

    pub fn new_delegated_pending_inactive_stake(pool: &str) -> SubAccountIdentifier {
        SubAccountIdentifier {
            address: PENDING_INACTIVE_STAKE.to_string(),
            metadata: Some(SubAccountIdentifierMetadata::new_pool_address(
                AccountAddress::from_str(pool).unwrap(),
            )),
        }
    }

    pub fn new_delegated_inactive_stake(pool: &str) -> SubAccountIdentifier {
        SubAccountIdentifier {
            address: INACTIVE_STAKE.to_string(),
            metadata: Some(SubAccountIdentifierMetadata::new_pool_address(
                AccountAddress::from_str(pool).unwrap(),
            )),
        }
    }

    pub fn new_operator_stake(operator: AccountAddress) -> SubAccountIdentifier {
        SubAccountIdentifier {
            address: format!("{}-{}", STAKE, to_hex_lower(&operator)),
            metadata: None,
        }
    }

    pub fn new_commission() -> SubAccountIdentifier {
        SubAccountIdentifier {
            address: COMMISSION.to_string(),
            metadata: None,
        }
    }

    pub fn new_rewards() -> SubAccountIdentifier {
        SubAccountIdentifier {
            address: REWARDS.to_string(),
            metadata: None,
        }
    }

    pub fn is_total_stake(&self) -> bool {
        self.address.as_str() == STAKE
    }

    pub fn is_pending_active_stake(&self) -> bool {
        self.address.as_str() == PENDING_ACTIVE_STAKE
    }

    pub fn is_active_stake(&self) -> bool {
        self.address.as_str() == ACTIVE_STAKE && self.metadata.is_none()
    }

    pub fn is_pending_inactive_stake(&self) -> bool {
        self.address.as_str() == PENDING_INACTIVE_STAKE && self.metadata.is_none()
    }

    pub fn is_inactive_stake(&self) -> bool {
        self.address.as_str() == INACTIVE_STAKE && self.metadata.is_none()
    }

    pub fn is_commission(&self) -> bool {
        self.address.as_str() == COMMISSION && self.metadata.is_none()
    }

    pub fn is_rewards(&self) -> bool {
        self.address.as_str() == REWARDS && self.metadata.is_none()
    }

    pub fn is_delegator_active_stake(&self) -> bool {
        self.address.as_str() == ACTIVE_STAKE && self.metadata.is_some()
    }

    pub fn is_delegator_inactive_stake(&self) -> bool {
        self.address.as_str() == INACTIVE_STAKE && self.metadata.is_some()
    }

    pub fn is_delegator_pending_inactive_stake(&self) -> bool {
        self.address.as_str() == PENDING_INACTIVE_STAKE && self.metadata.is_some()
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

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SubAccountIdentifierMetadata {
    /// Hex encoded Pool beginning with 0x
    pub pool_address: String,
}

impl SubAccountIdentifierMetadata {
    pub fn new_pool_address(pool_address: AccountAddress) -> Self {
        SubAccountIdentifierMetadata {
            pool_address: to_hex_lower(&pool_address),
        }
    }
}

/// Identifier for a "block".  On Velor, we use a transaction model, so the index
/// represents multiple transactions in a "block" grouping of transactions
///
/// [API Spec](https://www.rosetta-api.org/docs/models/BlockIdentifier.html)
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BlockIdentifier {
    /// Block index, which points to a txn at the beginning of a "block"
    pub index: u64,
    /// A fake hash, that is actually `chain_id-block_height`
    pub hash: String,
}

impl BlockIdentifier {
    pub fn from_block(
        block: &velor_rest_client::velor_api_types::BcsBlock,
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
    /// Blockchain name, should always be `velor` and be hardcoded
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

/// Identifies a specific [`crate::types::Operation`] within a `Transaction`
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

impl From<velor_crypto::HashValue> for TransactionIdentifier {
    fn from(hash: velor_crypto::HashValue) -> Self {
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
        let active_stake_account = AccountIdentifier::active_stake_account(account);
        let pending_active_stake_account = AccountIdentifier::pending_active_stake_account(account);
        let inactive_stake_account = AccountIdentifier::inactive_stake_account(account);
        let pending_inactive_stake_account =
            AccountIdentifier::pending_inactive_stake_account(account);

        assert!(base_account.is_base_account());
        assert!(!operator_stake_account.is_base_account());
        assert!(!total_stake_account.is_base_account());
        assert!(!active_stake_account.is_base_account());
        assert!(!pending_active_stake_account.is_base_account());
        assert!(!inactive_stake_account.is_base_account());
        assert!(!pending_inactive_stake_account.is_base_account());

        assert!(!base_account.is_operator_stake());
        assert!(operator_stake_account.is_operator_stake());
        assert!(!total_stake_account.is_operator_stake());

        assert!(!base_account.is_total_stake());
        assert!(!operator_stake_account.is_total_stake());
        assert!(total_stake_account.is_total_stake());

        assert!(active_stake_account.is_active_stake());
        assert!(pending_active_stake_account.is_pending_active_stake());
        assert!(inactive_stake_account.is_inactive_stake());
        assert!(pending_inactive_stake_account.is_pending_inactive_stake());

        assert_eq!(Ok(account), base_account.account_address());
        assert_eq!(Ok(account), operator_stake_account.account_address());
        assert_eq!(Ok(account), total_stake_account.account_address());
        assert_eq!(Ok(account), active_stake_account.account_address());
        assert_eq!(Ok(account), pending_active_stake_account.account_address());
        assert_eq!(Ok(account), inactive_stake_account.account_address());
        assert_eq!(
            Ok(account),
            pending_inactive_stake_account.account_address()
        );

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
