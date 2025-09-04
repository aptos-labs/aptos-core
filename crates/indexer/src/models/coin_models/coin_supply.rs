// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

// This is required because a diesel macro makes clippy sad
#![allow(clippy::extra_unused_lifetimes)]
#![allow(clippy::unused_unit)]

use super::coin_infos::CoinInfoQuery;
use crate::schema::coin_supply;
use anyhow::Context;
use velor_api_types::WriteTableItem as APIWriteTableItem;
use bigdecimal::BigDecimal;
use field_count::FieldCount;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, FieldCount, Identifiable, Insertable, Serialize)]
#[diesel(primary_key(transaction_version, coin_type_hash))]
#[diesel(table_name = coin_supply)]
pub struct CoinSupply {
    pub transaction_version: i64,
    pub coin_type_hash: String,
    pub coin_type: String,
    pub supply: BigDecimal,
    pub transaction_timestamp: chrono::NaiveDateTime,
    pub transaction_epoch: i64,
}

impl CoinSupply {
    /// Currently only supports velor_coin. Aggregator table detail is in CoinInfo which for velor coin appears during genesis.
    /// We query for the aggregator table details (handle and key) once upon indexer initiation and use it to fetch supply.
    pub fn from_write_table_item(
        write_table_item: &APIWriteTableItem,
        maybe_velor_coin_info: &Option<CoinInfoQuery>,
        txn_version: i64,
        txn_timestamp: chrono::NaiveDateTime,
        txn_epoch: i64,
    ) -> anyhow::Result<Option<Self>> {
        if let Some(velor_coin_info) = maybe_velor_coin_info {
            // Return early if we don't have the velor aggregator table info
            if velor_coin_info.supply_aggregator_table_key.is_none()
                || velor_coin_info.supply_aggregator_table_handle.is_none()
            {
                return Ok(None);
            }
            if let Some(data) = &write_table_item.data {
                // Return early if not aggregator table type
                if !(data.key_type == "address" && data.value_type == "u128") {
                    return Ok(None);
                }
                // Return early if not aggregator table handle
                if &write_table_item.handle.to_string()
                    != velor_coin_info
                        .supply_aggregator_table_handle
                        .as_ref()
                        .unwrap()
                {
                    return Ok(None);
                }
                // Return early if not velor coin aggregator key
                let table_key = data
                    .key
                    .as_str()
                    .context(format!("key is not a string: {:?}", data.key))?;
                if table_key
                    != velor_coin_info
                        .supply_aggregator_table_key
                        .as_ref()
                        .unwrap()
                {
                    return Ok(None);
                }
                // Everything matches. Get the coin supply
                let supply = data
                    .value
                    .as_str()
                    .map(|s| s.parse::<BigDecimal>())
                    .context(format!(
                        "value is not a string: {:?}, table_item {:?}, version {}",
                        data.value, write_table_item, txn_version
                    ))?
                    .context(format!(
                        "cannot parse string as u128: {:?}, version {}",
                        data.value, txn_version
                    ))?;
                return Ok(Some(Self {
                    transaction_version: txn_version,
                    coin_type_hash: velor_coin_info.coin_type_hash.clone(),
                    coin_type: velor_coin_info.coin_type.clone(),
                    supply,
                    transaction_timestamp: txn_timestamp,
                    transaction_epoch: txn_epoch,
                }));
            }
        }
        Ok(None)
    }
}
