// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    account::view,
    common::native_coin,
    error::ApiError,
    types::{AccountIdentifier, Amount, STAKING_CONTRACT_MODULE},
    AccountAddress, ApiResult,
};
use aptos_rest_client::aptos_api_types::{EntryFunctionId, ViewRequest};
use aptos_types::stake_pool::StakePool;
use move_core_types::ident_str;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::{
    convert::TryFrom,
    fmt::{Display, Formatter},
    str::FromStr,
};

// TODO: Move these to `move_types.rs`
static DELEGATION_POOL_GET_STAKE_FUNCTION: Lazy<EntryFunctionId> =
    Lazy::new(|| "0x1::delegation_pool::get_stake".parse().unwrap());
static STAKE_GET_LOCKUP_SECS_FUNCTION: Lazy<EntryFunctionId> =
    Lazy::new(|| "0x1::stake::get_lockup_secs".parse().unwrap());

/// Errors that can be returned by the API
///
/// Internally [`ApiError`] is used, but it is converted to this for on wire representation
///
/// [API Spec](https://www.rosetta-api.org/docs/models/Error.html)
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Error {
    /// Error code
    pub code: u32,
    /// Message that always matches the error code
    pub message: String,
    /// Whether a call can retry on the error
    pub retriable: bool,
    /// Specific details of the error e.g. stack trace
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<ErrorDetails>,
}

/// Error details that are specific to the instance
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ErrorDetails {
    /// Related error details
    pub details: String,
}

/// Status of an operation
///
/// [API Spec](https://www.rosetta-api.org/docs/models/OperationStatus.html)
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct OperationStatus {
    pub status: String,
    pub successful: bool,
}

/// UNUSED Represents a Peer, used for discovery
///
/// [API Spec](https://www.rosetta-api.org/docs/models/Peer.html)
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Peer {
    peer_id: String,
}

/// UNUSED Represents the current status of the node vs expected state
///
/// [API Spec](https://www.rosetta-api.org/docs/models/SyncStatus.html)
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SyncStatus {
    #[serde(skip_serializing_if = "Option::is_none")]
    current_index: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    target_index: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stage: Option<String>,
    synced: bool,
}

/// Version information for the current deployment to handle software version matching
///
/// [API Spec](https://www.rosetta-api.org/docs/models/Version.html)
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Version {
    /// Rosetta version, this should be hardcoded
    pub rosetta_version: String,
    /// Node version, this should come from the node
    pub node_version: String,
    /// Middleware version, this should be the version of this software
    pub middleware_version: String,
}

/// Represents the result of the balance retrieval
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BalanceResult {
    pub balance: Option<Amount>,
    /// Time at which the lockup expires and pending_inactive balance becomes inactive
    pub lockup_expiration: u64,
}

/// An internal enum to support Operation typing
///
/// NOTE: Order is important here for sorting later, this order must not change, and if there are new
/// types added, they should be added before Fee.  We sort the sub operations so that they have a
/// stable order for things like transfers.
#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub enum OperationType {
    // Create must always be first for ordering
    CreateAccount,
    // Withdraw must come before deposit
    Withdraw,
    Deposit,
    StakingReward,
    SetOperator,
    SetVoter,
    InitializeStakePool,
    ResetLockup,
    UnlockStake,
    UpdateCommission,
    WithdrawUndelegatedFunds,
    DistributeStakingRewards,
    AddDelegatedStake,
    UnlockDelegatedStake,
    // Fee must always be last for ordering
    Fee,
}

impl OperationType {
    const ADD_DELEGATED_STAKE: &'static str = "add_delegated_stake";
    const CREATE_ACCOUNT: &'static str = "create_account";
    const DEPOSIT: &'static str = "deposit";
    const DISTRIBUTE_STAKING_REWARDS: &'static str = "distribute_staking_rewards";
    const FEE: &'static str = "fee";
    const INITIALIZE_STAKE_POOL: &'static str = "initialize_stake_pool";
    const RESET_LOCKUP: &'static str = "reset_lockup";
    const SET_OPERATOR: &'static str = "set_operator";
    const SET_VOTER: &'static str = "set_voter";
    const STAKING_REWARD: &'static str = "staking_reward";
    const UNLOCK_DELEGATED_STAKE: &'static str = "unlock_delegated_stake";
    const UNLOCK_STAKE: &'static str = "unlock_stake";
    const UPDATE_COMMISSION: &'static str = "update_commission";
    const WITHDRAW: &'static str = "withdraw";
    const WITHDRAW_UNDELEGATED_FUNDS: &'static str = "withdraw_undelegated_funds";

