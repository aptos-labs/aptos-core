// Copyright © Aptos Foundation

// This is required because a diesel macro makes clippy sad
#![allow(clippy::extra_unused_lifetimes)]

use crate::{
    models::{default_models::move_tables::TableItem, stake_models::stake_utils::StakeResource},
    schema::current_delegator_balances,
    utils::util::standardize_address,
};
use anyhow::Context;
use aptos_protos::transaction::testing1::v1::{
    write_set_change::Change, DeleteTableItem, Transaction, WriteResource, WriteTableItem,
};
use bigdecimal::{BigDecimal, Zero};
use field_count::FieldCount;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub type TableHandle = String;
pub type Address = String;
pub type ActiveShareMapping = HashMap<TableHandle, Address>;
pub type CurrentDelegatorBalancePK = (Address, Address, String);
pub type CurrentDelegatorBalanceMap = HashMap<CurrentDelegatorBalancePK, CurrentDelegatorBalance>;

#[derive(Debug, Deserialize, FieldCount, Identifiable, Insertable, Serialize)]
#[diesel(primary_key(delegator_address, pool_address, pool_type))]
#[diesel(table_name = current_delegator_balances)]
pub struct CurrentDelegatorBalance {
    pub delegator_address: String,
    pub pool_address: String,
    pub pool_type: String,
    pub table_handle: String,
    pub amount: BigDecimal,
    pub last_transaction_version: i64,
}

impl CurrentDelegatorBalance {
    /// We're only indexing active_shares for now because that's all the UI needs and indexing
    /// the inactive_shares or pending_withdrawal_shares would be more complicated.
    pub fn from_write_table_item(
        write_table_item: &WriteTableItem,
        txn_version: i64,
        active_share_mapping: &ActiveShareMapping,
    ) -> anyhow::Result<Option<Self>> {
        let table_handle = standardize_address(&write_table_item.handle.to_string());
        // The mapping will tell us if the table item is an active share table
        if let Some(pool_address) = active_share_mapping.get(&table_handle) {
            let pool_address = standardize_address(pool_address);
            let delegator_address = standardize_address(&write_table_item.key.to_string());

            // Convert to TableItem model. Some fields are just placeholders
            let (table_item_model, _) =
                TableItem::from_write_table_item(write_table_item, 0, txn_version, 0);

            let amount = table_item_model
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
                delegator_address,
                pool_address,
                pool_type: "active_shares".to_string(),
                table_handle,
                amount,
                last_transaction_version: txn_version,
            }));
        }
        Ok(None)
    }

    // Setting amount to 0 if table item is deleted
    pub fn from_delete_table_item(
        delete_table_item: &DeleteTableItem,
        txn_version: i64,
        active_share_mapping: &ActiveShareMapping,
    ) -> anyhow::Result<Option<Self>> {
        let table_handle = standardize_address(&delete_table_item.handle.to_string());
        // The mapping will tell us if the table item is an active share table
        if let Some(pool_address) = active_share_mapping.get(&table_handle) {
            let delegator_address = standardize_address(&delete_table_item.key.to_string());

            return Ok(Some(Self {
                delegator_address,
                pool_address: pool_address.clone(),
                pool_type: "active_shares".to_string(),
                table_handle,
                amount: BigDecimal::zero(),
                last_transaction_version: txn_version,
            }));
        }
        Ok(None)
    }

    pub fn get_active_share_map(
        write_resource: &WriteResource,
        txn_version: i64,
    ) -> anyhow::Result<Option<ActiveShareMapping>> {
        if let Some(StakeResource::DelegationPool(inner)) =
            StakeResource::from_write_resource(write_resource, txn_version)?
        {
            let staking_pool_address = standardize_address(&write_resource.address.to_string());
            let table_handle = standardize_address(&inner.active_shares.shares.inner.handle);
            return Ok(Some(HashMap::from([(table_handle, staking_pool_address)])));
        }
        Ok(None)
    }

    pub fn from_transaction(
        transaction: &Transaction,
    ) -> anyhow::Result<CurrentDelegatorBalanceMap> {
        let mut active_share_mapping: ActiveShareMapping = HashMap::new();
        let mut current_delegator_balances: CurrentDelegatorBalanceMap = HashMap::new();

        // Do a first pass to get the mapping of active_share table handles to staking pool addresses
        let txn_version = transaction.version as i64;
        for wsc in &transaction.info.as_ref().unwrap().changes {
            if let Change::WriteResource(write_resource) = wsc.change.as_ref().unwrap() {
                let maybe_map = Self::get_active_share_map(write_resource, txn_version).unwrap();
                if let Some(map) = maybe_map {
                    active_share_mapping.extend(map);
                }
            }
        }
        // Now make a pass through table items to get the actual delegator balances
        for wsc in &transaction.info.as_ref().unwrap().changes {
            if let Change::WriteTableItem(table_item) = wsc.change.as_ref().unwrap() {
                let maybe_delegator_balance =
                    Self::from_write_table_item(table_item, txn_version, &active_share_mapping)
                        .unwrap();
                if let Some(delegator_balance) = maybe_delegator_balance {
                    current_delegator_balances.insert(
                        (
                            delegator_balance.delegator_address.clone(),
                            delegator_balance.pool_address.clone(),
                            delegator_balance.pool_type.clone(),
                        ),
                        delegator_balance,
                    );
                }
            }
        }
        Ok(current_delegator_balances)
    }
}
