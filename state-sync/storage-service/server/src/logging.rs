// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::Error;
use aptos_config::network_id::PeerNetworkId;
use aptos_logger::Schema;
use aptos_storage_service_types::requests::StorageServiceRequest;
use serde::Serialize;

#[derive(Schema)]
pub struct LogSchema<'a> {
    name: LogEntry,
    error: Option<&'a Error>,
    message: Option<&'a str>,
    peer_network_id: Option<&'a PeerNetworkId>,
    response: Option<&'a str>,
    request: Option<&'a StorageServiceRequest>,
    subscription_related: Option<bool>,
}

impl<'a> LogSchema<'a> {
    pub fn new(name: LogEntry) -> Self {
        Self {
            name,
            error: None,
            message: None,
            peer_network_id: None,
            response: None,
            request: None,
            subscription_related: None,
        }
    }
}

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LogEntry {
    ReceivedStorageRequest,
    SentStorageResponse,
    StorageServiceError,
    StorageSummaryRefresh,
    SubscriptionRefresh,
    SubscriptionResponse,
    SubscriptionRequest,
}
