// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use velor_consensus_types::common::Author;
use velor_crypto::HashValue;
use velor_logger::Schema;
use velor_types::block_info::Round;
use serde::Serialize;

#[derive(Schema)]
pub struct LogSchema {
    event: LogEvent,
    author: Option<Author>,
    remote_peer: Option<Author>,
    epoch: Option<u64>,
    round: Option<Round>,
    id: Option<HashValue>,
}

#[derive(Serialize)]
pub enum LogEvent {
    BroadcastOrderVote,
    CommitViaBlock,
    CommitViaSync,
    IncrementalProofExpired,
    NetworkReceiveProposal,
    NewEpoch,
    NewRound,
    ProofOfStoreInit,
    ProofOfStoreReady,
    ProofOfStoreCommit,
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
    ReceiveRoundTimeout,
    ReceiveOrderVote,
    RetrieveBlock,
    StateSync,
    Timeout,
    Vote,
    VoteNIL,
    // log events related to randomness generation
    BroadcastRandShare,
    ReceiveProactiveRandShare,
    ReceiveReactiveRandShare,
    BroadcastAugData,
    ReceiveAugData,
    BroadcastCertifiedAugData,
    ReceiveCertifiedAugData,
    // randomness fast path
    BroadcastRandShareFastPath,
    ReceiveRandShareFastPath,
    // optimistic proposal
    OptPropose,
    NetworkReceiveOptProposal,
    ReceiveOptProposal,
    ProcessOptProposal,
}

impl LogSchema {
    pub fn new(event: LogEvent) -> Self {
        Self {
            event,
            author: None,
            remote_peer: None,
            epoch: None,
            round: None,
            id: None,
        }
    }
}
