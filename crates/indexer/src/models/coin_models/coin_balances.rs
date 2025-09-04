// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

// This is required because a diesel macro makes clippy sad
#![allow(clippy::extra_unused_lifetimes)]
#![allow(clippy::unused_unit)]

use super::{
    coin_activities::EventToCoinType,
    coin_utils::{CoinInfoType, CoinResource},
};
use crate::{
    schema::{coin_balances, current_coin_balances},
    util::standardize_address,
};
use velor_api_types::WriteResource as APIWriteResource;
use bigdecimal::BigDecimal;
use field_count::FieldCount;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Deserialize, FieldCount, Identifiable, Insertable, Serialize)]
#[diesel(primary_key(transaction_version, owner_address, coin_type))]
#[diesel(table_name = coin_balances)]
pub struct CoinBalance {
    pub transaction_version: i64,
    pub owner_address: String,
    pub coin_type_hash: String,
    pub coin_type: String,
    pub amount: BigDecimal,
    pub transaction_timestamp: chrono::NaiveDateTime,
}

#[derive(Debug, Deserialize, FieldCount, Identifiable, Insertable, Serialize)]
#[diesel(primary_key(owner_address, coin_type))]
#[diesel(table_name = current_coin_balances)]
pub struct CurrentCoinBalance {
    pub owner_address: String,
    pub coin_type_hash: String,
    pub coin_type: String,
    pub amount: BigDecimal,
    pub last_transaction_version: i64,
    pub last_transaction_timestamp: chrono::NaiveDateTime,
}

impl CoinBalance {
    /// We can find coin info from resources. If the coin info appears multiple times we will only keep the first transaction because it can't be modified.
    pub fn from_write_resource(
        write_resource: &APIWriteResource,
        txn_version: i64,
        txn_timestamp: chrono::NaiveDateTime,
    ) -> anyhow::Result<Option<(Self, CurrentCoinBalance, EventToCoinType)>> {
        match &CoinResource::from_write_resource(write_resource, txn_version)? {
            Some(CoinResource::CoinStoreResource(inner)) => {
                let coin_info_type = &CoinInfoType::from_move_type(
                    &write_resource.data.typ.generic_type_params[0],
                    txn_version,
                )?;
                let owner_address = standardize_address(&write_resource.address.to_string());
                let coin_balance = Self {
                    transaction_version: txn_version,
                    owner_address: owner_address.clone(),
                    coin_type_hash: coin_info_type.to_hash(),
                    coin_type: coin_info_type.get_coin_type_trunc(),
                    amount: inner.coin.value.clone(),
                    transaction_timestamp: txn_timestamp,
                };
                let current_coin_balance = CurrentCoinBalance {
                    owner_address,
                    coin_type_hash: coin_info_type.to_hash(),
                    coin_type: coin_info_type.get_coin_type_trunc(),
                    amount: inner.coin.value.clone(),
                    last_transaction_version: txn_version,
                    last_transaction_timestamp: txn_timestamp,
                };
                let event_to_coin_mapping: EventToCoinType = HashMap::from([
                    (
                        (inner.withdraw_events.guid.id.clone()),
                        coin_balance.coin_type.clone(),
                    ),
                    (
                        (inner.deposit_events.guid.id.clone()),
                        coin_balance.coin_type.clone(),
                    ),
                ]);

                Ok(Some((
                    coin_balance,
                    current_coin_balance,
                    event_to_coin_mapping,
                )))
            },
            _ => Ok(None),
        }
    }
}
