// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::models::move_resources::MoveResource;
use anyhow::{Context, Result};
use aptos_api_types::{deserialize_from_string, WriteResource};
use bigdecimal::BigDecimal;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StakePoolResource {
    pub delegated_voter: String,
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
pub enum StakeResource {
    StakePool(StakePoolResource),
}

impl StakeResource {
    fn is_resource_supported(data_type: &str) -> bool {
        matches!(data_type, "0x1::stake::StakePool")
    }

    fn from_resource(data_type: &str, data: &serde_json::Value, txn_version: i64) -> Result<Self> {
        match data_type {
            "0x1::stake::StakePool" => serde_json::from_value(data.clone())
                .map(|inner| Some(StakeResource::StakePool(inner))),
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
        let type_str = format!(
            "{}::{}::{}",
            write_resource.data.typ.address,
            write_resource.data.typ.module,
            write_resource.data.typ.name
        );
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
}

impl StakeEvent {
    pub fn from_event(
        data_type: &str,
        data: &serde_json::Value,
        txn_version: i64,
    ) -> Result<Option<Self>> {
        match data_type {
            "0x1::aptos_governance::VoteEvent" => serde_json::from_value(data.clone())
                .map(|inner| Some(StakeEvent::GovernanceVoteEvent(inner))),
            _ => Ok(None),
        }
        .context(format!(
            "version {} failed! failed to parse type {}, data {:?}",
            txn_version, data_type, data
        ))
    }
}
