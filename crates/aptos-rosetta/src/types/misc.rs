// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    common::native_coin,
    error::ApiError,
    types::{AccountIdentifier, Amount},
    AccountAddress, ApiResult,
};
use aptos_types::stake_pool::StakePool;
use serde::{Deserialize, Serialize};
use std::{
    convert::TryFrom,
    fmt::{Display, Formatter},
    str::FromStr,
};

/// Errors that can be returned by the API
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

/// Represents a Peer, used for discovery
///
/// [API Spec](https://www.rosetta-api.org/docs/models/Peer.html)
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Peer {
    peer_id: String,
}

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
    WithdrawUndelegated,
    DistributeStakingRewards,
    // Fee must always be last for ordering
    Fee,
}

impl OperationType {
    const CREATE_ACCOUNT: &'static str = "create_account";
    const DEPOSIT: &'static str = "deposit";
    const DISTRIBUTE_STAKING_REWARDS: &'static str = "distribute_staking_rewards";
    const FEE: &'static str = "fee";
    const INITIALIZE_STAKE_POOL: &'static str = "initialize_stake_pool";
    const RESET_LOCKUP: &'static str = "reset_lockup";
    const SET_OPERATOR: &'static str = "set_operator";
    const SET_VOTER: &'static str = "set_voter";
    const STAKING_REWARD: &'static str = "staking_reward";
    const UNLOCK_STAKE: &'static str = "unlock_stake";
    const WITHDRAW: &'static str = "withdraw";
    const WITHDRAW_UNDELEGATED: &'static str = "withdraw_undelegated";

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
            WithdrawUndelegated,
            DistributeStakingRewards,
        ]
    }
}

impl FromStr for OperationType {
    type Err = ApiError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
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
            Self::WITHDRAW_UNDELEGATED => Ok(OperationType::WithdrawUndelegated),
            Self::DISTRIBUTE_STAKING_REWARDS => Ok(OperationType::DistributeStakingRewards),
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
            WithdrawUndelegated => Self::WITHDRAW_UNDELEGATED,
            DistributeStakingRewards => Self::DISTRIBUTE_STAKING_REWARDS,
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

pub async fn get_stake_balances(
    rest_client: &aptos_rest_client::Client,
    owner_account: &AccountIdentifier,
    pool_address: AccountAddress,
    version: u64,
) -> ApiResult<Option<BalanceResult>> {
    const STAKE_POOL: &str = "0x1::stake::StakePool";
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

        if owner_account.is_active_stake() {
            requested_balance = Some(stake_pool.active.to_string());
        } else if owner_account.is_pending_active_stake() {
            requested_balance = Some(stake_pool.pending_active.to_string());
        } else if owner_account.is_inactive_stake() {
            requested_balance = Some(stake_pool.inactive.to_string());
        } else if owner_account.is_pending_inactive_stake() {
            requested_balance = Some(stake_pool.pending_inactive.to_string());
        } else if owner_account.is_total_stake() {
            requested_balance = Some(stake_pool.get_total_staked_amount().to_string());
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
