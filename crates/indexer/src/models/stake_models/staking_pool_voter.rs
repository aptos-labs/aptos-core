// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

// This is required because a diesel macro makes clippy sad
#![allow(clippy::extra_unused_lifetimes)]

use super::stake_utils::StakeResource;
use crate::{schema::current_staking_pool_voter, util::standardize_address};
use velor_api_types::{Transaction as APITransaction, WriteSetChange as APIWriteSetChange};
use field_count::FieldCount;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

type StakingPoolAddress = String;
pub type StakingPoolVoterMap = HashMap<StakingPoolAddress, CurrentStakingPoolVoter>;

#[derive(Debug, Deserialize, FieldCount, Identifiable, Insertable, Serialize)]
#[diesel(primary_key(staking_pool_address))]
#[diesel(table_name = current_staking_pool_voter)]
pub struct CurrentStakingPoolVoter {
    pub staking_pool_address: String,
    pub voter_address: String,
    pub last_transaction_version: i64,
    pub operator_address: String,
}

impl CurrentStakingPoolVoter {
    pub fn from_transaction(transaction: &APITransaction) -> anyhow::Result<StakingPoolVoterMap> {
        let mut staking_pool_voters = HashMap::new();
        let empty_change = vec![];
        let (txn_version, changes) = match transaction {
            APITransaction::UserTransaction(txn) => (txn.info.version.0 as i64, &txn.info.changes),
            APITransaction::GenesisTransaction(txn) => {
                (txn.info.version.0 as i64, &txn.info.changes)
            },
            APITransaction::BlockMetadataTransaction(txn) => {
                (txn.info.version.0 as i64, &txn.info.changes)
            },
            _ => (0, &empty_change),
        };
        for wsc in changes {
            if let APIWriteSetChange::WriteResource(write_resource) = wsc {
                if let Some(StakeResource::StakePool(inner)) =
                    StakeResource::from_write_resource(write_resource, txn_version)?
                {
                    let staking_pool_address =
                        standardize_address(&write_resource.address.to_string());
                    let operator_address = standardize_address(&inner.operator_address);
                    let voter_address = standardize_address(&inner.delegated_voter);
                    staking_pool_voters.insert(staking_pool_address.clone(), Self {
                        staking_pool_address,
                        voter_address,
                        last_transaction_version: txn_version,
                        operator_address,
                    });
                }
            }
        }

        Ok(staking_pool_voters)
    }
}
