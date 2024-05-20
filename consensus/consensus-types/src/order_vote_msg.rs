// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{order_vote::OrderVote, quorum_cert::QuorumCert};
use anyhow::{ensure, Context};
use aptos_types::validator_verifier::ValidatorVerifier;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct OrderVoteMsg {
    order_vote: OrderVote,
    quorum_cert: QuorumCert,
}

impl Display for OrderVoteMsg {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(
            f,
            "OrderVote: [{}], QuorumCert: [{}]",
            self.order_vote, self.quorum_cert
        )
    }
}

impl OrderVoteMsg {
    pub fn new(order_vote: OrderVote, quorum_cert: QuorumCert) -> Self {
        Self {
            order_vote,
            quorum_cert,
        }
    }

    pub fn order_vote(&self) -> &OrderVote {
        &self.order_vote
    }

    pub fn quorum_cert(&self) -> &QuorumCert {
        &self.quorum_cert
    }

    pub fn epoch(&self) -> u64 {
        self.order_vote.epoch()
    }

    pub fn verify(&self, validator: &ValidatorVerifier) -> anyhow::Result<()> {
        ensure!(
            self.quorum_cert().certified_block() == self.order_vote().ledger_info().commit_info(),
            "QuorumCert and OrderVote do not match"
        );
        self.order_vote
            .verify(validator)
            .context("[OrderVoteMsg] OrderVote verification failed")?;

        // TODO: As we receive many order votes with the same quroum cert, we could cache it
        // without verifying it every time.
        self.quorum_cert
            .verify(validator)
            .context("[OrderVoteMsg QuorumCert verification failed")?;
        Ok(())
    }
}
