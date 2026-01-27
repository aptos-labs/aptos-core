// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_consensus_types::common::{Author, Round};
use aptos_logger::Schema;
use serde::Serialize;

#[derive(Schema)]
pub struct LogSchema {
    event: LogEvent,
    remote_peer: Option<Author>,
    round: Option<Round>,
}

#[derive(Serialize)]
pub enum LogEvent {
    EpochStart,
    ModeTransition,
    BroadcastNode,
    ReceiveNode,
    Vote,
    ReceiveVote,
    BroadcastCertifiedNode,
    ReceiveCertifiedNode,
    ReceiveAck,
    OrderedAnchor,
    NewRound,
    FetchNodes,
    ReceiveFetchNodes,
    ActiveMode,
    SyncMode,
    SyncOutcome,
    Shutdown,
}

impl LogSchema {
    pub fn new(event: LogEvent) -> Self {
        Self {
            event,
            remote_peer: None,
            round: None,
        }
    }
}
