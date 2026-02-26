// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_consensus_types::common::Author;
use aptos_crypto::HashValue;
use aptos_logger::Schema;
use aptos_types::block_info::Round;
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
    // optimistic proposal
    OptPropose,
    NetworkReceiveOptProposal,
    ReceiveOptProposal,
    ProcessOptProposal,
    // secret sharing events
    ReceiveSecretShare,
    BroadcastSecretShare,
    ReceiveReactiveSecretShare,
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
