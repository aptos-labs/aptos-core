// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

// This is required because a diesel macro makes clippy sad
#![allow(clippy::extra_unused_lifetimes)]
#![allow(clippy::unused_unit)]

use std::collections::HashMap;

use super::coin_utils::{CoinInfoType, CoinResource};
use crate::schema::coin_infos;
use anyhow::Context;
use aptos_api_types::{WriteResource as APIWriteResource, WriteTableItem as APIWriteTableItem};
use bigdecimal::BigDecimal;
use field_count::FieldCount;
use serde::{Deserialize, Serialize};

pub type TableHandle = String;
pub type TableKey = String;
pub type Supply = BigDecimal;
pub type CoinSupplyLookup = HashMap<(TableHandle, TableKey), Supply>;

#[derive(Debug, Deserialize, FieldCount, Identifiable, Insertable, Serialize)]
#[diesel(primary_key(coin_type_hash))]
#[diesel(table_name = coin_infos)]
pub struct CoinInfo {
    pub coin_type_hash: String,
    pub coin_type: String,
    pub transaction_version_created: i64,
    pub creator_address: String,
    pub name: String,
    pub symbol: String,
    pub decimals: i32,
    pub transaction_created_timestamp: chrono::NaiveDateTime,
}

impl CoinInfo {
    /// We can find coin info from resources. If the coin info appears multiple times we will only keep the first transaction because it can't be modified.
    pub fn from_write_resource(
        write_resource: &APIWriteResource,
        txn_version: i64,
        txn_timestamp: chrono::NaiveDateTime,
    ) -> anyhow::Result<Option<Self>> {
        match &CoinResource::from_write_resource(write_resource, txn_version)? {
            Some(CoinResource::CoinInfoResource(inner)) => {
                let coin_info_type = &CoinInfoType::from_move_type(
                    &write_resource.data.typ.generic_type_params[0],
                    txn_version,
                )?;

                Ok(Some(Self {
                    coin_type_hash: coin_info_type.to_hash(),
                    coin_type: coin_info_type.get_coin_type_trunc(),
                    transaction_version_created: txn_version,
                    creator_address: coin_info_type.creator_address.clone(),
                    name: inner.get_name_trunc(),
                    symbol: inner.get_symbol_trunc(),
                    decimals: inner.decimals,
                    transaction_created_timestamp: txn_timestamp,
                }))
            }
            _ => Ok(None),
        }
    }

    pub fn get_aggregator_supply_lookup(
        table_item: &APIWriteTableItem,
    ) -> anyhow::Result<CoinSupplyLookup> {
        if let Some(data) = &table_item.data {
            if data.key_type == "address" && data.value_type == "u128" {
                let value_str = data
                    .value
                    .as_str()
                    .map(|s| s.parse::<BigDecimal>())
                    .context(format!(
                        "value is not a string: {:?}, table_item {:?}",
                        data.value, table_item
                    ))?
                    .context(format!("cannot parse string as u128: {:?}", data.value))?;
                return Ok(HashMap::from([(
                    (
                        table_item.handle.to_string(),
                        data.key
                            .as_str()
                            .context(format!("key is not a string: {:?}", data.key))?
                            .to_string(),
                    ),
                    value_str,
                )]));
            }
        }
        Ok(HashMap::new())
    }
}