    /// Returns all operations types, order doesn't matter.
    pub fn all() -> Vec<OperationType> {
        use OperationType::*;
        vec![
            CreateAccount,
            Withdraw,
            Deposit,
            Fee,
            SetOperator,
            SetVoter,
            StakingReward,
            InitializeStakePool,
            ResetLockup,
            UnlockStake,
            WithdrawUndelegatedFunds,
            DistributeStakingRewards,
            AddDelegatedStake,
            UnlockDelegatedStake,
        ]
    }
}

impl FromStr for OperationType {
    type Err = ApiError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Handles string to operation Rust typing
        match s.to_lowercase().trim() {
            Self::CREATE_ACCOUNT => Ok(OperationType::CreateAccount),
            Self::DEPOSIT => Ok(OperationType::Deposit),
            Self::WITHDRAW => Ok(OperationType::Withdraw),
            Self::FEE => Ok(OperationType::Fee),
            Self::STAKING_REWARD => Ok(OperationType::StakingReward),
            Self::SET_OPERATOR => Ok(OperationType::SetOperator),
            Self::SET_VOTER => Ok(OperationType::SetVoter),
            Self::INITIALIZE_STAKE_POOL => Ok(OperationType::InitializeStakePool),
            Self::RESET_LOCKUP => Ok(OperationType::ResetLockup),
            Self::UNLOCK_STAKE => Ok(OperationType::UnlockStake),
            Self::UPDATE_COMMISSION => Ok(OperationType::UpdateCommission),
            Self::DISTRIBUTE_STAKING_REWARDS => Ok(OperationType::DistributeStakingRewards),
            Self::ADD_DELEGATED_STAKE => Ok(OperationType::AddDelegatedStake),
            Self::UNLOCK_DELEGATED_STAKE => Ok(OperationType::UnlockDelegatedStake),
            Self::WITHDRAW_UNDELEGATED_FUNDS => Ok(OperationType::WithdrawUndelegatedFunds),
            _ => Err(ApiError::DeserializationFailed(Some(format!(
                "Invalid OperationType: {}",
                s
            )))),
        }
    }
}

impl Display for OperationType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        use OperationType::*;
        f.write_str(match self {
            CreateAccount => Self::CREATE_ACCOUNT,
            Deposit => Self::DEPOSIT,
            Withdraw => Self::WITHDRAW,
            StakingReward => Self::STAKING_REWARD,
            SetOperator => Self::SET_OPERATOR,
            SetVoter => Self::SET_VOTER,
            InitializeStakePool => Self::INITIALIZE_STAKE_POOL,
            ResetLockup => Self::RESET_LOCKUP,
            UnlockStake => Self::UNLOCK_STAKE,
            UpdateCommission => Self::UPDATE_COMMISSION,
            DistributeStakingRewards => Self::DISTRIBUTE_STAKING_REWARDS,
            AddDelegatedStake => Self::ADD_DELEGATED_STAKE,
            UnlockDelegatedStake => Self::UNLOCK_DELEGATED_STAKE,
            WithdrawUndelegatedFunds => Self::WITHDRAW_UNDELEGATED_FUNDS,
            Fee => Self::FEE,
        })
    }
}

/// An internal type to support typing of Operation statuses
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum OperationStatusType {
    /// Operation was part of a successfully committed transaction
    Success,
    /// Operation was not part of a successfully committed transaction
    Failure,
}

impl OperationStatusType {
    const FAILURE: &'static str = "failure";
    const SUCCESS: &'static str = "success";

    pub fn all() -> Vec<OperationStatusType> {
        vec![OperationStatusType::Success, OperationStatusType::Failure]
    }
}

impl From<OperationStatusType> for OperationStatus {
    fn from(status: OperationStatusType) -> Self {
        let successful = match status {
            OperationStatusType::Success => true,
            OperationStatusType::Failure => false,
        };

        OperationStatus {
            status: status.to_string(),
            successful,
        }
    }
}

impl TryFrom<OperationStatus> for OperationStatusType {
    type Error = ApiError;

    fn try_from(status: OperationStatus) -> Result<Self, Self::Error> {
        OperationStatusType::from_str(&status.status)
    }
}

