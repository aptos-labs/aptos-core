// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Vote messages without attached certificates.
//!
//! Validators broadcast votes to all peers, so each receiver forms every QC itself. The
//! `SyncInfo` in a `VoteMsg` and the QC in an `OrderVoteMsg` repeat data that the receiver
//! already has in the normal case. When certificates are large (for example, ML-DSA
//! certificates without aggregation), these copies dominate network traffic.
//!
//! A receiver that is behind gets the missing certificates from proposals and timeout
//! messages, which still carry a full `SyncInfo`, or asks a peer with a `SyncInfoRequest`.

use crate::{
    common::{Author, Round},
    order_vote::OrderVote,
    vote::Vote,
};
use anyhow::ensure;
use aptos_crypto::HashValue;
use aptos_types::validator_verifier::ValidatorVerifier;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

/// A non-timeout vote without the sender's `SyncInfo`.
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct SlimVoteMsg {
    vote: Vote,
}

impl Display for SlimVoteMsg {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "SlimVoteMsg: [{}]", self.vote)
    }
}

impl SlimVoteMsg {
    pub fn new(vote: Vote) -> Self {
        Self { vote }
    }

    pub fn vote(&self) -> &Vote {
        &self.vote
    }

    pub fn epoch(&self) -> u64 {
        self.vote.epoch()
    }

    pub fn proposed_block_id(&self) -> HashValue {
        self.vote.vote_data().proposed().id()
    }

    /// Timeout votes must carry a `SyncInfo`, so a slim vote cannot be a timeout vote.
    pub fn verify(&self, sender: Author, validator: &ValidatorVerifier) -> anyhow::Result<()> {
        ensure!(
            self.vote.author() == sender,
            "Vote author {:?} is different from the sender {:?}",
            self.vote.author(),
            sender,
        );
        ensure!(
            !self.vote.is_timeout(),
            "A slim vote cannot carry a timeout"
        );
        self.vote.verify(validator)
    }
}

/// Asks a peer for its `SyncInfo`. A node sends this when slim votes show that it is
/// behind, because slim votes do not carry the certificates it needs to catch up.
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct SyncInfoRequest {
    epoch: u64,
    /// The current round of the requester. A peer replies only if it is ahead.
    round: Round,
}

impl Display for SyncInfoRequest {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(
            f,
            "SyncInfoRequest: [epoch: {}, round: {}]",
            self.epoch, self.round
        )
    }
}

impl SyncInfoRequest {
    pub fn new(epoch: u64, round: Round) -> Self {
        Self { epoch, round }
    }

    pub fn epoch(&self) -> u64 {
        self.epoch
    }

    pub fn round(&self) -> Round {
        self.round
    }
}

/// An order vote without the quorum certificate of the ordered block.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SlimOrderVoteMsg {
    order_vote: OrderVote,
}

impl Display for SlimOrderVoteMsg {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "SlimOrderVoteMsg: [{}]", self.order_vote)
    }
}

impl SlimOrderVoteMsg {
    pub fn new(order_vote: OrderVote) -> Self {
        Self { order_vote }
    }

    pub fn order_vote(&self) -> &OrderVote {
        &self.order_vote
    }

    pub fn epoch(&self) -> u64 {
        self.order_vote.epoch()
    }

    /// The receiver checks the order vote against its local QC when it processes the vote.
    pub fn verify(&self, sender: Author, validator: &ValidatorVerifier) -> anyhow::Result<()> {
        ensure!(
            self.order_vote.author() == sender,
            "Order vote author {:?} is different from the sender {:?}",
            self.order_vote.author(),
            sender
        );
        self.order_vote.verify(validator)
    }
}
