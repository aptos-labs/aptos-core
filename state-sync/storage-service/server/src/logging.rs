// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::Error;
use aptos_logger::Schema;
use aptos_storage_service_types::requests::StorageServiceRequest;
use serde::Serialize;

#[derive(Schema)]
pub struct LogSchema<'a> {
    name: LogEntry,
    error: Option<&'a Error>,
    message: Option<&'a str>,
    response: Option<&'a str>,
    request: Option<&'a StorageServiceRequest>,
}

impl<'a> LogSchema<'a> {
    pub fn new(name: LogEntry) -> Self {
        Self {
            name,
            error: None,
            message: None,
            response: None,
            request: None,
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
}
