// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::error::Error;
use velor_config::network_id::PeerNetworkId;
use velor_logger::Schema;
use velor_storage_service_types::requests::StorageServiceRequest;
use serde::Serialize;

#[derive(Schema)]
pub struct LogSchema<'a> {
    name: LogEntry,
    #[schema(debug)]
    error: Option<&'a Error>,
    event: Option<LogEvent>,
    message: Option<&'a str>,
    #[schema(display)]
    peer: Option<&'a PeerNetworkId>,
    #[schema(debug)]
    request_data: Option<&'a StorageServiceRequest>,
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
            peer: None,
            request_data: None,
            request_id: None,
            request_type: None,
        }
    }
}

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LogEntry {
    DataSummaryPoller,
    LatencyMonitor,
    PeerStates,
    StorageServiceRequest,
    StorageServiceResponse,
    StorageSummaryResponse,
}

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LogEvent {
    AggregateSummary,
    CaughtUpToLatest,
    NoPeersToPoll,
    PeerIgnored,
    PeerNoLongerIgnored,
    PeerPollingError,
    PeerRequestResponseCounts,
    PeerSelectionError,
    PriorityAndRegularPeers,
    PriorityPeerCategories,
    ResponseError,
    ResponseSuccess,
    SendRequest,
    StorageReadFailed,
    UnexpectedError,
    WaitingForCatchup,
}
