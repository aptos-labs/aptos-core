// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use aptos_consensus_types::common::Author;
use aptos_crypto::HashValue;
use aptos_logger::Schema;
use aptos_types::block_info::Round;
use serde::Serialize;

#[derive(Schema)]
pub struct LogSchema {
    event: LogEvent,
    remote_peer: Option<Author>,
    epoch: Option<u64>,
    round: Option<Round>,
    item_id: Option<HashValue>,
    rounds: Option<Vec<Round>>,
    first_round: Option<Round>,
    timestamps: Option<Vec<u64>>,
}

#[derive(Serialize)]
pub enum LogEvent {
    CommitViaBlock,
    CommitViaSync,
    NewEpoch,
    NewRound,
    Propose,
    ReceiveBatchRetrieval,
    ReceiveBlockRetrieval,
    ReceiveEpochChangeProof,
    ReceiveEpochRetrieval,
    ReceiveMessageFromDifferentEpoch,
    ReceiveNewCertificate,
    ReceiveProposal,
    ReceiveSyncInfo,
    ReceiveVote,
    RetrieveBlock,
    StateSync,
    Timeout,
    Vote,
    VoteNIL,
    SendRandToLeader,
    LeaderReceiveRand,
    LeaderBCastRand,
    ReceiveRand,
    BCastRandToAll,
}

impl LogSchema {
    pub fn new(event: LogEvent) -> Self {
        Self {
            event,
            remote_peer: None,
            epoch: None,
            round: None,
            item_id: None,
            rounds: None,
            first_round: None,
            timestamps: None,
        }
    }
}
