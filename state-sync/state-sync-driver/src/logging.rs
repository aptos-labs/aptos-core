// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{error::Error, notification_handlers::ErrorNotification};
use aptos_logger::Schema;
use serde::Serialize;

#[derive(Schema)]
pub struct LogSchema<'a> {
    name: LogEntry,
    error: Option<&'a Error>,
    error_notification: Option<ErrorNotification>,
    message: Option<&'a str>,
}

impl LogSchema<'_> {
    pub fn new(name: LogEntry) -> Self {
        Self {
            name,
            error: None,
            error_notification: None,
            message: None,
        }
    }
}

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LogEntry {
    AutoBootstrapping,
    Bootstrapper,
    ClientNotification,
    ConsensusNotification,
    Driver,
    NotificationHandler,
    StorageSynchronizer,
    SynchronizerNotification,
}
