// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{common::Author, order_vote::OrderVote, quorum_cert::QuorumCert};
use anyhow::{ensure, Context};
use velor_types::validator_verifier::ValidatorVerifier;
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

    /// This function verifies the order_vote component in the order_vote_msg.
    /// The quorum cert is verified in the round manager when the quorum certificate is used.
    pub fn verify_order_vote(
        &self,
        sender: Author,
        validator: &ValidatorVerifier,
    ) -> anyhow::Result<()> {
        ensure!(
            self.order_vote.author() == sender,
            "Order vote author {:?} is different from the sender {:?}",
            self.order_vote.author(),
            sender
        );
        ensure!(
            self.quorum_cert().certified_block() == self.order_vote().ledger_info().commit_info(),
            "QuorumCert and OrderVote do not match"
        );
        self.order_vote
            .verify(validator)
            .context("[OrderVoteMsg] OrderVote verification failed")?;
        Ok(())
    }
}
