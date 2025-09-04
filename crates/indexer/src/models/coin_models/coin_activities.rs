// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

// This is required because a diesel macro makes clippy sad
#![allow(clippy::extra_unused_lifetimes)]
#![allow(clippy::unused_unit)]

use super::{
    coin_balances::{CoinBalance, CurrentCoinBalance},
    coin_infos::{CoinInfo, CoinInfoQuery},
    coin_supply::CoinSupply,
    coin_utils::{CoinEvent, EventGuidResource},
};
use crate::{
    schema::coin_activities,
    util::{parse_timestamp, standardize_address, truncate_str},
};
use velor_api_types::{
    Event as APIEvent, Transaction as APITransaction, TransactionInfo as APITransactionInfo,
    TransactionPayload, UserTransactionRequest, WriteSetChange as APIWriteSetChange,
};
use velor_types::{VelorCoinType, CoinType as CoinTypeTrait};
use bigdecimal::BigDecimal;
use field_count::FieldCount;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

const GAS_FEE_EVENT: &str = "0x1::velor_coin::GasFeeEvent";
// We will never have a negative number on chain so this will avoid collision in postgres
const BURN_GAS_EVENT_CREATION_NUM: i64 = -1;
const BURN_GAS_EVENT_INDEX: i64 = -1;
pub const MAX_ENTRY_FUNCTION_LENGTH: usize = 100;

type OwnerAddress = String;
type CoinType = String;
// Primary key of the current_coin_balances table, i.e. (owner_address, coin_type)
pub type CurrentCoinBalancePK = (OwnerAddress, CoinType);
pub type EventToCoinType = HashMap<EventGuidResource, CoinType>;

#[derive(Debug, Deserialize, FieldCount, Identifiable, Insertable, Serialize)]
#[diesel(primary_key(
    transaction_version,
    event_account_address,
    event_creation_number,
    event_sequence_number
))]
#[diesel(table_name = coin_activities)]
pub struct CoinActivity {
    pub transaction_version: i64,
    pub event_account_address: String,
    pub event_creation_number: i64,
    pub event_sequence_number: i64,
    pub owner_address: String,
    pub coin_type: String,
    pub amount: BigDecimal,
    pub activity_type: String,
    pub is_gas_fee: bool,
    pub is_transaction_success: bool,
    pub entry_function_id_str: Option<String>,
    pub block_height: i64,
    pub transaction_timestamp: chrono::NaiveDateTime,
    pub event_index: Option<i64>,
}

impl CoinActivity {
    /// There are different objects containing different information about balances and coins.
    /// Events: Withdraw and Deposit event containing amounts. There is no coin type so we need to get that from Resources. (from event guid)
    /// CoinInfo Resource: Contains name, symbol, decimals and supply. (if supply is aggregator, however, actual supply amount will live in a separate table)
    /// CoinStore Resource: Contains owner address and coin type information used to complete events
    /// Aggregator Table Item: Contains current supply of a coin
    /// Note, we're not currently tracking supply
    pub fn from_transaction(
        transaction: &APITransaction,
        maybe_velor_coin_info: &Option<CoinInfoQuery>,
    ) -> (
        Vec<Self>,
        Vec<CoinBalance>,
        HashMap<CoinType, CoinInfo>,
        HashMap<CurrentCoinBalancePK, CurrentCoinBalance>,
        Vec<CoinSupply>,
    ) {
        let mut coin_activities = Vec::new();
        let mut coin_balances = Vec::new();
        let mut coin_infos: HashMap<CoinType, CoinInfo> = HashMap::new();
        let mut current_coin_balances: HashMap<CurrentCoinBalancePK, CurrentCoinBalance> =
            HashMap::new();
        let mut all_event_to_coin_type: EventToCoinType = HashMap::new();
        let mut all_coin_supply = Vec::new();

        #[allow(deprecated)]
        let (txn_info, writesets, events, maybe_user_request, txn_timestamp) = match &transaction {
            APITransaction::GenesisTransaction(inner) => (
                &inner.info,
                &inner.info.changes,
                &inner.events,
                None,
                chrono::NaiveDateTime::from_timestamp_opt(0, 0).unwrap(),
            ),
            APITransaction::UserTransaction(inner) => (
                &inner.info,
                &inner.info.changes,
                &inner.events,
                Some(&inner.request),
                parse_timestamp(inner.timestamp.0, inner.info.version.0 as i64),
            ),
            _ => return Default::default(),
        };

        // Get coin info, then coin balances. We can leverage coin balances to get the metadata required for events
        let txn_version = txn_info.version.0 as i64;
        let txn_epoch = txn_info.epoch.unwrap().0 as i64;
        let mut entry_function_id_str = None;
        if let Some(user_request) = maybe_user_request {
            entry_function_id_str = match &user_request.payload {
                TransactionPayload::EntryFunctionPayload(payload) => Some(truncate_str(
                    &payload.function.to_string(),
                    MAX_ENTRY_FUNCTION_LENGTH,
                )),
                _ => None,
            };
            coin_activities.push(Self::get_gas_event(
                txn_info,
                user_request,
                &entry_function_id_str,
                txn_timestamp,
            ));
        }

        for wsc in writesets {
            let (maybe_coin_info, maybe_coin_balance_data) =
                if let APIWriteSetChange::WriteResource(write_resource) = wsc {
                    (
                        CoinInfo::from_write_resource(write_resource, txn_version, txn_timestamp)
                            .unwrap(),
                        CoinBalance::from_write_resource(
                            write_resource,
                            txn_version,
                            txn_timestamp,
                        )
                        .unwrap(),
                    )
                } else {
                    (None, None)
                };

            let maybe_coin_supply = if let APIWriteSetChange::WriteTableItem(table_item) = &wsc {
                CoinSupply::from_write_table_item(
                    table_item,
                    maybe_velor_coin_info,
                    txn_version,
                    txn_timestamp,
                    txn_epoch,
                )
                .unwrap()
            } else {
                None
            };

            if let Some(coin_info) = maybe_coin_info {
                coin_infos.insert(coin_info.coin_type.clone(), coin_info);
            }
            if let Some((coin_balance, current_coin_balance, event_to_coin_type)) =
                maybe_coin_balance_data
            {
                current_coin_balances.insert(
                    (
                        coin_balance.owner_address.clone(),
                        coin_balance.coin_type.clone(),
                    ),
                    current_coin_balance,
                );
                coin_balances.push(coin_balance);
                all_event_to_coin_type.extend(event_to_coin_type);
            }
            if let Some(coin_supply) = maybe_coin_supply {
                all_coin_supply.push(coin_supply);
            }
        }
        for (index, event) in events.iter().enumerate() {
            let event_type = event.typ.to_string();
            if let Some(parsed_event) =
                CoinEvent::from_event(event_type.as_str(), &event.data, txn_version).unwrap()
            {
                coin_activities.push(Self::from_parsed_event(
                    &event_type,
                    event,
                    &parsed_event,
                    txn_version,
                    &all_event_to_coin_type,
                    txn_info.block_height.unwrap().0 as i64,
                    &entry_function_id_str,
                    txn_timestamp,
                    index as i64,
                ));
            };
        }
        (
            coin_activities,
            coin_balances,
            coin_infos,
            current_coin_balances,
            all_coin_supply,
        )
    }

