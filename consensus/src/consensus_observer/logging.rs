// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::consensus_observer::error::Error;
use aptos_config::network_id::PeerNetworkId;
use aptos_logger::Schema;
use serde::Serialize;

#[derive(Schema)]
pub struct LogSchema<'a> {
    name: LogEntry,
    #[schema(debug)]
    error: Option<&'a Error>,
    event: Option<LogEvent>,
    log: Option<&'a str>,
    message_content: Option<&'a str>,
    message_type: Option<&'a str>,
    #[schema(display)]
    peer: Option<&'a PeerNetworkId>,
    request_id: Option<u64>,
    request_type: Option<&'a str>,
}

impl<'a> LogSchema<'a> {
    pub fn new(name: LogEntry) -> Self {
        Self {
            name,
            error: None,
            event: None,
            log: None,
            message_content: None,
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
    UnexpectedError,
}