impl FromStr for OperationStatusType {
    type Err = ApiError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().trim() {
            Self::SUCCESS => Ok(OperationStatusType::Success),
            Self::FAILURE => Ok(OperationStatusType::Failure),
            _ => Err(ApiError::DeserializationFailed(Some(format!(
                "Invalid OperationStatusType: {}",
                s
            )))),
        }
    }
}

impl Display for OperationStatusType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            OperationStatusType::Success => Self::SUCCESS,
            OperationStatusType::Failure => Self::FAILURE,
        })
    }
}

/// Retrieves stake balances for an owner with the associated pool
pub async fn get_stake_balances(
    rest_client: &aptos_rest_client::Client,
    owner_account: &AccountIdentifier,
    pool_address: AccountAddress,
    version: u64,
) -> ApiResult<Option<BalanceResult>> {
    const STAKE_POOL: &str = "0x1::stake::StakePool";

    // Retreive the pool resource
    if let Ok(response) = rest_client
        .get_account_resource_at_version_bcs::<StakePool>(pool_address, STAKE_POOL, version)
        .await
    {
        let stake_pool = response.into_inner();

        // Stake isn't allowed for base accounts
        if owner_account.is_base_account() {
            return Err(ApiError::InvalidInput(Some(
                "Stake pool not supported for base account".to_string(),
            )));
        }

        // If the operator address is different, skip
        if owner_account.is_operator_stake()
            && owner_account.operator_address()? != stake_pool.operator_address
        {
            return Err(ApiError::InvalidInput(Some(
                "Stake pool not for matching operator".to_string(),
            )));
        }

        // Any stake pools that match, retrieve that.
        let mut requested_balance: Option<String> = None;
        let lockup_expiration = stake_pool.locked_until_secs;
        let owner_address = owner_account.account_address()?;
        let operator_address = stake_pool.operator_address;

        let staking_contract_amounts_response = view::<Vec<u64>>(
            rest_client,
            version,
            AccountAddress::ONE,
            ident_str!(STAKING_CONTRACT_MODULE),
            ident_str!("staking_contract_amounts"),
            vec![],
            vec![
                bcs::to_bytes(&owner_address)?,
                bcs::to_bytes(&operator_address)?,
            ],
        )
        .await?;
        let total_active_stake = staking_contract_amounts_response[0];
        let accumulated_rewards = staking_contract_amounts_response[1];
        let commission_amount = staking_contract_amounts_response[2];

        // TODO: I think all of these are off, probably need to recalculate all of them
        // see the get_staking_contract_amounts_internal function in staking_contract.move for more
        // information on why commission is only subtracted from active and total stake
        if owner_account.is_active_stake() {
            // active stake is principal and rewards (including commission) so subtract the commission
            requested_balance = Some((total_active_stake - commission_amount).to_string());
        } else if owner_account.is_pending_active_stake() {
            // pending_active cannot have commission because it is new principal
            requested_balance = Some(stake_pool.pending_active.to_string());
        } else if owner_account.is_inactive_stake() {
            // inactive will not have commission because commission has already been extracted
            requested_balance = Some(stake_pool.inactive.to_string());
        } else if owner_account.is_pending_inactive_stake() {
            // pending_inactive will not have commission because commission has already been extracted
            requested_balance = Some(stake_pool.pending_inactive.to_string());
        } else if owner_account.is_total_stake() {
            // total stake includes commission since it includes active stake, which includes commission
            requested_balance =
                Some((stake_pool.get_total_staked_amount() - commission_amount).to_string());
        } else if owner_account.is_commission() {
            requested_balance = Some(commission_amount.to_string());
        } else if owner_account.is_rewards() {
            requested_balance = Some(accumulated_rewards.to_string());
        }

        if let Some(balance) = requested_balance {
            Ok(Some(BalanceResult {
                balance: Some(Amount {
                    value: balance,
                    currency: native_coin(),
                }),
                lockup_expiration,
            }))
        } else {
            Ok(None)
        }
    } else {
        Ok(None)
    }
}

