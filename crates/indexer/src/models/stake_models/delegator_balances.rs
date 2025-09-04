// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

// This is required because a diesel macro makes clippy sad
#![allow(clippy::extra_unused_lifetimes)]

use super::delegator_pools::{DelegatorPool, DelegatorPoolBalanceMetadata, PoolBalanceMetadata};
use crate::{
    database::PgPoolConnection,
    models::token_models::collection_datas::{QUERY_RETRIES, QUERY_RETRY_DELAY_MS},
    schema::current_delegator_balances,
    util::standardize_address,
};
use anyhow::Context;
use velor_api_types::{
    DeleteTableItem as APIDeleteTableItem, Transaction as APITransaction,
    WriteResource as APIWriteResource, WriteSetChange as APIWriteSetChange,
    WriteTableItem as APIWriteTableItem,
};
use bigdecimal::{BigDecimal, Zero};
use diesel::{prelude::*, ExpressionMethods};
use field_count::FieldCount;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub type TableHandle = String;
pub type Address = String;
pub type ShareToStakingPoolMapping = HashMap<TableHandle, DelegatorPoolBalanceMetadata>;
pub type ShareToPoolMapping = HashMap<TableHandle, PoolBalanceMetadata>;
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
    pub last_transaction_version: i64,
    pub shares: BigDecimal,
    pub parent_table_handle: String,
}

#[derive(Debug, Identifiable, Queryable)]
#[diesel(primary_key(delegator_address, pool_address, pool_type))]
#[diesel(table_name = current_delegator_balances)]
pub struct CurrentDelegatorBalanceQuery {
    pub delegator_address: String,
    pub pool_address: String,
    pub pool_type: String,
    pub table_handle: String,
    pub last_transaction_version: i64,
    pub inserted_at: chrono::NaiveDateTime,
    pub shares: BigDecimal,
    pub parent_table_handle: String,
}

impl CurrentDelegatorBalance {
    /// Getting active share balances. Only 1 active pool per staking pool tracked in a single table
    pub fn get_active_share_from_write_table_item(
        write_table_item: &APIWriteTableItem,
        txn_version: i64,
        active_pool_to_staking_pool: &ShareToStakingPoolMapping,
    ) -> anyhow::Result<Option<Self>> {
        let table_handle = standardize_address(&write_table_item.handle.to_string());
        // The mapping will tell us if the table item is an active share table
        if let Some(pool_balance) = active_pool_to_staking_pool.get(&table_handle) {
            let pool_address = pool_balance.staking_pool_address.clone();
            let delegator_address = standardize_address(&write_table_item.key.to_string());
            let data = write_table_item.data.as_ref().unwrap_or_else(|| {
                panic!(
                    "This table item should be an active share item, table_item {:?}, version {}",
                    write_table_item, txn_version
                )
            });
            let shares = data
                .value
                .as_str()
                .map(|s| s.parse::<BigDecimal>())
                .context(format!(
                    "value is not a string: {:?}, table_item {:?}, version {}",
                    data.value, write_table_item, txn_version
                ))?
                .context(format!(
                    "cannot parse string as u64: {:?}, version {}",
                    data.value, txn_version
                ))?;
            let shares = shares / &pool_balance.scaling_factor;
            Ok(Some(Self {
                delegator_address,
                pool_address,
                pool_type: "active_shares".to_string(),
                table_handle: table_handle.clone(),
                last_transaction_version: txn_version,
                shares,
                parent_table_handle: table_handle,
            }))
        } else {
            Ok(None)
        }
    }

    /// Getting inactive share balances. There could be multiple inactive pool per staking pool so we have
    /// 2 layers of mapping (table w/ all inactive pools -> staking pool, table w/ delegator inactive shares -> each inactive pool)
    pub fn get_inactive_share_from_write_table_item(
        write_table_item: &APIWriteTableItem,
        txn_version: i64,
        inactive_pool_to_staking_pool: &ShareToStakingPoolMapping,
        inactive_share_to_pool: &ShareToPoolMapping,
        conn: &mut PgPoolConnection,
    ) -> anyhow::Result<Option<Self>> {
        let table_handle = standardize_address(&write_table_item.handle.to_string());
        // The mapping will tell us if the table item belongs to an inactive pool
        if let Some(pool_balance) = inactive_share_to_pool.get(&table_handle) {
            // If it is, we need to get the inactive staking pool handle and use it to look up the staking pool
            let inactive_pool_handle = pool_balance.parent_table_handle.clone();

            let pool_address = match inactive_pool_to_staking_pool
                .get(&inactive_pool_handle)
                .map(|metadata| metadata.staking_pool_address.clone())
            {
                Some(pool_address) => pool_address,
                None => {
                    Self::get_staking_pool_from_inactive_share_handle(conn, &inactive_pool_handle)
                        .context(format!("Failed to get staking pool address from inactive share handle {}, txn version {}",
                        inactive_pool_handle, txn_version
                    ))?
                },
            };
            let delegator_address = standardize_address(&write_table_item.key.to_string());
            let data = write_table_item.data.as_ref().unwrap_or_else(|| {
                panic!(
                    "This table item should be an active share item, table_item {:?}, version {}",
                    write_table_item, txn_version
                )
            });
            let shares = data
                .value
                .as_str()
                .map(|s| s.parse::<BigDecimal>())
                .context(format!(
                    "value is not a string: {:?}, table_item {:?}, version {}",
                    data.value, write_table_item, txn_version
                ))?
                .context(format!(
                    "cannot parse string as u64: {:?}, version {}",
                    data.value, txn_version
                ))?;
            let shares = shares / &pool_balance.scaling_factor;
            Ok(Some(Self {
                delegator_address,
                pool_address,
                pool_type: "inactive_shares".to_string(),
                table_handle: table_handle.clone(),
                last_transaction_version: txn_version,
                shares,
                parent_table_handle: inactive_pool_handle,
            }))
        } else {
            Ok(None)
        }
    }

