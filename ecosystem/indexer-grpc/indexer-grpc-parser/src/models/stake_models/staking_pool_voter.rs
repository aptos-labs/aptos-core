// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This is required because a diesel macro makes clippy sad
#![allow(clippy::extra_unused_lifetimes)]

use super::stake_utils::StakeResource;
use crate::{schema::current_staking_pool_voter, utils::util::standardize_address};
use aptos_protos::transaction::v1::{write_set_change::Change, Transaction};
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
    pub fn from_transaction(transaction: &Transaction) -> anyhow::Result<StakingPoolVoterMap> {
        let mut staking_pool_voters = HashMap::new();

        let txn_version = transaction.version as i64;
        for wsc in &transaction.info.as_ref().unwrap().changes {
            if let Change::WriteResource(write_resource) = wsc.change.as_ref().unwrap() {
                if let Some(StakeResource::StakePool(inner)) =
                    StakeResource::from_write_resource(write_resource, txn_version)?
                {
                    let staking_pool_address =
                        standardize_address(&write_resource.address.to_string());
                    staking_pool_voters.insert(staking_pool_address.clone(), Self {
                        staking_pool_address,
                        voter_address: inner.get_delegated_voter(),
                        last_transaction_version: txn_version,
                        operator_address: inner.get_operator_address(),
                    });
                }
            }
        }

        Ok(staking_pool_voters)
    }
}
