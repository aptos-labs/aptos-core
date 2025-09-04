// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

// This is required because a diesel macro makes clippy sad
#![allow(clippy::extra_unused_lifetimes)]

use super::stake_utils::StakeEvent;
use crate::{
    schema::proposal_votes,
    util::{parse_timestamp, standardize_address},
};
use velor_api_types::Transaction as APITransaction;
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
    pub fn from_transaction(transaction: &APITransaction) -> anyhow::Result<Vec<Self>> {
        let mut proposal_votes = vec![];
        if let APITransaction::UserTransaction(user_txn) = transaction {
            for event in &user_txn.events {
                let txn_version = user_txn.info.version.0 as i64;
                let event_type = event.typ.to_string();
                if let Some(StakeEvent::GovernanceVoteEvent(ev)) =
                    StakeEvent::from_event(event_type.as_str(), &event.data, txn_version)?
                {
                    proposal_votes.push(Self {
                        transaction_version: txn_version,
                        proposal_id: ev.proposal_id as i64,
                        voter_address: standardize_address(&ev.voter),
                        staking_pool_address: standardize_address(&ev.stake_pool),
                        num_votes: ev.num_votes.clone(),
                        should_pass: ev.should_pass,
                        transaction_timestamp: parse_timestamp(user_txn.timestamp.0, txn_version),
                    });
                }
            }
        }
        Ok(proposal_votes)
    }
}
