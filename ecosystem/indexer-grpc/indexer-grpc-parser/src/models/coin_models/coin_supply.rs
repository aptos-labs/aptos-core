// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This is required because a diesel macro makes clippy sad
#![allow(clippy::extra_unused_lifetimes)]
#![allow(clippy::unused_unit)]

use super::coin_infos::CoinInfoQuery;
use crate::{models::default_models::move_tables::TableItem, schema::coin_supply};
use anyhow::Context;
use aptos_protos::transaction::testing1::v1::WriteTableItem;
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
    /// Currently only supports aptos_coin. Aggregator table detail is in CoinInfo which for aptos coin appears during genesis.
    /// We query for the aggregator table details (handle and key) once upon indexer initiation and use it to fetch supply.
    pub fn from_write_table_item(
        write_table_item: &WriteTableItem,
        maybe_aptos_coin_info: &Option<CoinInfoQuery>,
        txn_version: i64,
        txn_timestamp: chrono::NaiveDateTime,
        txn_epoch: i64,
    ) -> anyhow::Result<Option<Self>> {
        if let Some(aptos_coin_info) = maybe_aptos_coin_info {
            // Return early if we don't have the aptos aggregator table info
            if aptos_coin_info.supply_aggregator_table_key.is_none()
                || aptos_coin_info.supply_aggregator_table_handle.is_none()
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
                    != aptos_coin_info
                        .supply_aggregator_table_handle
                        .as_ref()
                        .unwrap()
                {
                    return Ok(None);
                }

                // Convert to TableItem model. Some fields are just placeholders
                let (table_item_model, _) =
                    TableItem::from_write_table_item(write_table_item, 0, txn_version, 0);

                // Return early if not aptos coin aggregator key
                let table_key = &table_item_model.decoded_key.as_str().unwrap();
                if table_key
                    != aptos_coin_info
                        .supply_aggregator_table_key
                        .as_ref()
                        .unwrap()
                {
                    return Ok(None);
                }
                // Everything matches. Get the coin supply
                let supply = table_item_model
                    .decoded_value
                    .as_ref()
                    .unwrap()
                    .as_str()
                    .unwrap()
                    .parse::<BigDecimal>()
                    .context(format!(
                        "cannot parse string as u128: {:?}, version {}",
                        table_item_model.decoded_value.as_ref(),
                        txn_version
                    ))?;
                return Ok(Some(Self {
                    transaction_version: txn_version,
                    coin_type_hash: aptos_coin_info.coin_type_hash.clone(),
                    coin_type: aptos_coin_info.coin_type.clone(),
                    supply,
                    transaction_timestamp: txn_timestamp,
                    transaction_epoch: txn_epoch,
                }));
            }
        }
        Ok(None)
    }
}
