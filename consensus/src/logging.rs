// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_logger::Schema;
use aptos_types::block_info::Round;
use consensus_types::common::Author;
use serde::Serialize;

#[derive(Schema)]
pub struct LogSchema {
    event: LogEvent,
    remote_peer: Option<Author>,
    epoch: Option<u64>,
    round: Option<Round>,
}

#[derive(Serialize)]
pub enum LogEvent {
    CommitViaBlock,
    CommitViaSync,
    NewEpoch,
    NewRound,
    Propose,
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
}

impl LogSchema {
    pub fn new(event: LogEvent) -> Self {
        Self {
            event,
            remote_peer: None,
            epoch: None,
            round: None,
        }
    }
}
