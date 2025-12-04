// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::consensus_observer::common::error::Error;
use aptos_config::network_id::PeerNetworkId;
use aptos_logger::Schema;
use serde::Serialize;

#[derive(Schema)]
pub struct LogSchema<'a> {
    name: LogEntry,
    #[schema(debug)]
    error: Option<&'a Error>,
    event: Option<LogEvent>,
    message: Option<&'a str>,
    message_type: Option<&'a str>,
    #[schema(display)]
    peer: Option<&'a PeerNetworkId>,
    request_id: Option<u64>,
    request_type: Option<&'a str>,
}

impl LogSchema<'_> {
    pub fn new(name: LogEntry) -> Self {
        Self {
            name,
            error: None,
            event: None,
            message: None,
            message_type: None,
            peer: None,
            request_id: None,
            request_type: None,
        }
    }
}

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LogEntry {
    ConsensusObserver,
    ConsensusPublisher,
    GetDownstreamPeers,
    SendDirectSendMessage,
    SendRpcRequest,
}

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LogEvent {
    InvalidRpcResponse,
    NetworkError,
    SendDirectSendMessage,
    SendRpcRequest,
    Subscription,
    UnexpectedError,
}
