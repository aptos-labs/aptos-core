// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

// This is required because a diesel macro makes clippy sad
#![allow(clippy::extra_unused_lifetimes)]

use crate::{
    models::move_resources::MoveResource,
    util::{hash_str, truncate_str},
};
use anyhow::{Context, Result};
use velor_api_types::{deserialize_from_string, MoveType, WriteResource};
use bigdecimal::BigDecimal;
use serde::{Deserialize, Serialize};

const COIN_TYPE_HASH_LENGTH: usize = 5000;
/**
 * This file defines deserialized coin types as defined in our 0x1 contracts.
 */

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CoinInfoResource {
    name: String,
    symbol: String,
    pub decimals: i32,
    pub supply: OptionalAggregatorWrapperResource,
}

impl CoinInfoResource {
    pub fn get_name_trunc(&self) -> String {
        truncate_str(&self.name, 32)
    }

    pub fn get_symbol_trunc(&self) -> String {
        truncate_str(&self.symbol, 10)
    }

    /// Getting the table item location of the supply aggregator
    pub fn get_aggregator_metadata(&self) -> Option<AggregatorResource> {
        if let Some(inner) = self.supply.vec.first() {
            inner.aggregator.get_aggregator_metadata()
        } else {
            None
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct OptionalAggregatorWrapperResource {
    pub vec: Vec<OptionalAggregatorResource>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct OptionalAggregatorResource {
    pub aggregator: AggregatorWrapperResource,
    pub integer: IntegerWrapperResource,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AggregatorWrapperResource {
    pub vec: Vec<AggregatorResource>,
}

impl AggregatorWrapperResource {
    /// In case we do want to track supply
    pub fn get_aggregator_metadata(&self) -> Option<AggregatorResource> {
        self.vec.first().cloned()
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct IntegerWrapperResource {
    pub vec: Vec<IntegerResource>,
}

impl IntegerWrapperResource {
    /// In case we do want to track supply
    pub fn get_supply(&self) -> Option<BigDecimal> {
        self.vec.first().map(|inner| inner.value.clone())
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AggregatorResource {
    pub handle: String,
    pub key: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct IntegerResource {
    #[serde(deserialize_with = "deserialize_from_string")]
    pub value: BigDecimal,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CoinStoreResource {
    pub coin: Coin,
    pub deposit_events: DepositEventResource,
    pub withdraw_events: WithdrawEventResource,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Coin {
    #[serde(deserialize_with = "deserialize_from_string")]
    pub value: BigDecimal,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DepositEventResource {
    pub guid: EventGuidResourceWrapper,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WithdrawEventResource {
    pub guid: EventGuidResourceWrapper,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EventGuidResourceWrapper {
    pub id: EventGuidResource,
}

#[derive(Serialize, Deserialize, Debug, Clone, Hash, Eq, PartialEq)]
pub struct EventGuidResource {
    pub addr: String,
    #[serde(deserialize_with = "deserialize_from_string")]
    pub creation_num: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WithdrawCoinEvent {
    #[serde(deserialize_with = "deserialize_from_string")]
    pub amount: BigDecimal,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DepositCoinEvent {
    #[serde(deserialize_with = "deserialize_from_string")]
    pub amount: BigDecimal,
}

pub struct CoinInfoType {
    coin_type: String,
    pub creator_address: String,
}

impl CoinInfoType {
    pub fn from_move_type(move_type: &MoveType, txn_version: i64) -> anyhow::Result<Self> {
        let coin_type = move_type.to_string();
        let (address, _, _) = if let MoveType::Struct(inner) = move_type {
            (
                inner.address.to_string(),
                inner.module.to_string(),
                inner.name.to_string(),
            )
        } else {
            Err(anyhow::anyhow!(
                "MoveType is not a struct: {:?}, version: {}",
                move_type,
                txn_version
            ))?
        };
        Ok(Self {
            coin_type,
            creator_address: address,
        })
    }

    pub fn to_hash(&self) -> String {
        hash_str(&self.coin_type.to_string())
    }

    pub fn get_coin_type_trunc(&self) -> String {
        truncate_str(&self.coin_type, COIN_TYPE_HASH_LENGTH)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum CoinResource {
    CoinInfoResource(CoinInfoResource),
    CoinStoreResource(CoinStoreResource),
}

impl CoinResource {
    pub fn is_resource_supported(data_type: &str) -> bool {
        matches!(data_type, "0x1::coin::CoinInfo" | "0x1::coin::CoinStore")
    }

    pub fn from_resource(
        data_type: &str,
        data: &serde_json::Value,
        txn_version: i64,
    ) -> Result<CoinResource> {
        match data_type {
            "0x1::coin::CoinInfo" => serde_json::from_value(data.clone())
                .map(|inner| Some(CoinResource::CoinInfoResource(inner))),
            "0x1::coin::CoinStore" => serde_json::from_value(data.clone())
                .map(|inner| Some(CoinResource::CoinStoreResource(inner))),
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
    ) -> Result<Option<CoinResource>> {
        let type_str = format!(
            "{}::{}::{}",
            write_resource.data.typ.address,
            write_resource.data.typ.module,
            write_resource.data.typ.name
        );
        if !CoinResource::is_resource_supported(type_str.as_str()) {
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
pub enum CoinEvent {
    WithdrawCoinEvent(WithdrawCoinEvent),
    DepositCoinEvent(DepositCoinEvent),
}

impl CoinEvent {
    pub fn from_event(
        data_type: &str,
        data: &serde_json::Value,
        txn_version: i64,
    ) -> Result<Option<CoinEvent>> {
        match data_type {
            "0x1::coin::WithdrawEvent" => serde_json::from_value(data.clone())
                .map(|inner| Some(CoinEvent::WithdrawCoinEvent(inner))),
            "0x1::coin::DepositEvent" => serde_json::from_value(data.clone())
                .map(|inner| Some(CoinEvent::DepositCoinEvent(inner))),
            _ => Ok(None),
        }
        .context(format!(
            "version {} failed! failed to parse type {}, data {:?}",
            txn_version, data_type, data
        ))
    }
}