    fn from_parsed_event(
        event_type: &str,
        event: &APIEvent,
        coin_event: &CoinEvent,
        txn_version: i64,
        event_to_coin_type: &EventToCoinType,
        block_height: i64,
        entry_function_id_str: &Option<String>,
        transaction_timestamp: chrono::NaiveDateTime,
        event_index: i64,
    ) -> Self {
        let amount = match coin_event {
            CoinEvent::WithdrawCoinEvent(inner) => inner.amount.clone(),
            CoinEvent::DepositCoinEvent(inner) => inner.amount.clone(),
        };
        let event_move_guid = EventGuidResource {
            addr: event.guid.account_address.to_string(),
            creation_num: event.guid.creation_number.0 as i64,
        };
        let coin_type =
            event_to_coin_type
                .get(&event_move_guid)
                .unwrap_or_else(|| {
                    panic!(
                        "Could not find event in resources (CoinStore), version: {}, event guid: {:?}, mapping: {:?}",
                        txn_version, event_move_guid, event_to_coin_type
                    )
                }).clone();

        Self {
            transaction_version: txn_version,
            event_account_address: standardize_address(&event.guid.account_address.to_string()),
            event_creation_number: event.guid.creation_number.0 as i64,
            event_sequence_number: event.sequence_number.0 as i64,
            owner_address: standardize_address(&event.guid.account_address.to_string()),
            coin_type,
            amount,
            activity_type: event_type.to_string(),
            is_gas_fee: false,
            is_transaction_success: true,
            entry_function_id_str: entry_function_id_str.clone(),
            block_height,
            transaction_timestamp,
            event_index: Some(event_index),
        }
    }

    fn get_gas_event(
        txn_info: &APITransactionInfo,
        user_transaction_request: &UserTransactionRequest,
        entry_function_id_str: &Option<String>,
        transaction_timestamp: chrono::NaiveDateTime,
    ) -> Self {
        let velor_coin_burned =
            BigDecimal::from(txn_info.gas_used.0 * user_transaction_request.gas_unit_price.0);

        Self {
            transaction_version: txn_info.version.0 as i64,
            event_account_address: standardize_address(
                &user_transaction_request.sender.to_string(),
            ),
            event_creation_number: BURN_GAS_EVENT_CREATION_NUM,
            event_sequence_number: user_transaction_request.sequence_number.0 as i64,
            owner_address: standardize_address(&user_transaction_request.sender.to_string()),
            coin_type: VelorCoinType::type_tag().to_canonical_string(),
            amount: velor_coin_burned,
            activity_type: GAS_FEE_EVENT.to_string(),
            is_gas_fee: true,
            is_transaction_success: txn_info.success,
            entry_function_id_str: entry_function_id_str.clone(),
            block_height: txn_info.block_height.unwrap().0 as i64,
            transaction_timestamp,
            event_index: Some(BURN_GAS_EVENT_INDEX),
        }
    }
}
