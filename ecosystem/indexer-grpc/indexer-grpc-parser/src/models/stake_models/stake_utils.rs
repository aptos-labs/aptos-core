// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    models::{default_models::move_resources::MoveResource, token_models::token_utils::Table},
    utils::util::{deserialize_from_string, standardize_address},
};
use anyhow::{Context, Result};
use aptos_protos::transaction::v1::WriteResource;
use bigdecimal::BigDecimal;
use serde::{Deserialize, Serialize};

const STAKE_ADDR: &str = "0x0000000000000000000000000000000000000000000000000000000000000001";
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StakePoolResource {
    delegated_voter: String,
    operator_address: String,
}

impl StakePoolResource {
    pub fn get_delegated_voter(&self) -> String {
        standardize_address(&self.delegated_voter)
    }

    pub fn get_operator_address(&self) -> String {
        standardize_address(&self.operator_address)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DelegationPoolResource {
    pub active_shares: PoolResource,
    pub inactive_shares: Table,
    #[serde(deserialize_with = "deserialize_from_string")]
    pub operator_commission_percentage: BigDecimal,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PoolResource {
    pub shares: SharesInnerResource,
    #[serde(deserialize_with = "deserialize_from_string")]
    pub total_coins: BigDecimal,
    #[serde(deserialize_with = "deserialize_from_string")]
    pub total_shares: BigDecimal,
    #[serde(deserialize_with = "deserialize_from_string")]
    pub scaling_factor: BigDecimal,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SharesInnerResource {
    pub inner: Table,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GovernanceVoteEvent {
    #[serde(deserialize_with = "deserialize_from_string")]
    pub proposal_id: u64,
    pub voter: String,
    pub stake_pool: String,
    #[serde(deserialize_with = "deserialize_from_string")]
    pub num_votes: BigDecimal,
    pub should_pass: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DistributeRewardsEvent {
    pub pool_address: String,
    #[serde(deserialize_with = "deserialize_from_string")]
    pub rewards_amount: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AddStakeEvent {
    #[serde(deserialize_with = "deserialize_from_string")]
    pub amount_added: u64,
    pub delegator_address: String,
    pub pool_address: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UnlockStakeEvent {
    #[serde(deserialize_with = "deserialize_from_string")]
    pub amount_unlocked: u64,
    pub delegator_address: String,
    pub pool_address: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WithdrawStakeEvent {
    #[serde(deserialize_with = "deserialize_from_string")]
    pub amount_withdrawn: u64,
    pub delegator_address: String,
    pub pool_address: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ReactivateStakeEvent {
    #[serde(deserialize_with = "deserialize_from_string")]
    pub amount_reactivated: u64,
    pub delegator_address: String,
    pub pool_address: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum StakeTableItem {
    Pool(PoolResource),
}

impl StakeTableItem {
    pub fn from_table_item_type(
        data_type: &str,
        data: &str,
        txn_version: i64,
    ) -> Result<Option<Self>> {
        match data_type {
            "0x1::pool_u64_unbound::Pool" => {
                serde_json::from_str(data).map(|inner| Some(StakeTableItem::Pool(inner)))
            },
            _ => Ok(None),
        }
        .context(format!(
            "version {} failed! failed to parse type {}, data {:?}",
            txn_version, data_type, data
        ))
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum StakeResource {
    StakePool(StakePoolResource),
    DelegationPool(DelegationPoolResource),
}

impl StakeResource {
    fn is_resource_supported(data_type: &str) -> bool {
        [
            format!("{}::stake::StakePool", STAKE_ADDR),
            format!("{}::delegation_pool::DelegationPool", STAKE_ADDR),
        ]
        .contains(&data_type.to_string())
    }

    fn from_resource(data_type: &str, data: &serde_json::Value, txn_version: i64) -> Result<Self> {
        match data_type {
            x if x == format!("{}::stake::StakePool", STAKE_ADDR) => {
                serde_json::from_value(data.clone())
                    .map(|inner| Some(StakeResource::StakePool(inner)))
            },
            x if x == format!("{}::delegation_pool::DelegationPool", STAKE_ADDR) => {
                serde_json::from_value(data.clone())
                    .map(|inner| Some(StakeResource::DelegationPool(inner)))
            },
            _ => Ok(None),
        }
        .context(format!(
            "version {} failed! failed to parse type {}, data {:?}",
            txn_version, data_type, data
        ))?
        .context(format!(
            "Resource unsupported! Call is_resource_supported first. version {} type {}",
            txn_version, data_type
        ))
    }

    pub fn from_write_resource(
        write_resource: &WriteResource,
        txn_version: i64,
    ) -> Result<Option<Self>> {
        let type_str = MoveResource::get_outer_type_from_resource(write_resource);
        if !Self::is_resource_supported(type_str.as_str()) {
            return Ok(None);
        }
        let resource = MoveResource::from_write_resource(
            write_resource,
            0, // Placeholder, this isn't used anyway
            txn_version,
            0, // Placeholder, this isn't used anyway
        );
        Ok(Some(Self::from_resource(
            &type_str,
            resource.data.as_ref().unwrap(),
            txn_version,
        )?))
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum StakeEvent {
    GovernanceVoteEvent(GovernanceVoteEvent),
    DistributeRewardsEvent(DistributeRewardsEvent),
    AddStakeEvent(AddStakeEvent),
    UnlockStakeEvent(UnlockStakeEvent),
    WithdrawStakeEvent(WithdrawStakeEvent),
    ReactivateStakeEvent(ReactivateStakeEvent),
}

impl StakeEvent {
    pub fn from_event(data_type: &str, data: &str, txn_version: i64) -> Result<Option<Self>> {
        match data_type {
            "0x1::aptos_governance::VoteEvent" => {
                serde_json::from_str(data).map(|inner| Some(StakeEvent::GovernanceVoteEvent(inner)))
            },
            "0x1::stake::DistributeRewardsEvent" => serde_json::from_str(data)
                .map(|inner| Some(StakeEvent::DistributeRewardsEvent(inner))),
            "0x1::delegation_pool::AddStakeEvent" => {
                serde_json::from_str(data).map(|inner| Some(StakeEvent::AddStakeEvent(inner)))
            },
            "0x1::delegation_pool::UnlockStakeEvent" => {
                serde_json::from_str(data).map(|inner| Some(StakeEvent::UnlockStakeEvent(inner)))
            },
            "0x1::delegation_pool::WithdrawStakeEvent" => {
                serde_json::from_str(data).map(|inner| Some(StakeEvent::WithdrawStakeEvent(inner)))
            },
            "0x1::delegation_pool::ReactivateStakeEvent" => serde_json::from_str(data)
                .map(|inner| Some(StakeEvent::ReactivateStakeEvent(inner))),
            _ => Ok(None),
        }
        .context(format!(
            "version {} failed! failed to parse type {}, data {:?}",
            txn_version, data_type, data
        ))
    }
}
