// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This is required because a diesel macro makes clippy sad
#![allow(clippy::extra_unused_lifetimes)]

use super::stake_utils::StakeResource;
use crate::{
    schema::{
        current_delegated_staking_pool_balances, delegated_staking_pool_balances,
        delegated_staking_pools,
    },
    util::standardize_address,
};
use aptos_api_types::{Transaction, WriteResource, WriteSetChange};
use bigdecimal::BigDecimal;
use field_count::FieldCount;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

type StakingPoolAddress = String;
pub type DelegatorPoolMap = HashMap<StakingPoolAddress, DelegatorPool>;
pub type DelegatorPoolBalanceMap = HashMap<StakingPoolAddress, CurrentDelegatorPoolBalance>;

// All pools
#[derive(Debug, Deserialize, FieldCount, Identifiable, Insertable, Serialize)]
#[diesel(primary_key(staking_pool_address))]
#[diesel(table_name = delegated_staking_pools)]
pub struct DelegatorPool {
    pub staking_pool_address: String,
    pub first_transaction_version: i64,
}

// Metadata to fill pool balances and delegator balance
#[derive(Debug, Deserialize, Serialize)]
pub struct DelegatorPoolBalanceMetadata {
    pub transaction_version: i64,
    pub staking_pool_address: String,
    pub total_coins: BigDecimal,
    pub total_shares: BigDecimal,
    pub scaling_factor: BigDecimal,
    pub active_share_table_handle: String,
}

// Pools balances
#[derive(Debug, Deserialize, FieldCount, Identifiable, Insertable, Serialize)]
#[diesel(primary_key(transaction_version, staking_pool_address))]
#[diesel(table_name = delegated_staking_pool_balances)]
pub struct DelegatorPoolBalance {
    pub transaction_version: i64,
    pub staking_pool_address: String,
    pub total_coins: BigDecimal,
    pub total_shares: BigDecimal,
}

// All pools w latest balances (really a more comprehensive version than DelegatorPool)
#[derive(Debug, Deserialize, FieldCount, Identifiable, Insertable, Serialize)]
#[diesel(primary_key(staking_pool_address))]
#[diesel(table_name = current_delegated_staking_pool_balances)]
pub struct CurrentDelegatorPoolBalance {
    pub staking_pool_address: String,
    pub total_coins: BigDecimal,
    pub total_shares: BigDecimal,
    last_transaction_version: i64,
}

impl DelegatorPool {
    pub fn from_transaction(
        transaction: &Transaction,
    ) -> anyhow::Result<(
        DelegatorPoolMap,
        Vec<DelegatorPoolBalance>,
        DelegatorPoolBalanceMap,
    )> {
        let mut delegator_pool_map = HashMap::new();
        let mut delegator_pool_balances = vec![];
        let mut delegator_pool_balances_map = HashMap::new();
        // Do a first pass to get the mapping of active_share table handles to staking pool addresses
        if let Transaction::UserTransaction(user_txn) = transaction {
            let txn_version = user_txn.info.version.0 as i64;
            for wsc in &user_txn.info.changes {
                if let WriteSetChange::WriteResource(write_resource) = wsc {
                    let maybe_write_resource =
                        Self::from_write_resource(write_resource, txn_version)?;
                    if let Some((pool, pool_balances, current_pool_balances)) = maybe_write_resource
                    {
                        let staking_pool_address = pool.staking_pool_address.clone();
                        delegator_pool_map.insert(staking_pool_address.clone(), pool);
                        delegator_pool_balances.push(pool_balances);
                        delegator_pool_balances_map
                            .insert(staking_pool_address.clone(), current_pool_balances);
                    }
                }
            }
        }
        Ok((
            delegator_pool_map,
            delegator_pool_balances,
            delegator_pool_balances_map,
        ))
    }

    pub fn get_balance_metadata(
        write_resource: &WriteResource,
        txn_version: i64,
    ) -> anyhow::Result<Option<DelegatorPoolBalanceMetadata>> {
        if let Some(StakeResource::DelegationPool(inner)) =
            StakeResource::from_write_resource(write_resource, txn_version)?
        {
            let staking_pool_address = standardize_address(&write_resource.address.to_string());
            let total_coins = inner.active_shares.total_coins;
            let total_shares =
                &inner.active_shares.total_shares / &inner.active_shares.scaling_factor;
            Ok(Some(DelegatorPoolBalanceMetadata {
                transaction_version: txn_version,
                staking_pool_address,
                total_coins,
                total_shares,
                scaling_factor: inner.active_shares.scaling_factor,
                active_share_table_handle: standardize_address(
                    &inner.active_shares.shares.inner.handle,
                ),
            }))
        } else {
            Ok(None)
        }
    }

    pub fn from_write_resource(
        write_resource: &WriteResource,
        txn_version: i64,
    ) -> anyhow::Result<Option<(Self, DelegatorPoolBalance, CurrentDelegatorPoolBalance)>> {
        if let Some(balance) = Self::get_balance_metadata(write_resource, txn_version)? {
            let staking_pool_address = balance.staking_pool_address.clone();
            let total_coins = balance.total_coins.clone();
            let total_shares = balance.total_shares.clone();
            let transaction_version = balance.transaction_version;
            Ok(Some((
                Self {
                    staking_pool_address: staking_pool_address.clone(),
                    first_transaction_version: transaction_version,
                },
                DelegatorPoolBalance {
                    transaction_version,
                    staking_pool_address: staking_pool_address.clone(),
                    total_coins: total_coins.clone(),
                    total_shares: total_shares.clone(),
                },
                CurrentDelegatorPoolBalance {
                    staking_pool_address,
                    total_coins,
                    total_shares,
                    last_transaction_version: transaction_version,
                },
            )))
        } else {
            Ok(None)
        }
    }
}