/// Retrieve delegation stake balances for a given owner, pool, and version
pub async fn get_delegation_stake_balances(
    rest_client: &aptos_rest_client::Client,
    account_identifier: &AccountIdentifier,
    owner_address: AccountAddress,
    pool_address: AccountAddress,
    version: u64,
) -> ApiResult<Option<BalanceResult>> {
    // get requested_balance
    let balances_response = rest_client
        .view(
            &ViewRequest {
                function: DELEGATION_POOL_GET_STAKE_FUNCTION.clone(),
                type_arguments: vec![],
                arguments: vec![
                    serde_json::Value::String(pool_address.to_string()),
                    serde_json::Value::String(owner_address.to_string()),
                ],
            },
            Some(version),
        )
        .await?;

    let requested_balance =
        parse_requested_balance(account_identifier, balances_response.into_inner());

    // get lockup_secs
    let lockup_secs_response = rest_client
        .view(
            &ViewRequest {
                function: STAKE_GET_LOCKUP_SECS_FUNCTION.clone(),
                type_arguments: vec![],
                arguments: vec![serde_json::Value::String(pool_address.to_string())],
            },
            Some(version),
        )
        .await?;
    let lockup_expiration = parse_lockup_expiration(lockup_secs_response.into_inner());

    if let Some(balance) = requested_balance {
        Ok(Some(BalanceResult {
            balance: Some(Amount {
                value: balance,
                currency: native_coin(),
            }),
            lockup_expiration,
        }))
    } else {
        Err(ApiError::InternalError(Some(
            "Unable to construct BalanceResult instance".to_string(),
        )))
    }
}

fn parse_requested_balance(
    account_identifier: &AccountIdentifier,
    balances_result: Vec<serde_json::Value>,
) -> Option<String> {
    if account_identifier.is_delegator_active_stake() {
        return balances_result
            .first()
            .and_then(|v| v.as_str().map(|s| s.to_owned()));
    } else if account_identifier.is_delegator_inactive_stake() {
        return balances_result
            .get(1)
            .and_then(|v| v.as_str().map(|s| s.to_owned()));
    } else if account_identifier.is_delegator_pending_inactive_stake() {
        return balances_result
            .get(2)
            .and_then(|v| v.as_str().map(|s| s.to_owned()));
    } else if account_identifier.is_total_stake() {
        return Some(
            balances_result
                .iter()
                .map(|v| {
                    v.as_str()
                        .map(|s| s.to_owned())
                        .and_then(|s| s.parse::<u64>().ok())
                        .unwrap_or(0)
                })
                .sum::<u64>()
                .to_string(),
        );
    }

    None
}

fn parse_lockup_expiration(lockup_secs_result: Vec<serde_json::Value>) -> u64 {
    return lockup_secs_result
        .first()
        .and_then(|v| v.as_str().and_then(|s| s.parse::<u64>().ok()))
        .unwrap_or(0);
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::types::SubAccountIdentifier;

    #[test]
    fn test_parse_requested_balance() {
        let balances_result = vec![
            serde_json::Value::String("300".to_string()),
            serde_json::Value::String("200".to_string()),
            serde_json::Value::String("100".to_string()),
        ];

        // Total stake balance is sum of all 3
        assert_eq!(
            Some("600".to_string()),
            parse_requested_balance(
                &AccountIdentifier {
                    address: "0x123".to_string(),
                    sub_account: Some(SubAccountIdentifier::new_delegated_total_stake("0xabc")),
                },
                balances_result.clone()
            )
        );

        assert_eq!(
            Some("300".to_string()),
            parse_requested_balance(
                &AccountIdentifier {
                    address: "0x123".to_string(),
                    sub_account: Some(SubAccountIdentifier::new_delegated_active_stake("0xabc")),
                },
                balances_result.clone()
            )
        );

        assert_eq!(
            Some("200".to_string()),
            parse_requested_balance(
                &AccountIdentifier {
                    address: "0x123".to_string(),
                    sub_account: Some(SubAccountIdentifier::new_delegated_inactive_stake("0xabc")),
                },
                balances_result.clone()
            )
        );

        assert_eq!(
            Some("100".to_string()),
            parse_requested_balance(
                &AccountIdentifier {
                    address: "0x123".to_string(),
                    sub_account: Some(SubAccountIdentifier::new_delegated_pending_inactive_stake(
                        "0xabc"
                    )),
                },
                balances_result.clone()
            )
        );

        assert_eq!(
            None,
            parse_requested_balance(
                &AccountIdentifier {
                    address: "0x123".to_string(),
                    sub_account: Some(SubAccountIdentifier::new_active_stake()),
                },
                balances_result
            )
        );
    }

    #[test]
    fn test_parse_lockup_expiration() {
        let lockup_secs_result = vec![serde_json::Value::String("123456".to_string())];
        assert_eq!(123456, parse_lockup_expiration(lockup_secs_result));
    }
}