    // Setting amount to 0 if table item is deleted
    pub fn get_active_share_from_delete_table_item(
        delete_table_item: &APIDeleteTableItem,
        txn_version: i64,
        active_pool_to_staking_pool: &ShareToStakingPoolMapping,
    ) -> anyhow::Result<Option<Self>> {
        let table_handle = standardize_address(&delete_table_item.handle.to_string());
        // The mapping will tell us if the table item is an active share table
        if let Some(pool_balance) = active_pool_to_staking_pool.get(&table_handle) {
            let delegator_address = standardize_address(&delete_table_item.key.to_string());

            return Ok(Some(Self {
                delegator_address,
                pool_address: pool_balance.staking_pool_address.clone(),
                pool_type: "active_shares".to_string(),
                table_handle: table_handle.clone(),
                last_transaction_version: txn_version,
                shares: BigDecimal::zero(),
                parent_table_handle: table_handle,
            }));
        }
        Ok(None)
    }

    // Setting amount to 0 if table item is deleted
    pub fn get_inactive_share_from_delete_table_item(
        delete_table_item: &APIDeleteTableItem,
        txn_version: i64,
        inactive_pool_to_staking_pool: &ShareToStakingPoolMapping,
        inactive_share_to_pool: &ShareToPoolMapping,
        conn: &mut PgPoolConnection,
    ) -> anyhow::Result<Option<Self>> {
        let table_handle = standardize_address(&delete_table_item.handle.to_string());
        // The mapping will tell us if the table item belongs to an inactive pool
        if let Some(pool_balance) = inactive_share_to_pool.get(&table_handle) {
            // If it is, we need to get the inactive staking pool handle and use it to look up the staking pool
            let inactive_pool_handle = pool_balance.parent_table_handle.clone();

            let pool_address = match inactive_pool_to_staking_pool
                .get(&inactive_pool_handle)
                .map(|metadata| metadata.staking_pool_address.clone())
            {
                Some(pool_address) => pool_address,
                None => {
                    match Self::get_staking_pool_from_inactive_share_handle(
                        conn,
                        &inactive_pool_handle,
                    ) {
                        Ok(pool) => pool,
                        Err(_) => {
                            velor_logger::error!(
                                transaction_version = txn_version,
                                lookup_key = &inactive_pool_handle,
                                "Failed to get staking pool address from inactive share handle. You probably should backfill db.",
                            );
                            return Ok(None);
                        },
                    }
                },
            };
            let delegator_address = standardize_address(&delete_table_item.key.to_string());

            return Ok(Some(Self {
                delegator_address,
                pool_address,
                pool_type: "inactive_shares".to_string(),
                table_handle: table_handle.clone(),
                last_transaction_version: txn_version,
                shares: BigDecimal::zero(),
                parent_table_handle: table_handle,
            }));
        }
        Ok(None)
    }

    /// Key is the inactive share table handle obtained from 0x1::delegation_pool::DelegationPool
    /// Value is the same metadata although it's not really used
    pub fn get_active_pool_to_staking_pool_mapping(
        write_resource: &APIWriteResource,
        txn_version: i64,
    ) -> anyhow::Result<Option<ShareToStakingPoolMapping>> {
        if let Some(balance) = DelegatorPool::get_delegated_pool_metadata_from_write_resource(
            write_resource,
            txn_version,
        )? {
            Ok(Some(HashMap::from([(
                balance.active_share_table_handle.clone(),
                balance,
            )])))
        } else {
            Ok(None)
        }
    }

    /// Key is the inactive share table handle obtained from 0x1::delegation_pool::DelegationPool
    /// Value is the same metadata although it's not really used
    pub fn get_inactive_pool_to_staking_pool_mapping(
        write_resource: &APIWriteResource,
        txn_version: i64,
    ) -> anyhow::Result<Option<ShareToStakingPoolMapping>> {
        if let Some(balance) = DelegatorPool::get_delegated_pool_metadata_from_write_resource(
            write_resource,
            txn_version,
        )? {
            Ok(Some(HashMap::from([(
                balance.inactive_share_table_handle.clone(),
                balance,
            )])))
        } else {
            Ok(None)
        }
    }

