// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This is required because a diesel macro makes clippy sad
#![allow(clippy::extra_unused_lifetimes)]

use super::stake_utils::StakeResource;
use crate::{schema::delegated_staking_pools, util::standardize_address};
use aptos_api_types::{Transaction, WriteResource, WriteSetChange};
use field_count::FieldCount;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

type StakingPoolAddress = String;
pub type DelegatorPoolMap = HashMap<StakingPoolAddress, DelegatorPool>;

// All pools
#[derive(Debug, Deserialize, FieldCount, Identifiable, Insertable, Serialize)]
#[diesel(primary_key(staking_pool_address))]
#[diesel(table_name = delegated_staking_pools)]
pub struct DelegatorPool {
    pub staking_pool_address: String,
    pub first_transaction_version: i64,
}

impl DelegatorPool {
    pub fn from_transaction(transaction: &Transaction) -> anyhow::Result<DelegatorPoolMap> {
        let mut delegator_pool_map = HashMap::new();
        // Do a first pass to get the mapping of active_share table handles to staking pool addresses
        if let Transaction::UserTransaction(user_txn) = transaction {
            let txn_version = user_txn.info.version.0 as i64;
            for wsc in &user_txn.info.changes {
                if let WriteSetChange::WriteResource(write_resource) = wsc {
                    let maybe_write_resource =
                        Self::from_write_resource(write_resource, txn_version)?;
                    if let Some(map) = maybe_write_resource {
                        delegator_pool_map.insert(map.staking_pool_address.clone(), map);
                    }
                }
            }
        }
        Ok(delegator_pool_map)
    }

    pub fn from_write_resource(
        write_resource: &WriteResource,
        txn_version: i64,
    ) -> anyhow::Result<Option<Self>> {
        if let Some(StakeResource::DelegationPool(_)) =
            StakeResource::from_write_resource(write_resource, txn_version)?
        {
            let staking_pool_address = standardize_address(&write_resource.address.to_string());
            return Ok(Some(Self {
                staking_pool_address,
                first_transaction_version: txn_version,
            }));
        }
        Ok(None)
    }
}
