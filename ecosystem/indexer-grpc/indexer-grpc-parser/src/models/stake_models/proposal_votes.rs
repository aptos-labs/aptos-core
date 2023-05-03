// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This is required because a diesel macro makes clippy sad
#![allow(clippy::extra_unused_lifetimes)]

use super::stake_utils::StakeEvent;
use crate::{
    schema::proposal_votes,
    utils::util::{parse_timestamp, standardize_address},
};
use aptos_protos::transaction::testing1::v1::{transaction::TxnData, Transaction};
use bigdecimal::BigDecimal;
use field_count::FieldCount;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, FieldCount, Identifiable, Insertable, Serialize)]
#[diesel(primary_key(transaction_version, proposal_id, voter_address))]
#[diesel(table_name = proposal_votes)]
pub struct ProposalVote {
    pub transaction_version: i64,
    pub proposal_id: i64,
    pub voter_address: String,
    pub staking_pool_address: String,
    pub num_votes: BigDecimal,
    pub should_pass: bool,
    pub transaction_timestamp: chrono::NaiveDateTime,
}

impl ProposalVote {
    pub fn from_transaction(transaction: &Transaction) -> anyhow::Result<Vec<Self>> {
        let mut proposal_votes = vec![];
        let txn_data = transaction
            .txn_data
            .as_ref()
            .expect("Txn Data doesn't exit!");
        let txn_version = transaction.version as i64;

        if let TxnData::User(user_txn) = txn_data {
            for event in &user_txn.events {
                if let Some(StakeEvent::GovernanceVoteEvent(ev)) =
                    StakeEvent::from_event(event.type_str.as_str(), &event.data, txn_version)?
                {
                    proposal_votes.push(Self {
                        transaction_version: txn_version,
                        proposal_id: ev.proposal_id as i64,
                        voter_address: standardize_address(&ev.voter),
                        staking_pool_address: standardize_address(&ev.stake_pool),
                        num_votes: ev.num_votes.clone(),
                        should_pass: ev.should_pass,
                        transaction_timestamp: parse_timestamp(
                            transaction.timestamp.as_ref().unwrap(),
                            txn_version,
                        ),
                    });
                }
            }
        }
        Ok(proposal_votes)
    }
}
