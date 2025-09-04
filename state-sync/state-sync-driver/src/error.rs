// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use velor_types::transaction::Version;
use futures::channel::{mpsc::SendError, oneshot::Canceled};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Clone, Debug, Deserialize, Error, PartialEq, Eq, Serialize)]
pub enum Error {
    #[error("State sync has already finished bootstrapping! Error: {0}")]
    AlreadyBootstrapped(String),
    #[error("Advertised data error: {0}")]
    AdvertisedDataError(String),
    #[error("State sync has not yet finished bootstrapping! Error: {0}")]
    BootstrapNotComplete(String),
    #[error("Failed to send callback: {0}")]
    CallbackSendFailed(String),
    #[error("Timed-out waiting for a data stream too many times. Times: {0}")]
    CriticalDataStreamTimeout(String),
    #[error("Timed-out waiting for a notification from the data stream. Timeout: {0}")]
    DataStreamNotificationTimeout(String),
    #[error("Error encountered in the event subscription service: {0}")]
    EventNotificationError(String),
    #[error("A consensus notification was sent to a full node: {0}")]
    FullNodeConsensusNotification(String),
    #[error("An integer overflow has occurred: {0}")]
    IntegerOverflow(String),
    #[error("An invalid payload was received: {0}")]
    InvalidPayload(String),
    #[error("Failed to notify mempool of the new commit: {0}")]
    NotifyMempoolError(String),
    #[error("Failed to notify the storage service of the new commit: {0}")]
    NotifyStorageServiceError(String),
    #[error("Received an old sync request for version {0}, but our committed version is: {1}")]
    OldSyncRequest(Version, Version),
    #[error("Received oneshot::canceled. The sender of a channel was dropped: {0}")]
    SenderDroppedError(String),
    #[error("Unexpected storage error: {0}")]
    StorageError(String),
    #[error("Synced beyond the target version. Committed version: {0}, target version: {1}")]
    SyncedBeyondTarget(Version, Version),
    #[error("Verification error: {0}")]
    VerificationError(String),
    #[error("Unexpected error: {0}")]
    UnexpectedError(String),
    #[error("Failed to verify waypoint satisfiability: {0}")]
    UnsatisfiableWaypoint(String),
}

impl Error {
    /// Returns a summary label for the error
    pub fn get_label(&self) -> &'static str {
        match self {
            Error::AlreadyBootstrapped(_) => "already_boostrapped",
            Error::AdvertisedDataError(_) => "advertised_data_error",
            Error::BootstrapNotComplete(_) => "bootstrap_not_complete",
            Error::CallbackSendFailed(_) => "callback_send_failed",
            Error::CriticalDataStreamTimeout(_) => "critical_data_stream_timeout",
            Error::DataStreamNotificationTimeout(_) => "data_stream_notification_timeout",
            Error::EventNotificationError(_) => "event_notification_error",
            Error::FullNodeConsensusNotification(_) => "full_node_consensus_notification",
            Error::IntegerOverflow(_) => "integer_overflow",
            Error::InvalidPayload(_) => "invalid_payload",
            Error::NotifyMempoolError(_) => "notify_mempool_error",
            Error::NotifyStorageServiceError(_) => "notify_storage_service_error",
            Error::OldSyncRequest(_, _) => "old_sync_request",
            Error::SenderDroppedError(_) => "sender_dropped_error",
            Error::StorageError(_) => "storage_error",
            Error::SyncedBeyondTarget(_, _) => "synced_beyond_target",
            Error::VerificationError(_) => "verification_error",
            Error::UnexpectedError(_) => "unexpected_error",
            Error::UnsatisfiableWaypoint(_) => "unsatisfiable_waypoint",
        }
    }
}

impl From<Canceled> for Error {
    fn from(canceled: Canceled) -> Self {
        Error::SenderDroppedError(canceled.to_string())
    }
}

impl From<velor_data_streaming_service::error::Error> for Error {
    fn from(error: velor_data_streaming_service::error::Error) -> Self {
        Error::UnexpectedError(error.to_string())
    }
}

impl From<velor_event_notifications::Error> for Error {
    fn from(error: velor_event_notifications::Error) -> Self {
        Error::EventNotificationError(error.to_string())
    }
}

impl From<SendError> for Error {
    fn from(error: SendError) -> Self {
        Error::UnexpectedError(error.to_string())
    }
}