    /// Key is the inactive share table handle obtained from 0x1::pool_u64_unbound::Pool
    /// Value is the 0x1::pool_u64_unbound::Pool metadata that will be used to populate a user's inactive balance
    pub fn get_inactive_share_to_pool_mapping(
        write_table_item: &APIWriteTableItem,
        txn_version: i64,
    ) -> anyhow::Result<Option<ShareToPoolMapping>> {
        if let Some(balance) = DelegatorPool::get_inactive_pool_metadata_from_write_table_item(
            write_table_item,
            txn_version,
        )? {
            Ok(Some(HashMap::from([(
                balance.shares_table_handle.clone(),
                balance,
            )])))
        } else {
            Ok(None)
        }
    }

    pub fn get_staking_pool_from_inactive_share_handle(
        conn: &mut PgPoolConnection,
        table_handle: &str,
    ) -> anyhow::Result<String> {
        let mut retried = 0;
        while retried < QUERY_RETRIES {
            retried += 1;
            match CurrentDelegatorBalanceQuery::get_by_inactive_share_handle(conn, table_handle) {
                Ok(current_delegator_balance) => return Ok(current_delegator_balance.pool_address),
                Err(_) => {
                    std::thread::sleep(std::time::Duration::from_millis(QUERY_RETRY_DELAY_MS));
                },
            }
        }
        Err(anyhow::anyhow!(
            "Failed to get staking pool address from inactive share handle"
        ))
    }

    pub fn from_transaction(
        transaction: &APITransaction,
        conn: &mut PgPoolConnection,
    ) -> anyhow::Result<CurrentDelegatorBalanceMap> {
        let mut active_pool_to_staking_pool: ShareToStakingPoolMapping = HashMap::new();
        let mut inactive_pool_to_staking_pool: ShareToStakingPoolMapping = HashMap::new();
        let mut inactive_share_to_pool: ShareToPoolMapping = HashMap::new();
        let mut current_delegator_balances: CurrentDelegatorBalanceMap = HashMap::new();
        // Do a first pass to get the mapping of active_share table handles to staking pool resource
        if let APITransaction::UserTransaction(user_txn) = transaction {
            let txn_version = user_txn.info.version.0 as i64;
            for wsc in &user_txn.info.changes {
                if let APIWriteSetChange::WriteResource(write_resource) = wsc {
                    if let Some(map) =
                        Self::get_active_pool_to_staking_pool_mapping(write_resource, txn_version)
                            .unwrap()
                    {
                        active_pool_to_staking_pool.extend(map);
                    }
                    if let Some(map) =
                        Self::get_inactive_pool_to_staking_pool_mapping(write_resource, txn_version)
                            .unwrap()
                    {
                        inactive_pool_to_staking_pool.extend(map);
                    }
                }
                if let APIWriteSetChange::WriteTableItem(table_item) = wsc {
                    if let Some(map) =
                        Self::get_inactive_share_to_pool_mapping(table_item, txn_version).unwrap()
                    {
                        inactive_share_to_pool.extend(map);
                    }
                }
            }
            // Now make a pass through table items to get the actual delegator balances
            for wsc in &user_txn.info.changes {
                let txn_version = user_txn.info.version.0 as i64;
                let maybe_delegator_balance = match wsc {
                    APIWriteSetChange::DeleteTableItem(table_item) => {
                        if let Some(balance) = Self::get_active_share_from_delete_table_item(
                            table_item,
                            txn_version,
                            &active_pool_to_staking_pool,
                        )
                        .unwrap()
                        {
                            Some(balance)
                        } else {
                            Self::get_inactive_share_from_delete_table_item(
                                table_item,
                                txn_version,
                                &inactive_pool_to_staking_pool,
                                &inactive_share_to_pool,
                                conn,
                            )
                            .unwrap()
                        }
                    },
                    APIWriteSetChange::WriteTableItem(table_item) => {
                        if let Some(balance) = Self::get_active_share_from_write_table_item(
                            table_item,
                            txn_version,
                            &active_pool_to_staking_pool,
                        )
                        .unwrap()
                        {
                            Some(balance)
                        } else {
                            Self::get_inactive_share_from_write_table_item(
                                table_item,
                                txn_version,
                                &inactive_pool_to_staking_pool,
                                &inactive_share_to_pool,
                                conn,
                            )
                            .unwrap()
                        }
                    },
                    _ => None,
                };
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

impl CurrentDelegatorBalanceQuery {
    pub fn get_by_inactive_share_handle(
        conn: &mut PgPoolConnection,
        table_handle: &str,
    ) -> diesel::QueryResult<Self> {
        current_delegator_balances::table
            .filter(current_delegator_balances::parent_table_handle.eq(table_handle))
            .first::<Self>(conn)
    }
}
